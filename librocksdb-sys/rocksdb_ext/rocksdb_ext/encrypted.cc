// Copyright 2020 Hoang Phan
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#ifndef ROCKSDB_LITE
 #ifdef OPENSSL

#include <algorithm>
#include <limits>
#include <stdlib.h> /* srand, rand */
#include <time.h>   /* time */

#include <openssl/opensslv.h>

#include "port/port.h"
#include "rocksdb_ext/encrypted.h"

template <typename T>
inline ::std::string ToString(T value)
{
#if !(defined OS_ANDROID) && !(defined CYGWIN) && !(defined OS_FREEBSD)
    return std::to_string(value);
#else
    // Andorid or cygwin doesn't support all of C++11, std::to_string() being
    // one of the not supported features.
    std::ostringstream os;
    os << value;
    return os.str();
#endif
}

namespace rocksdb_ext
{
    namespace encryption
    {

        namespace
        {
            uint64_t GetBigEndian64(const unsigned char *buf)
            {
                if (rocksdb::port::kLittleEndian)
                {
                    return (static_cast<uint64_t>(buf[0]) << 56) +
                           (static_cast<uint64_t>(buf[1]) << 48) +
                           (static_cast<uint64_t>(buf[2]) << 40) +
                           (static_cast<uint64_t>(buf[3]) << 32) +
                           (static_cast<uint64_t>(buf[4]) << 24) +
                           (static_cast<uint64_t>(buf[5]) << 16) +
                           (static_cast<uint64_t>(buf[6]) << 8) +
                           (static_cast<uint64_t>(buf[7]));
                }
                else
                {
                    return *(reinterpret_cast<const uint64_t *>(buf));
                }
            }

            void PutBigEndian64(uint64_t value, unsigned char *buf)
            {
                if (rocksdb::port::kLittleEndian)
                {
                    buf[0] = static_cast<unsigned char>((value >> 56) & 0xff);
                    buf[1] = static_cast<unsigned char>((value >> 48) & 0xff);
                    buf[2] = static_cast<unsigned char>((value >> 40) & 0xff);
                    buf[3] = static_cast<unsigned char>((value >> 32) & 0xff);
                    buf[4] = static_cast<unsigned char>((value >> 24) & 0xff);
                    buf[5] = static_cast<unsigned char>((value >> 16) & 0xff);
                    buf[6] = static_cast<unsigned char>((value >> 8) & 0xff);
                    buf[7] = static_cast<unsigned char>(value & 0xff);
                }
                else
                {
                    *(reinterpret_cast<uint64_t *>(buf)) = value;
                }
            }
        } // anonymous namespace

        inline size_t KeySize(EncryptionMethod method)
        {
            switch (method)
            {
            case EncryptionMethod::kAES128_CTR:
                return 16;
            case EncryptionMethod::kAES192_CTR:
                return 24;
            case EncryptionMethod::kAES256_CTR:
                return 32;
            default:
                return 0;
            };
        }

