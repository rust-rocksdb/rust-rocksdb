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

extern crate libc;
extern crate rocksdb;

mod util;

use libc::size_t;

use rocksdb::{DBVector, Error, IteratorMode, Options, Snapshot, WriteBatch, DB};
use std::sync::Arc;
use std::{mem, thread};
use util::DBPath;

#[test]
fn test_db_vector() {
    use std::mem;
    let len: size_t = 4;
    let data: *mut u8 = unsafe { libc::calloc(len, mem::size_of::<u8>()) as *mut u8 };
    let v = unsafe { DBVector::from_c(data, len) };
    let ctrl = [0u8, 0, 0, 0];
    assert_eq!(&*v, &ctrl[..]);
}

#[test]
fn external() {
    let path = DBPath::new("_rust_rocksdb_externaltest");

    {
        let db = DB::open_default(&path).unwrap();

        assert!(db.put(b"k1", b"v1111").is_ok());

        let r: Result<Option<DBVector>, Error> = db.get(b"k1");

        assert!(r.unwrap().unwrap().to_utf8().unwrap() == "v1111");
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

        let r: Result<Option<DBVector>, Error> = db.get(b"k1");
        let vector = r.unwrap().unwrap();

        assert!(get_byte_slice(&vector) == b"v1111");
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
            let _ = batch.put(b"k1", b"v1111");
            assert_eq!(batch.len(), 1);
            assert!(!batch.is_empty());
            assert!(db.get(b"k1").unwrap().is_none());
            assert!(db.write(batch).is_ok());
            let r: Result<Option<DBVector>, Error> = db.get(b"k1");
            assert!(r.unwrap().unwrap().to_utf8().unwrap() == "v1111");
        }
        {
            // test delete
            let mut batch = WriteBatch::default();
            let _ = batch.delete(b"k1");
            assert_eq!(batch.len(), 1);
            assert!(!batch.is_empty());
            assert!(db.write(batch).is_ok());
            assert!(db.get(b"k1").unwrap().is_none());
        }
        {
            // test size_in_bytes
            let mut batch = WriteBatch::default();
            let before = batch.size_in_bytes();
            let _ = batch.put(b"k1", b"v1234567890");
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
        assert!(snap.get(b"k1").unwrap().unwrap().to_utf8().unwrap() == "v1111");

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

    fn check<K>(&self, key: K, value: &str) -> bool
    where
        K: AsRef<[u8]>,
    {
        self.snapshot.get(key).unwrap().unwrap().to_utf8().unwrap() == value
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
    let handler_1 = thread::spawn(move || wrapper_1.check("k1", "v1"));

    let wrapper_2 = wrapper.clone();
    let handler_2 = thread::spawn(move || wrapper_2.check("k2", "v2"));

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
