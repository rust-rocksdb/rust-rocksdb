// Copyright 2020 Tyler Neely
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

mod util;

use std::{sync::Arc, thread};

use rocksdb::DB;
use util::DBPath;

const N: usize = 100_000;

#[test]
pub fn test_multithreaded() {
    let n = DBPath::new("_rust_rocksdb_multithreadtest");
    {
        let db = DB::open_default(&n).unwrap();
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

        let j3 = thread::spawn(move || {
            for _ in 1..N {
                let result = match db.get(b"key") {
                    Ok(Some(v)) => !(&v[..] != b"value1" && &v[..] != b"value2"),
                    _ => false,
                };
                assert!(result);
            }
        });
        j1.join().unwrap();
        j2.join().unwrap();
        j3.join().unwrap();
    }
}
