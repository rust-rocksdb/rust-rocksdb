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

use rocksdb::{Error, IteratorMode, Options, Snapshot, WriteBatch, DB};
use std::sync::Arc;
use std::time::Duration;
use std::{mem, thread};
use util::DBPath;

#[test]
fn external() {
    let path = DBPath::new("_rust_rocksdb_externaltest");

    {
        let db = DB::open_default(&path).unwrap();

        assert!(db.put(b"k1", b"v1111").is_ok());

        let r: Result<Option<Vec<u8>>, Error> = db.get(b"k1");

        assert_eq!(r.unwrap().unwrap(), b"v1111");
        assert!(db.delete(b"k1").is_ok());
        assert!(db.get(b"k1").unwrap().is_none());
    }
}

#[test]
fn db_vector_as_ref_byte_slice() {
    let path = DBPath::new("_rust_rocksdb_db_vector_as_ref_byte_slice");

    {
        let db = DB::open_default(&path).unwrap();

        assert!(db.put(b"k1", b"v1111").is_ok());

        let result = db.get(b"k1").unwrap().unwrap();

        assert_eq!(get_byte_slice(&result), b"v1111");
    }
}

fn get_byte_slice<T: AsRef<[u8]>>(source: &'_ T) -> &'_ [u8] {
    source.as_ref()
}

#[test]
fn errors_do_stuff() {
    let path = DBPath::new("_rust_rocksdb_error");
    let _db = DB::open_default(&path).unwrap();
    let opts = Options::default();
    // The DB will still be open when we try to destroy it and the lock should fail.
    match DB::destroy(&opts, &path) {
        Err(s) => {
            let message = s.to_string();
            assert!(message.find("IO error:").is_some());
            assert!(message.find("_rust_rocksdb_error").is_some());
            assert!(message.find("/LOCK:").is_some());
        }
        Ok(_) => panic!("should fail"),
    }
}

#[test]
fn writebatch_works() {
    let path = DBPath::new("_rust_rocksdb_writebacktest");
    {
        let db = DB::open_default(&path).unwrap();
        {
            // test put
            let mut batch = WriteBatch::default();
            assert!(db.get(b"k1").unwrap().is_none());
            assert_eq!(batch.len(), 0);
            assert!(batch.is_empty());
            batch.put(b"k1", b"v1111");
            assert_eq!(batch.len(), 1);
            assert!(!batch.is_empty());
            assert!(db.get(b"k1").unwrap().is_none());
            assert!(db.write(batch).is_ok());
            let r: Result<Option<Vec<u8>>, Error> = db.get(b"k1");
            assert_eq!(r.unwrap().unwrap(), b"v1111");
        }
        {
            // test delete
            let mut batch = WriteBatch::default();
            batch.delete(b"k1");
            assert_eq!(batch.len(), 1);
            assert!(!batch.is_empty());
            assert!(db.write(batch).is_ok());
            assert!(db.get(b"k1").unwrap().is_none());
        }
        {
            // test size_in_bytes
            let mut batch = WriteBatch::default();
            let before = batch.size_in_bytes();
            batch.put(b"k1", b"v1234567890");
            let after = batch.size_in_bytes();
            assert!(before + 10 <= after);
        }
    }
}

#[test]
fn iterator_test() {
    let path = DBPath::new("_rust_rocksdb_iteratortest");
    {
        let data = [(b"k1", b"v1111"), (b"k2", b"v2222"), (b"k3", b"v3333")];
        let db = DB::open_default(&path).unwrap();

        for (key, value) in &data {
            assert!(db.put(key, value).is_ok());
        }

        let iter = db.iterator(IteratorMode::Start);

        for (idx, (db_key, db_value)) in iter.enumerate() {
            let (key, value) = data[idx];
            assert_eq!((&key[..], &value[..]), (db_key.as_ref(), db_value.as_ref()));
        }
    }
}

#[test]
fn snapshot_test() {
    let path = DBPath::new("_rust_rocksdb_snapshottest");
    {
        let db = DB::open_default(&path).unwrap();

        assert!(db.put(b"k1", b"v1111").is_ok());

        let snap = db.snapshot();
        assert_eq!(snap.get(b"k1").unwrap().unwrap(), b"v1111");

        assert!(db.put(b"k2", b"v2222").is_ok());

        assert!(db.get(b"k2").unwrap().is_some());
        assert!(snap.get(b"k2").unwrap().is_none());
    }
}

#[derive(Clone)]
struct SnapshotWrapper {
    snapshot: Arc<Snapshot<'static>>,
}

impl SnapshotWrapper {
    fn new(db: &DB) -> Self {
        Self {
            snapshot: Arc::new(unsafe { mem::transmute(db.snapshot()) }),
        }
    }

    fn check<K>(&self, key: K, value: &[u8]) -> bool
    where
        K: AsRef<[u8]>,
    {
        self.snapshot.get(key).unwrap().unwrap() == value
    }
}

#[test]
fn sync_snapshot_test() {
    let path = DBPath::new("_rust_rocksdb_sync_snapshottest");
    let db = DB::open_default(&path).unwrap();

    assert!(db.put(b"k1", b"v1").is_ok());
    assert!(db.put(b"k2", b"v2").is_ok());

    let wrapper = SnapshotWrapper::new(&db);
    let wrapper_1 = wrapper.clone();
    let handler_1 = thread::spawn(move || wrapper_1.check("k1", b"v1"));
    let handler_2 = thread::spawn(move || wrapper.check("k2", b"v2"));

    assert!(handler_1.join().unwrap());
    assert!(handler_2.join().unwrap());
}

#[test]
fn set_option_test() {
    let path = DBPath::new("_rust_rocksdb_set_optionstest");
    {
        let db = DB::open_default(&path).unwrap();
        // set an option to valid values
        assert!(db
            .set_options(&[("disable_auto_compactions", "true")])
            .is_ok());
        assert!(db
            .set_options(&[("disable_auto_compactions", "false")])
            .is_ok());
        // invalid names/values should result in an error
        assert!(db
            .set_options(&[("disable_auto_compactions", "INVALID_VALUE")])
            .is_err());
        assert!(db
            .set_options(&[("INVALID_NAME", "INVALID_VALUE")])
            .is_err());
        // option names/values must not contain NULLs
        assert!(db
            .set_options(&[("disable_auto_compactions", "true\0")])
            .is_err());
        assert!(db
            .set_options(&[("disable_auto_compactions\0", "true")])
            .is_err());
        // empty options are not allowed
        assert!(db.set_options(&[]).is_err());
        // multiple options can be set in a single API call
        let multiple_options = [
            ("paranoid_file_checks", "true"),
            ("report_bg_io_stats", "true"),
        ];
        db.set_options(&multiple_options).unwrap();
    }
}

#[test]
fn test_sequence_number() {
    let path = DBPath::new("_rust_rocksdb_test_sequence_number");
    {
        let db = DB::open_default(&path).unwrap();
        assert_eq!(db.latest_sequence_number(), 0);
        let _ = db.put(b"key", b"value");
        assert_eq!(db.latest_sequence_number(), 1);
    }
}

struct OperationCounts {
    puts: usize,
    deletes: usize,
}

impl rocksdb::WriteBatchIterator for OperationCounts {
    fn put(&mut self, _key: Box<[u8]>, _value: Box<[u8]>) {
        self.puts += 1;
    }
    fn delete(&mut self, _key: Box<[u8]>) {
        self.deletes += 1;
    }
}

#[test]
fn test_get_updates_since_empty() {
    let path = DBPath::new("_rust_rocksdb_test_get_updates_since_empty");
    let db = DB::open_default(&path).unwrap();
    // get_updates_since() on an empty database
    let mut iter = db.get_updates_since(0).unwrap();
    assert!(iter.next().is_none());
}

#[test]
fn test_get_updates_since_multiple_batches() {
    let path = DBPath::new("_rust_rocksdb_test_get_updates_since_multiple_batches");
    let db = DB::open_default(&path).unwrap();
    // add some records and collect sequence numbers,
    // verify 3 batches of 1 put each were done
    db.put(b"key1", b"value1").unwrap();
    let seq1 = db.latest_sequence_number();
    db.put(b"key2", b"value2").unwrap();
    db.put(b"key3", b"value3").unwrap();
    db.put(b"key4", b"value4").unwrap();
    let mut iter = db.get_updates_since(seq1).unwrap();
    let mut counts = OperationCounts {
        puts: 0,
        deletes: 0,
    };
    let (seq, batch) = iter.next().unwrap();
    assert_eq!(seq, 2);
    batch.iterate(&mut counts);
    let (seq, batch) = iter.next().unwrap();
    assert_eq!(seq, 3);
    batch.iterate(&mut counts);
    let (seq, batch) = iter.next().unwrap();
    assert_eq!(seq, 4);
    batch.iterate(&mut counts);
    assert!(iter.next().is_none());
    assert_eq!(counts.puts, 3);
    assert_eq!(counts.deletes, 0);
}

#[test]
fn test_get_updates_since_one_batch() {
    let path = DBPath::new("_rust_rocksdb_test_get_updates_since_one_batch");
    let db = DB::open_default(&path).unwrap();
    db.put(b"key2", b"value2").unwrap();
    // some puts and deletes in a single batch,
    // verify 1 put and 1 delete were done
    let seq1 = db.latest_sequence_number();
    assert_eq!(seq1, 1);
    let mut batch = WriteBatch::default();
    batch.put(b"key1", b"value1");
    batch.delete(b"key2");
    db.write(batch).unwrap();
    assert_eq!(db.latest_sequence_number(), 3);
    let mut iter = db.get_updates_since(seq1).unwrap();
    let mut counts = OperationCounts {
        puts: 0,
        deletes: 0,
    };
    let (seq, batch) = iter.next().unwrap();
    assert_eq!(seq, 2);
    batch.iterate(&mut counts);
    assert!(iter.next().is_none());
    assert_eq!(counts.puts, 1);
    assert_eq!(counts.deletes, 1);
}

#[test]
fn test_get_updates_since_nothing() {
    let path = DBPath::new("_rust_rocksdb_test_get_updates_since_nothing");
    let db = DB::open_default(&path).unwrap();
    // get_updates_since() with no new changes
    db.put(b"key1", b"value1").unwrap();
    let seq1 = db.latest_sequence_number();
    let mut iter = db.get_updates_since(seq1).unwrap();
    assert!(iter.next().is_none());
}

#[test]
fn test_get_updates_since_out_of_range() {
    let path = DBPath::new("_rust_rocksdb_test_get_updates_since_out_of_range");
    let db = DB::open_default(&path).unwrap();
    db.put(b"key1", b"value1").unwrap();
    // get_updates_since() with an out of bounds sequence number
    let result = db.get_updates_since(1000);
    assert!(result.is_err());
}

#[test]
fn test_open_as_secondary() {
    let primary_path = DBPath::new("_rust_rocksdb_test_open_as_secondary_primary");

    let db = DB::open_default(&primary_path).unwrap();
    db.put(b"key1", b"value1").unwrap();

    let mut opts = Options::default();
    opts.set_max_open_files(-1);

    let secondary_path = DBPath::new("_rust_rocksdb_test_open_as_secondary_secondary");
    let secondary = DB::open_as_secondary(&opts, &primary_path, &secondary_path).unwrap();

    let result = secondary.get(b"key1").unwrap().unwrap();
    assert_eq!(get_byte_slice(&result), b"value1");

    db.put(b"key1", b"value2").unwrap();
    assert!(secondary.try_catch_up_with_primary().is_ok());

    let result = secondary.get(b"key1").unwrap().unwrap();
    assert_eq!(get_byte_slice(&result), b"value2");
}

#[test]
fn test_open_with_ttl() {
    let path = DBPath::new("_rust_rocksdb_test_open_with_ttl");

    let mut opts = Options::default();
    opts.create_if_missing(true);
    let db = DB::open_with_ttl(&opts, &path, Duration::from_secs(1)).unwrap();
    db.put(b"key1", b"value1").unwrap();

    thread::sleep(Duration::from_secs(2));
    // Trigger a manual compaction, this will check the TTL filter
    // in the database and drop all expired entries.
    db.compact_range(None::<&[u8]>, None::<&[u8]>);
    assert!(db.get(b"key1").unwrap().is_none());
}
