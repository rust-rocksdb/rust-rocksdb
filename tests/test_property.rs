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

use rocksdb::{properties, Options, DB};
use util::DBPath;

#[test]
fn property_test() {
    let n = DBPath::new("_rust_rocksdb_property_test");
    {
        let db = DB::open_default(&n).unwrap();
        let value = db.property_value(properties::STATS).unwrap().unwrap();

        assert!(value.contains("Stats"));
    }
}

#[test]
fn property_cf_test() {
    let n = DBPath::new("_rust_rocksdb_property_cf_test");
    {
        let opts = Options::default();
        #[cfg(feature = "multi-threaded-cf")]
        let db = DB::open_default(&n).unwrap();
        #[cfg(not(feature = "multi-threaded-cf"))]
        let mut db = DB::open_default(&n).unwrap();
        db.create_cf("cf1", &opts).unwrap();
        let cf = db.cf_handle("cf1").unwrap();
        let value = db
            .property_value_cf(&cf, properties::STATS)
            .unwrap()
            .unwrap();

        assert!(value.contains("Stats"));
    }
}

#[test]
fn property_int_test() {
    let n = DBPath::new("_rust_rocksdb_property_int_test");
    {
        let db = DB::open_default(&n).unwrap();
        let value = db
            .property_int_value(properties::ESTIMATE_LIVE_DATA_SIZE)
            .unwrap();

        assert_eq!(value, Some(0));
    }
}

#[test]
fn property_int_cf_test() {
    let n = DBPath::new("_rust_rocksdb_property_int_cf_test");
    {
        let opts = Options::default();
        #[cfg(feature = "multi-threaded-cf")]
        let db = DB::open_default(&n).unwrap();
        #[cfg(not(feature = "multi-threaded-cf"))]
        let mut db = DB::open_default(&n).unwrap();
        db.create_cf("cf1", &opts).unwrap();
        let cf = db.cf_handle("cf1").unwrap();
        let total_keys = db
            .property_int_value_cf(&cf, properties::ESTIMATE_NUM_KEYS)
            .unwrap();

        assert_eq!(total_keys, Some(0));
    }
}
