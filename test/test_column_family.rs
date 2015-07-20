/*
   Copyright 2014 Tyler Neely

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/
use rocksdb::{Options, RocksDB, Writable, Direction};

#[test]
pub fn test_column_family() {
    let path = "_rust_rocksdb_cftest";

    // should be able to create column families
    {
        let mut db = RocksDB::open_default(path).unwrap();
        let opts = Options::new();
        match db.create_cf("cf1", &opts) {
            Ok(_) => println!("cf1 created successfully"),
            Err(e) => {
                panic!("could not create column family: {}", e);
            },
        }
    }

    // should fail to open db without specifying same column families
    {
        match RocksDB::open_default(path) {
            Ok(_) => panic!("should not have opened DB successfully without specifying column
            families"),
            Err(e) => assert!(e.starts_with("Invalid argument: You have to open all column families.")),
        }
    }

    // should properly open db when specyfing all column families
    {
        match RocksDB::open_cf(&Options::new(), path, vec!["cf1"]) {
            Ok(_) => println!("successfully opened db with column family"),
            Err(e) => panic!("failed to open db with column family"),
        }

    }
    // should be able to write, read, merge, batch, and iterate over a cf
    {

    }

    assert!(RocksDB::destroy(&Options::new(), path).is_ok());
}