        // AESCTRCipherStream use OpenSSL EVP API with CTR mode to encrypt and decrypt
        // data, instead of using the CTR implementation provided by
        // BlockAccessCipherStream. Benefits:
        //
        // 1. The EVP API automatically figure out if AES-NI can be enabled.
        // 2. Keep the data format consistent with OpenSSL (e.g. how IV is interpreted
        // as block counter).
        //
        // References for the openssl EVP API:
        // * man page: https://www.openssl.org/docs/man1.1.1/man3/EVP_EncryptUpdate.html
        // * SO answer for random access: https://stackoverflow.com/a/57147140/11014942
        // * https://medium.com/@amit.kulkarni/encrypting-decrypting-a-file-using-openssl-evp-b26e0e4d28d4
        Status AESCTRCipherStream::Cipher(uint64_t file_offset, char *data,
                                          size_t data_size, bool is_encrypt)
        {
#if OPENSSL_VERSION_NUMBER < 0x01000200f
            (void)file_offset;
            (void)data;
            (void)data_size;
            (void)is_encrypt;
            return Status::NotSupported("OpenSSL version < 1.0.2");
#else
            int ret = 1;
            EVP_CIPHER_CTX *ctx = nullptr;
            InitCipherContext(ctx);
            if (ctx == nullptr)
            {
                return Status::IOError("Failed to create cipher context.");
            }

            uint64_t block_index = file_offset / AES_BLOCK_SIZE;
            uint64_t block_offset = file_offset % AES_BLOCK_SIZE;

            // In CTR mode, OpenSSL EVP API treat the IV as a 128-bit big-endien, and
            // increase it by 1 for each block.
            //
            // In case of unsigned integer overflow in c++, the result is moduloed by
            // range, means only the lowest bits of the result will be kept.
            // http://www.cplusplus.com/articles/DE18T05o/
            uint64_t iv_high = initial_iv_high_;
            uint64_t iv_low = initial_iv_low_ + block_index;
            if (std::numeric_limits<uint64_t>::max() - block_index < initial_iv_low_)
            {
                iv_high++;
            }
            unsigned char iv[AES_BLOCK_SIZE];
            PutBigEndian64(iv_high, iv);
            PutBigEndian64(iv_low, iv + sizeof(uint64_t));

            ret = EVP_CipherInit(ctx, cipher_,
                                 reinterpret_cast<const unsigned char *>(key_.data()), iv,
                                 (is_encrypt ? 1 : 0));
            if (ret != 1)
            {
                return Status::IOError("Failed to init cipher.");
            }

            // Disable padding. After disabling padding, data size should always be
            // multiply of block size.
            ret = EVP_CIPHER_CTX_set_padding(ctx, 0);
            if (ret != 1)
            {
                return Status::IOError("Failed to disable padding for cipher context.");
            }

            uint64_t data_offset = 0;
            size_t remaining_data_size = data_size;
            int output_size = 0;
            unsigned char partial_block[AES_BLOCK_SIZE];

            // In the following we assume EVP_CipherUpdate allow in and out buffer are
            // the same, to save one memcpy. This is not specified in official man page.

            // Handle partial block at the beginning. The parital block is copied to
            // buffer to fake a full block.
            if (block_offset > 0)
            {
                size_t partial_block_size =
                    std::min<size_t>(AES_BLOCK_SIZE - block_offset, remaining_data_size);
                memcpy(partial_block + block_offset, data, partial_block_size);
                ret = EVP_CipherUpdate(ctx, partial_block, &output_size, partial_block,
                                       AES_BLOCK_SIZE);
                if (ret != 1)
                {
                    return Status::IOError("Crypter failed for first block, offset " +
                                           ToString(file_offset));
                }
                if (output_size != AES_BLOCK_SIZE)
                {
                    return Status::IOError(
                        "Unexpected crypter output size for first block, expected " +
                        ToString(AES_BLOCK_SIZE) + " vs actual " + ToString(output_size));
                }
                memcpy(data, partial_block + block_offset, partial_block_size);
                data_offset += partial_block_size;
                remaining_data_size -= partial_block_size;
            }

            // Handle full blocks in the middle.
            if (remaining_data_size >= AES_BLOCK_SIZE)
            {
                size_t actual_data_size =
                    remaining_data_size - remaining_data_size % AES_BLOCK_SIZE;
                unsigned char *full_blocks =
                    reinterpret_cast<unsigned char *>(data) + data_offset;
                ret = EVP_CipherUpdate(ctx, full_blocks, &output_size, full_blocks,
                                       static_cast<int>(actual_data_size));
                if (ret != 1)
                {
                    return Status::IOError("Crypter failed at offset " +
                                           ToString(file_offset + data_offset));
                }
                if (output_size != static_cast<int>(actual_data_size))
                {
                    return Status::IOError("Unexpected crypter output size, expected " +
                                           ToString(actual_data_size) + " vs actual " +
                                           ToString(output_size));
                }
                data_offset += actual_data_size;
                remaining_data_size -= actual_data_size;
            }

            // Handle partial block at the end. The parital block is copied to buffer to
            // fake a full block.
            if (remaining_data_size > 0)
            {
                assert(remaining_data_size < AES_BLOCK_SIZE);
                memcpy(partial_block, data + data_offset, remaining_data_size);
                ret = EVP_CipherUpdate(ctx, partial_block, &output_size, partial_block,
                                       AES_BLOCK_SIZE);
                if (ret != 1)
                {
                    return Status::IOError("Crypter failed for last block, offset " +
                                           ToString(file_offset + data_offset));
                }
                if (output_size != AES_BLOCK_SIZE)
                {
                    return Status::IOError(
                        "Unexpected crypter output size for last block, expected " +
                        ToString(AES_BLOCK_SIZE) + " vs actual " + ToString(output_size));
                }
                memcpy(data + data_offset, partial_block, remaining_data_size);
            }
            FreeCipherContext(ctx);
            return Status::OK();
#endif
        }

