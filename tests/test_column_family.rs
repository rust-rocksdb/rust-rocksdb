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

use pretty_assertions::assert_eq;

use rocksdb::{ColumnFamilyDescriptor, MergeOperands, Options, DB, DEFAULT_COLUMN_FAMILY_NAME};
use util::DBPath;

#[test]
fn test_column_family() {
    let n = DBPath::new("_rust_rocksdb_cftest");

    // should be able to create column families
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_merge_operator_associative("test operator", test_provided_merge);
        let mut db = DB::open(&opts, &n).unwrap();
        let opts = Options::default();
        match db.create_cf("cf1", &opts) {
            Ok(()) => println!("cf1 created successfully"),
            Err(e) => {
                panic!("could not create column family: {}", e);
            }
        }
    }

    // should fail to open db without specifying same column families
    {
        let mut opts = Options::default();
        opts.set_merge_operator_associative("test operator", test_provided_merge);
        match DB::open(&opts, &n) {
            Ok(_db) => panic!(
                "should not have opened DB successfully without \
                        specifying column
            families"
            ),
            Err(e) => assert!(e.to_string().starts_with("Invalid argument")),
        }
    }

    // should properly open db when specyfing all column families
    {
        let mut opts = Options::default();
        opts.set_merge_operator_associative("test operator", test_provided_merge);
        match DB::open_cf(&opts, &n, &["cf1"]) {
            Ok(_db) => println!("successfully opened db with column family"),
            Err(e) => panic!("failed to open db with column family: {}", e),
        }
    }

    // should be able to list a cf
    {
        let opts = Options::default();
        let vec = DB::list_cf(&opts, &n);
        match vec {
            Ok(vec) => assert_eq!(vec, vec![DEFAULT_COLUMN_FAMILY_NAME, "cf1"]),
            Err(e) => panic!("failed to drop column family: {}", e),
        }
    }

    // TODO should be able to use writebatch ops with a cf
    {}
    // TODO should be able to iterate over a cf
    {}
    // should b able to drop a cf
    {
        let mut db = DB::open_cf(&Options::default(), &n, &["cf1"]).unwrap();
        match db.drop_cf("cf1") {
            Ok(_) => println!("cf1 successfully dropped."),
            Err(e) => panic!("failed to drop column family: {}", e),
        }
    }
}

#[test]
fn test_can_open_db_with_results_of_list_cf() {
    // Test scenario derived from GitHub issue #175 and 177

    let n = DBPath::new("_rust_rocksdb_cftest_with_list_cf");

    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let mut db = DB::open(&opts, &n).unwrap();
        let opts = Options::default();

        assert!(db.create_cf("cf1", &opts).is_ok());
    }

    {
        let options = Options::default();
        let cfs = DB::list_cf(&options, &n).unwrap();
        let db = DB::open_cf(&options, &n, &cfs).unwrap();

        assert!(db.cf_handle("cf1").is_some());
    }
}

#[test]
fn test_create_missing_column_family() {
    let n = DBPath::new("_rust_rocksdb_missing_cftest");

    // should be able to create new column families when opening a new database
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        match DB::open_cf(&opts, &n, &["cf1"]) {
            Ok(_db) => println!("successfully created new column family"),
            Err(e) => panic!("failed to create new column family: {}", e),
        }
    }
}

#[test]
#[ignore]
fn test_merge_operator() {
    let n = DBPath::new("_rust_rocksdb_cftest_merge");
    // TODO should be able to write, read, merge, batch, and iterate over a cf
    {
        let mut opts = Options::default();
        opts.set_merge_operator_associative("test operator", test_provided_merge);
        let db = match DB::open_cf(&opts, &n, &["cf1"]) {
            Ok(db) => {
                println!("successfully opened db with column family");
                db
            }
            Err(e) => panic!("failed to open db with column family: {}", e),
        };
        let cf1 = db.cf_handle("cf1").unwrap();
        assert!(db.put_cf(cf1, b"k1", b"v1").is_ok());
        assert_eq!(db.get_cf(cf1, b"k1").unwrap().unwrap(), b"v1");
        let p = db.put_cf(cf1, b"k1", b"a");
        assert!(p.is_ok());
        db.merge_cf(cf1, b"k1", b"b").unwrap();
        db.merge_cf(cf1, b"k1", b"c").unwrap();
        db.merge_cf(cf1, b"k1", b"d").unwrap();
        db.merge_cf(cf1, b"k1", b"efg").unwrap();
        let m = db.merge_cf(cf1, b"k1", b"h");
        println!("m is {:?}", m);
        // TODO assert!(m.is_ok());
        match db.get(b"k1") {
            Ok(Some(value)) => match std::str::from_utf8(&value) {
                Ok(v) => println!("retrieved utf8 value: {}", v),
                Err(_) => println!("did not read valid utf-8 out of the db"),
            },
            Err(_) => println!("error reading value"),
            _ => panic!("value not present!"),
        }

        let _ = db.get_cf(cf1, b"k1");
        // TODO assert!(r.unwrap().as_ref() == b"abcdefgh");
        assert!(db.delete(b"k1").is_ok());
        assert!(db.get(b"k1").unwrap().is_none());
    }
}

fn test_provided_merge(
    _: &[u8],
    existing_val: Option<&[u8]>,
    operands: &mut MergeOperands,
) -> Option<Vec<u8>> {
    let nops = operands.size_hint().0;
    let mut result: Vec<u8> = Vec::with_capacity(nops);
    if let Some(v) = existing_val {
        for e in v {
            result.push(*e);
        }
    }
    for op in operands {
        for e in op {
            result.push(*e);
        }
    }
    Some(result)
}

#[test]
fn test_column_family_with_options() {
    let n = DBPath::new("_rust_rocksdb_cf_with_optionstest");
    {
        let mut cfopts = Options::default();
        cfopts.set_max_write_buffer_number(16);
        let cf_descriptor = ColumnFamilyDescriptor::new("cf1", cfopts);

        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cfs = vec![cf_descriptor];
        match DB::open_cf_descriptors(&opts, &n, cfs) {
            Ok(_db) => println!("created db with column family descriptors successfully"),
            Err(e) => {
                panic!(
                    "could not create new database with column family descriptors: {}",
                    e
                );
            }
        }
    }

    {
        let mut cfopts = Options::default();
        cfopts.set_max_write_buffer_number(16);
        let cf_descriptor = ColumnFamilyDescriptor::new("cf1", cfopts);

        let opts = Options::default();
        let cfs = vec![cf_descriptor];

        match DB::open_cf_descriptors(&opts, &n, cfs) {
            Ok(_db) => println!("successfully re-opened database with column family descriptors"),
            Err(e) => {
                panic!(
                    "unable to re-open database with column family descriptors: {}",
                    e
                );
            }
        }
    }
}

#[test]
fn test_create_duplicate_column_family() {
    let n = DBPath::new("_rust_rocksdb_create_duplicate_column_family");
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let mut db = match DB::open_cf(&opts, &n, &["cf1"]) {
            Ok(d) => d,
            Err(e) => panic!("failed to create new column family: {}", e),
        };

        assert!(db.create_cf("cf1", &opts).is_err());
    }
}
