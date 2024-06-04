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

#pragma once
#ifndef ROCKSDB_LITE
 #ifdef OPENSSL
#include <openssl/aes.h>
#include <openssl/evp.h>

#include <string>

#include "rocksdb/env_encryption.h"
#include "util/string_util.h"

using rocksdb::BlockAccessCipherStream;
using rocksdb::EncryptionProvider;
using rocksdb::EnvOptions;
using rocksdb::Slice;
using rocksdb::Status;

namespace rocksdb_ext
{
  namespace encryption
  {

#if OPENSSL_VERSION_NUMBER < 0x01010000f

#define InitCipherContext(ctx) \
  EVP_CIPHER_CTX ctx##_var;    \
  ctx = &ctx##_var;            \
  EVP_CIPHER_CTX_init(ctx);

// do nothing
#define FreeCipherContext(ctx)

#else

#define InitCipherContext(ctx)          \
  ctx = EVP_CIPHER_CTX_new();           \
  if (ctx != nullptr)                   \
  {                                     \
    if (EVP_CIPHER_CTX_reset(ctx) != 1) \
    {                                   \
      ctx = nullptr;                    \
    }                                   \
  }

#define FreeCipherContext(ctx) EVP_CIPHER_CTX_free(ctx);

#endif

    enum class EncryptionMethod : int
    {
      kAES128_CTR = 0,
      kAES192_CTR = 1,
      kAES256_CTR = 2,
    };

    class AESCTRCipherStream : public BlockAccessCipherStream
    {
    public:
      AESCTRCipherStream(const EVP_CIPHER *cipher, const std::string &key,
                         uint64_t iv_high, uint64_t iv_low)
          : cipher_(cipher),
            key_(key),
            initial_iv_high_(iv_high),
            initial_iv_low_(iv_low) {}

      ~AESCTRCipherStream() = default;

      size_t BlockSize() override
      {
        return AES_BLOCK_SIZE; // 16
      }

      Status Encrypt(uint64_t file_offset, char *data, size_t data_size) override
      {
        return Cipher(file_offset, data, data_size, true /*is_encrypt*/);
      }

      Status Decrypt(uint64_t file_offset, char *data, size_t data_size) override
      {
        return Cipher(file_offset, data, data_size, false /*is_encrypt*/);
      }

    protected:
      // Following methods required by BlockAccessCipherStream is unused.

      void AllocateScratch(std::string & /*scratch*/) override
      {
        // should not be called.
        assert(false);
      }

      Status EncryptBlock(uint64_t /*block_index*/, char * /*data*/,
                          char * /*scratch*/) override
      {
        return Status::NotSupported("EncryptBlock should not be called.");
      }

      Status DecryptBlock(uint64_t /*block_index*/, char * /*data*/,
                          char * /*scratch*/) override
      {
        return Status::NotSupported("DecryptBlock should not be called.");
      }

    private:
      Status Cipher(uint64_t file_offset, char *data, size_t data_size,
                    bool is_encrypt);

      const EVP_CIPHER *cipher_;
      const std::string key_;
      const uint64_t initial_iv_high_;
      const uint64_t initial_iv_low_;
    };

    extern Status NewAESCTRCipherStream(
        EncryptionMethod method, const std::string &key, const std::string &iv,
        std::unique_ptr<AESCTRCipherStream> *result);

    class AESEncryptionProvider : public EncryptionProvider
    {
    public:
      AESEncryptionProvider(const std::string &key, EncryptionMethod method) : key_(key), method_(method) {}
      virtual ~AESEncryptionProvider() {}
      const char *Name() const override { return "AESEncryptionProvider"; }

      size_t GetPrefixLength() const override
      {
        return AES_BLOCK_SIZE; // 16
      }

      Status
      CreateNewPrefix(const std::string & /*fname*/, char * /*prefix*/,
                      size_t /*prefix_length*/) const override;

      Status AddCipher(const std::string & /*descriptor*/, const char * /*cipher*/,
                       size_t /*len*/, bool /*for_write*/) override
      {
        return Status::NotSupported();
      }

      Status CreateCipherStream(
          const std::string &fname, const EnvOptions &options, Slice &prefix,
          std::unique_ptr<BlockAccessCipherStream> *result) override;

    private:
      std::string key_;
      EncryptionMethod method_;
    };

  } // namespace encryption
} // namespace rocksdb_ext

 #endif // OPENSSL
#endif // !ROCKSDB_LITE