        Status NewAESCTRCipherStream(EncryptionMethod method, const std::string &key,
                                     const std::string &iv,
                                     std::unique_ptr<AESCTRCipherStream> *result)
        {
            assert(result != nullptr);
            const EVP_CIPHER *cipher = nullptr;
            switch (method)
            {
            case EncryptionMethod::kAES128_CTR:
                cipher = EVP_aes_128_ctr();
                break;
            case EncryptionMethod::kAES192_CTR:
                cipher = EVP_aes_192_ctr();
                break;
            case EncryptionMethod::kAES256_CTR:
                cipher = EVP_aes_256_ctr();
                break;
            default:
                return Status::InvalidArgument("Unsupported encryption method: " +
                                               ToString(static_cast<int>(method)));
            }
            if (key.size() != KeySize(method))
            {
                return Status::InvalidArgument("Encryption key size mismatch. " +
                                               ToString(key.size()) + "(actual) vs. " +
                                               ToString(KeySize(method)) + "(expected).");
            }
            if (iv.size() != AES_BLOCK_SIZE)
            {
                return Status::InvalidArgument(
                    "iv size not equal to block cipher block size: " + ToString(iv.size()) +
                    "(actual) vs. " + ToString(AES_BLOCK_SIZE) + "(expected).");
            }
            Slice iv_slice(iv);
            uint64_t iv_high =
                GetBigEndian64(reinterpret_cast<const unsigned char *>(iv.data()));
            uint64_t iv_low = GetBigEndian64(
                reinterpret_cast<const unsigned char *>(iv.data() + sizeof(uint64_t)));
            result->reset(new AESCTRCipherStream(cipher, key, iv_high, iv_low));
            return Status::OK();
        }

        Status AESEncryptionProvider::CreateNewPrefix(const std::string & /*fname*/, char *prefix, size_t prefix_length) const
        {
            // Create & seed rnd.
            srand(time(NULL));

            // Fill entire prefix block with random values to create IV for AES
            for (size_t i = 0; i < prefix_length; i++)
            {
                prefix[i] = (rand() % 256) & 0xFF;
            }
            return Status::OK();
        }

        Status AESEncryptionProvider::CreateCipherStream(
            const std::string &fname, const EnvOptions & /*options*/, Slice &prefix,
            std::unique_ptr<BlockAccessCipherStream> *result)
        {
            assert(result != nullptr);
            std::unique_ptr<AESCTRCipherStream> cipher_stream;
            std::string iv = std::string(prefix.data(), prefix.size());
            Status s = NewAESCTRCipherStream(method_, key_, iv,
                                             &cipher_stream);
            if (!s.ok())
            {
                return s;
            }
            *result = std::move(cipher_stream);
            return Status::OK();
        }

    } // namespace encryption
} // namespace rocksdb_ext

 #endif // OPENSSL
#endif // !ROCKSDB_LITE