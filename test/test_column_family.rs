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
use rocksdb::{Options, RocksDB, RocksDBResult, Writable, Direction, MergeOperands};

#[test]
pub fn test_column_family() {
    let path = "_rust_rocksdb_cftest";

    // should be able to create column families
    {
        let mut opts = Options::new();
        opts.create_if_missing(true);
        opts.add_merge_operator("test operator", test_provided_merge);
        let mut db = RocksDB::open(&opts, path).unwrap();
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
        let mut opts = Options::new();
        opts.add_merge_operator("test operator", test_provided_merge);
        match RocksDB::open(&opts, path) {
            Ok(_) => panic!("should not have opened DB successfully without specifying column
            families"),
            Err(e) => assert!(e.starts_with("Invalid argument: You have to open all column families.")),
        }
    }

    // should properly open db when specyfing all column families
    {
        let mut opts = Options::new();
        opts.add_merge_operator("test operator", test_provided_merge);
        match RocksDB::open_cf(&opts, path, &["cf1"]) {
            Ok(_) => println!("successfully opened db with column family"),
            Err(e) => panic!("failed to open db with column family: {}", e),
        }
    }
    // should be able to write, read, merge, batch, and iterate over a cf
    {
        let mut opts = Options::new();
        opts.add_merge_operator("test operator", test_provided_merge);
        let mut db = match RocksDB::open_cf(&opts, path, &["cf1"]) {
            Ok(db) => {
                println!("successfully opened db with column family");
                db
            },
            Err(e) => panic!("failed to open db with column family: {}", e),
        };
        assert!(db.put_cf("cf1", b"k1", b"v1").is_ok());
        assert!(db.get_cf("cf1", b"k1").unwrap().to_utf8().unwrap() == "v1");
        let p = db.put_cf("cf1", b"k1", b"a");
        assert!(p.is_ok());
        db.merge_cf("cf1", b"k1", b"b");
        db.merge_cf("cf1", b"k1", b"c");
        db.merge_cf("cf1", b"k1", b"d");
        db.merge_cf("cf1", b"k1", b"efg");
        let m = db.merge_cf("cf1", b"k1", b"h");
        println!("m is {:?}", m);
        assert!(m.is_ok());
        db.get(b"k1").map( |value| {
                match value.to_utf8() {
                Some(v) =>
                println!("retrieved utf8 value: {}", v),
                None =>
                println!("did not read valid utf-8 out of the db"),
                }
                }).on_absent( || { println!("value not present!") })
        .on_error( |e| { println!("error reading value")}); //: {", e) });

        let r = db.get_cf("cf1", b"k1");
        assert!(r.unwrap().to_utf8().unwrap() == "abcdefgh");
        assert!(db.delete(b"k1").is_ok());
        assert!(db.get(b"k1").is_none());
    }
    // should b able to drop a cf
    {
        let mut db = RocksDB::open_cf(&Options::new(), path, &["cf1"]).unwrap();
        match db.drop_cf("cf1") {
            Ok(_) => println!("cf1 successfully dropped."),
            Err(e) => panic!("failed to drop column family: {}", e),
        }
    }

    assert!(RocksDB::destroy(&Options::new(), path).is_ok());
}

fn test_provided_merge(new_key: &[u8],
                       existing_val: Option<&[u8]>,
                       mut operands: &mut MergeOperands)
                       -> Vec<u8> {
    let nops = operands.size_hint().0;
    let mut result: Vec<u8> = Vec::with_capacity(nops);
    match existing_val {
        Some(v) => {
            for e in v {
                result.push(*e);
            }
        },
        None => (),
    }
    for op in operands {
        for e in op {
            result.push(*e);
        }
    }
    result
}
