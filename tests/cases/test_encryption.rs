// Copyright 2018 PingCAP, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// See the License for the specific language governing permissions and
// limitations under the License.

use std::sync::Arc;

use rocksdb::{DBOptions, Env, Writable, DB};

use super::tempdir_with_prefix;

#[test]
fn test_ctr_encrypted_env() {
    let test_cipher_texts: &[&[u8]] = &[
        &[16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1],
        &[8, 7, 6, 5, 4, 3, 2, 1],
    ];
    for ciphertext in test_cipher_texts {
        let base_env = Arc::new(Env::new_mem());
        test_ctr_encrypted_env_impl(Arc::new(
            Env::new_ctr_encrypted_env(Arc::clone(&base_env), ciphertext).unwrap(),
        ));
    }
    for ciphertext in test_cipher_texts {
        test_ctr_encrypted_env_impl(Arc::new(
            Env::new_default_ctr_encrypted_env(ciphertext).unwrap(),
        ));
    }
}

fn test_ctr_encrypted_env_impl(encrypted_env: Arc<Env>) {
    let path = tempdir_with_prefix("_rust_rocksdb_cryption_env");
    let path_str = path.path().to_str().unwrap();

    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.set_env(encrypted_env.clone());
    let db = DB::open(opts, path_str).unwrap();

    let samples = vec![
        (b"key1".to_vec(), b"value1".to_vec()),
        (b"key2".to_vec(), b"value2".to_vec()),
        (b"key3".to_vec(), b"value3".to_vec()),
        (b"key4".to_vec(), b"value4".to_vec()),
    ];
    for &(ref k, ref v) in &samples {
        db.put(k, v).unwrap();

        // check value
        assert_eq!(v.as_slice(), &*db.get(k).unwrap().unwrap());
    }

    // flush to sst file
    db.flush(true).unwrap();

    // check value in db
    for &(ref k, ref v) in &samples {
        assert_eq!(v.as_slice(), &*db.get(k).unwrap().unwrap());
    }

    // close db and open again.
    drop(db);
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.set_env(encrypted_env);
    let db = DB::open(opts, path_str).unwrap();

    // check value in db again
    for &(ref k, ref v) in &samples {
        assert_eq!(v.as_slice(), &*db.get(k).unwrap().unwrap());
    }
}
