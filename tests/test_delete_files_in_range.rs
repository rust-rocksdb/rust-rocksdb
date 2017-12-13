// Copyright 2017 PingCAP, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// See the License for the specific language governing permissions and
// limitations under the License.

use rand::{self, Rng};
use rocksdb::*;
use tempdir::TempDir;

fn initial_data(path: &str) -> DB {
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let mut cf_opts = ColumnFamilyOptions::new();

    // DeleteFilesInRange ignore sst files in level 0,
    // this will makes all sst files fall into level 1.
    cf_opts.set_level_zero_file_num_compaction_trigger(1);
    let db = DB::open_cf(opts, path, vec![("default", cf_opts)]).unwrap();
    for i in 0..3 {
        let k = format!("key{}", i);
        let v = format!("value{}", i);
        db.put(k.as_bytes(), v.as_bytes()).unwrap();
        assert_eq!(v.as_bytes(), &*db.get(k.as_bytes()).unwrap().unwrap());
    }
    // sst1 [0, 3)
    db.flush(true).unwrap();

    for i in 3..6 {
        let k = format!("key{}", i);
        let v = format!("value{}", i);
        db.put(k.as_bytes(), v.as_bytes()).unwrap();
        assert_eq!(v.as_bytes(), &*db.get(k.as_bytes()).unwrap().unwrap());
    }
    // sst2 [3, 6)
    db.flush(true).unwrap();

    for i in 6..9 {
        let k = format!("key{}", i);
        let v = format!("value{}", i);
        db.put(k.as_bytes(), v.as_bytes()).unwrap();
        assert_eq!(v.as_bytes(), &*db.get(k.as_bytes()).unwrap().unwrap());
    }
    // sst3 [6, 9)
    db.flush(true).unwrap();

    db
}

#[test]
fn test_delete_files_in_range_with_iter() {
    let path = TempDir::new("_rust_rocksdb_test_delete_files_in_range_with_iter").expect("");
    let path_str = path.path().to_str().unwrap();
    let db = initial_data(path_str);

    // construct iterator before DeleteFilesInRange
    let mut iter = db.iter();

    // delete sst2
    db.delete_file_in_range(b"key2", b"key7").unwrap();

    let mut count = 0;
    assert!(iter.seek(SeekKey::Start));
    while iter.valid() {
        iter.next();
        count = count + 1;
    }

    // iterator will pin all sst files.
    assert_eq!(count, 9);
}

#[test]
fn test_delete_files_in_range_with_snap() {
    let path = TempDir::new("_rust_rocksdb_test_delete_files_in_range_with_snap").expect("");
    let path_str = path.path().to_str().unwrap();
    let db = initial_data(path_str);

    // construct snapshot before DeleteFilesInRange
    let snap = db.snapshot();

    // delete sst2
    db.delete_file_in_range(b"key2", b"key7").unwrap();

    let mut iter = snap.iter();
    assert!(iter.seek(SeekKey::Start));

    let mut count = 0;
    while iter.valid() {
        iter.next();
        count = count + 1;
    }

    // sst2 has been dropped.
    assert_eq!(count, 6);
}

#[test]
fn test_delete_files_in_range_with_delete_range() {
    // Regression test for https://github.com/facebook/rocksdb/issues/2833.
    let path = TempDir::new("_rocksdb_test_delete_files_in_range_with_delete_range").expect("");
    let path_str = path.path().to_str().unwrap();

    let sst_size = 1 << 10;
    let value_size = 8 << 10;
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let mut cf_opts = ColumnFamilyOptions::new();
    cf_opts.set_target_file_size_base(sst_size);
    cf_opts.set_level_zero_file_num_compaction_trigger(10);

    let db = DB::open_cf(opts, path_str, vec![("default", cf_opts)]).unwrap();

    // Flush 5 files in level 0.
    // File i will contain keys i and i+1.
    for i in 0..5 {
        let k1 = format!("{}", i);
        let k2 = format!("{}", i + 1);
        let mut v = vec![0; value_size];
        rand::thread_rng().fill_bytes(&mut v);
        db.put(k1.as_bytes(), v.as_slice()).unwrap();
        db.put(k2.as_bytes(), v.as_slice()).unwrap();
        db.flush(true).unwrap();
    }

    // Hold a snapshot to prevent the following delete range from dropping keys above.
    let snapshot = db.snapshot();
    db.delete_range(b"0", b"6").unwrap();
    db.flush(true).unwrap();
    // After this, we will have 3 files in level 1.
    // File i will contain keys i and i+1, and the delete range [0, 6).
    db.compact_range(None, None);
    drop(snapshot);

    // Before the fix, the file in the middle with keys 2 and 3 will be deleted,
    // which can be a problem when we compact later. After the fix, no file will
    // be deleted since they have an overlapped delete range [0, 6).
    db.delete_file_in_range(b"1", b"4").unwrap();

    // Flush a file with keys 4 and 5 to level 0.
    for i in 4..5 {
        let k1 = format!("{}", i);
        let k2 = format!("{}", i + 1);
        let mut v = vec![0; value_size];
        rand::thread_rng().fill_bytes(&mut v);
        db.put(k1.as_bytes(), v.as_slice()).unwrap();
        db.put(k2.as_bytes(), v.as_slice()).unwrap();
        db.flush(true).unwrap();
    }

    // After this, the delete range [0, 6) will drop all entries before it, so
    // we should have only keys 4 and 5.
    db.compact_range(None, None);

    let mut it = db.iter();
    it.seek(SeekKey::Start);
    assert!(it.valid());
    assert_eq!(it.key(), b"4");
    assert!(it.next());
    assert_eq!(it.key(), b"5");
    assert!(!it.next());
}
