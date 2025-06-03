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

use std::collections::HashMap;

use pretty_assertions::assert_eq;

use rocksdb::{Error, WriteBatch, WriteBatchIterator, WriteBatchIteratorCf, DB};
use util::DBPath;

#[test]
fn test_write_batch_clear() {
    let mut batch = WriteBatch::default();
    batch.put(b"1", b"2");
    assert_eq!(batch.len(), 1);
    batch.clear();
    assert_eq!(batch.len(), 0);
    assert!(batch.is_empty());
}

#[test]
fn test_write_batch_with_serialized_data() {
    struct Iterator {
        data: HashMap<Vec<u8>, Vec<u8>>,
    }

    impl WriteBatchIterator for Iterator {
        fn put(&mut self, key: &[u8], value: &[u8]) {
            match self.data.remove(key.as_ref()) {
                Some(expect) => {
                    assert_eq!(value, expect.as_slice());
                }
                None => {
                    panic!("key not exists");
                }
            }
        }

        fn delete(&mut self, _: &[u8]) {
            panic!("invalid delete operation");
        }
    }

    let mut kvs: HashMap<Vec<u8>, Vec<u8>> = HashMap::default();
    kvs.insert(vec![1], vec![2]);
    kvs.insert(vec![2], vec![3]);
    kvs.insert(vec![1, 2, 3, 4, 5], vec![4]);

    let mut b1 = WriteBatch::default();
    for (k, v) in &kvs {
        b1.put(k, v);
    }
    let data = b1.data();

    let b2 = WriteBatch::from_data(data);
    let mut it = Iterator { data: kvs };
    b2.iterate(&mut it);
}

#[test]
fn test_write_batch_cf_with_serialized_data() {
    struct Iterator {
        data: HashMap<Vec<u8>, (u32, Vec<u8>)>,
    }

    impl WriteBatchIteratorCf for Iterator {
        fn put_cf(&mut self, cf_id: u32, key: &[u8], value: &[u8]) {
            match self.data.remove(key.as_ref()) {
                Some((expect_cf_id, expect)) => {
                    assert_eq!(cf_id, expect_cf_id);
                    assert_eq!(value, expect.as_slice());
                }
                None => {
                    panic!("key not exists");
                }
            }
        }

        fn delete_cf(&mut self, _: u32, _: &[u8]) {
            panic!("invalid delete operation");
        }

        fn merge_cf(&mut self, _: u32, _: &[u8], _: &[u8]) {
            panic!("invalid merge operation");
        }
    }

    let mut kvs: HashMap<Vec<u8>, (u32, Vec<u8>)> = HashMap::default();
    kvs.insert(vec![1], (0, vec![2]));
    kvs.insert(vec![2], (0, vec![3]));
    kvs.insert(vec![1, 2, 3, 4, 5], (0, vec![4]));

    let mut b1 = WriteBatch::default();
    for (k, (_cf, val)) in &kvs {
        b1.put(k, val);
    }

    let data = b1.data();
    let b2 = WriteBatch::from_data(data);
    let mut it = Iterator { data: kvs };
    b2.iterate_cf(&mut it);
}

#[test]
fn test_write_batch_put_log_data() {
    let path = DBPath::new("writebatch_put_log_data");
    let db = DB::open_default(&path).unwrap();

    let mut batch = WriteBatch::default();
    batch.put(b"k1", b"v11111111");
    batch.put_log_data(b"log_data_value");

    let p = db.write(batch);
    assert!(p.is_ok());

    let r: Result<Option<Vec<u8>>, Error> = db.get(b"k1");
    assert_eq!(r.unwrap().unwrap(), b"v11111111");

    let mut called = false;

    let mut wal_iter = db.get_updates_since(0).unwrap();
    if let Ok((seq, write_batch)) = wal_iter.next().unwrap() {
        called = true;

        // Putting LOG data does not increase sequence number, only the put() call does
        assert_eq!(seq, 1);

        // there is only the put write in the WriteBatch
        assert_eq!(write_batch.len(), 1);

        // The WriteBatch data has the written "log_data"
        assert!(String::from_utf8(write_batch.data().to_vec())
            .unwrap()
            .contains("log_data_value"));
    }

    assert!(called);
}
