// Copyright 2014 Tyler Neely
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
//
extern crate rocksdb;

use rocksdb::{DB, Options};
use std::thread;
use std::sync::Arc;

const N: usize = 100_000;

#[test]
pub fn test_multithreaded() {
    let path = "_rust_rocksdb_multithreadtest";
    {
        let db = DB::open_default(path).unwrap();
        let db = Arc::new(db);

        db.put(b"key", b"value1").unwrap();

        let db1 = db.clone();
        let j1 = thread::spawn(move || {
            for _ in 1..N {
                db1.put(b"key", b"value1").unwrap();
            }
        });

        let db2 = db.clone();
        let j2 = thread::spawn(move || {
            for _ in 1..N {
                db2.put(b"key", b"value2").unwrap();
            }
        });

        let db3 = db.clone();
        let j3 = thread::spawn(move || {
            for _ in 1..N {
                match db3.get(b"key") {
                    Ok(Some(v)) => {
                        if &v[..] != b"value1" && &v[..] != b"value2" {
                            assert!(false);
                        }
                    }
                    _ => {
                        assert!(false);
                    }
                }
            }
        });

        j1.join().unwrap();
        j2.join().unwrap();
        j3.join().unwrap();
    }
    assert!(DB::destroy(&Options::default(), path).is_ok());
}
