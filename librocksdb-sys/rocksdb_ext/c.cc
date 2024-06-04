//  Copyright (c) 2011-present, Facebook, Inc.  All rights reserved.
//  This source code is licensed under the BSD-style license found in the
//  LICENSE file in the root directory of this source tree. An additional grant
//  of patent rights can be found in the PATENTS file in the same directory.
//
// Copyright (c) 2011 The LevelDB Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file. See the AUTHORS file for names of contributors.

#include "rocksdb_ext/c.h"
#include "rocksdb_ext/encrypted.h"
#include "rocksdb/env_encryption.h"
#include "rocksdb/db/c.cc"

//#if !defined(ROCKSDB_MAJOR) || !defined(ROCKSDB_MINOR) || \
//    !defined(ROCKSDB_PATCH)
// #error Only rocksdb 5.7.3+ is supported.
// #endif
//
// #if ROCKSDB_MAJOR * 10000 + ROCKSDB_MINOR * 100 + ROCKSDB_PATCH < 50703
// #error Only rocksdb 5.7.3+ is supported.
// #endif

using rocksdb::BlockCipher;
using rocksdb::EncryptionProvider;
using std::shared_ptr;

extern "C"
{

#ifdef OPENSSL
  rocksdb_env_t *crocksdb_ctr_aes_encrypted_env_create(rocksdb_env_t *base_env, const char *ciphertext, size_t ciphertext_len, uint32_t method)
  {
    auto result = new rocksdb_env_t;
    auto encryption_provider =
        std::make_shared<rocksdb_ext::encryption::AESEncryptionProvider>(std::string(ciphertext, ciphertext_len), static_cast<rocksdb_ext::encryption::EncryptionMethod>(method));
    result->rep = NewEncryptedEnv(base_env->rep, encryption_provider);
    result->is_default = false;
    return result;
  }
#endif

} // end extern "C"