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

use rocksdb::{properties, Options, TransactionDB, TransactionDBOptions};
use util::DBPath;

#[test]
fn transaction_db_property_test() {
    let path = DBPath::new("_rust_rocksdb_transaction_db_property_test");
    {
        let mut options = Options::default();
        options.create_if_missing(true);
        options.enable_statistics();
        let tx_db_options = TransactionDBOptions::default();
        let db = TransactionDB::open(&options, &tx_db_options, &path).unwrap();

        db.put("key1", "value1").unwrap();
        db.put("key2", "value2").unwrap();
        db.put("key3", "value3").unwrap();

        let prop_name: &std::ffi::CStr = properties::STATS;
        let value = db.property_value(prop_name).unwrap().unwrap();

        assert!(value.contains("Compaction Stats"));
        assert!(value.contains("Cumulative writes: 3 writes"));
    }
}

#[test]
fn transaction_db_int_property_test() {
    let path = DBPath::new("_rust_rocksdb_transaction_db_int_property_test");
    {
        let mut options = Options::default();
        options.create_if_missing(true);
        options.enable_statistics();
        let tx_db_options = TransactionDBOptions::default();
        let db = TransactionDB::open(&options, &tx_db_options, &path).unwrap();

        db.put("key1", "value1").unwrap();
        db.put("key2", "value2").unwrap();

        let prop_name: properties::PropertyName = properties::ESTIMATE_NUM_KEYS.to_owned();
        let value = db.property_int_value(&prop_name).unwrap().unwrap();

        assert_eq!(value, 2);
    }
}
