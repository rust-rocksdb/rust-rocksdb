// Copyright 2019 Tyler Neely
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

use rocksdb::{Options, TemporaryDBPath, DB};

#[test]
fn property_test() {
    let n = TemporaryDBPath::new();
    {
        let db = DB::open_default(&n).unwrap();
        let value = db.property_value("rocksdb.stats").unwrap().unwrap();

        assert!(value.contains("Stats"));
    }
}

#[test]
fn property_cf_test() {
    let n = TemporaryDBPath::new();
    {
        let opts = Options::default();
        let db = DB::open_default(&n).unwrap();
        let cf = db.create_cf("cf1", &opts).unwrap();
        let value = db.property_value_cf(cf, "rocksdb.stats").unwrap().unwrap();

        assert!(value.contains("Stats"));
    }
}

#[test]
fn property_int_test() {
    let n = TemporaryDBPath::new();
    {
        let db = DB::open_default(&n).unwrap();
        let value = db
            .property_int_value("rocksdb.estimate-live-data-size")
            .unwrap();

        assert!(value == Some(0));
    }
}

#[test]
fn property_int_cf_test() {
    let n = TemporaryDBPath::new();
    {
        let opts = Options::default();
        let db = DB::open_default(&n).unwrap();
        let cf = db.create_cf("cf1", &opts).unwrap();
        let total_keys = db
            .property_int_value_cf(cf, "rocksdb.estimate-num-keys")
            .unwrap();

        assert!(total_keys == Some(0));
    }
}
