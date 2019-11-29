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

use std::ops;

use rand::{self, RngCore};
use rocksdb::*;

use super::tempdir_with_prefix;

fn initial_data(path: &str) -> DB {
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let mut cf_opts = ColumnFamilyOptions::new();
    // We will control the compaction manually.
    cf_opts.set_disable_auto_compactions(true);
    let db = DB::open_cf(opts, path, vec![("default", cf_opts)]).unwrap();

    {
        let handle = db.cf_handle("default").unwrap();
        generate_file_bottom_level(&db, handle, 0..3);
        generate_file_bottom_level(&db, handle, 3..6);
        generate_file_bottom_level(&db, handle, 6..9);
    }

    db
}

/// Generates a file with `range` and put it to the bottommost level.
fn generate_file_bottom_level(db: &DB, handle: &CFHandle, range: ops::Range<u32>) {
    for i in range {
        let k = format!("key{}", i);
        let v = format!("value{}", i);
        db.put_cf(handle, k.as_bytes(), v.as_bytes()).unwrap();
    }
    db.flush_cf(handle, true).unwrap();

    let opts = db.get_options_cf(handle);
    let mut compact_opts = CompactOptions::new();
    compact_opts.set_change_level(true);
    compact_opts.set_target_level(opts.get_num_levels() as i32 - 1);
    compact_opts.set_bottommost_level_compaction(DBBottommostLevelCompaction::Skip);
    db.compact_range_cf_opt(handle, &compact_opts, None, None);
}

#[test]
fn test_delete_files_in_range_with_iter() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_files_in_range_with_iter");
    let path_str = path.path().to_str().unwrap();
    let db = initial_data(path_str);

    // construct iterator before DeleteFilesInRange
    let mut iter = db.iter();

    // delete sst2
    db.delete_files_in_range(b"key2", b"key7", false).unwrap();

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
    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_files_in_range_with_snap");
    let path_str = path.path().to_str().unwrap();
    let db = initial_data(path_str);

    // construct snapshot before DeleteFilesInRange
    let snap = db.snapshot();

    // delete sst2
    db.delete_files_in_range(b"key2", b"key7", false).unwrap();

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
    let path = tempdir_with_prefix("_rocksdb_test_delete_files_in_range_with_delete_range");
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
    db.delete_files_in_range(b"1", b"4", false).unwrap();

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

#[test]
fn test_delete_files_in_ranges() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_files_in_multi_ranges");
    let path_str = path.path().to_str().unwrap();
    let db = initial_data(path_str);

    // Delete files in multiple overlapped ranges.
    // File ["key0", "key2"], ["key3", "key5"] should have been deleted,
    // but file ["key6", "key8"] should not be deleted because "key8" is exclusive.
    let mut ranges = Vec::new();
    ranges.push(Range::new(b"key0", b"key4"));
    ranges.push(Range::new(b"key2", b"key6"));
    ranges.push(Range::new(b"key4", b"key8"));

    let cf = db.cf_handle("default").unwrap();
    db.delete_files_in_ranges_cf(cf, &ranges, false).unwrap();

    // Check that ["key0", "key5"] have been deleted, but ["key6", "key8"] still exist.
    let mut iter = db.iter();
    iter.seek(SeekKey::Start);
    for i in 6..9 {
        assert!(iter.valid());
        let k = format!("key{}", i);
        assert_eq!(iter.key(), k.as_bytes());
        iter.next();
    }
    assert!(!iter.valid());

    // Delete the last file.
    let ranges = vec![Range::new(b"key6", b"key8")];
    db.delete_files_in_ranges_cf(cf, &ranges, true).unwrap();
    let mut iter = db.iter();
    iter.seek(SeekKey::Start);
    assert!(!iter.valid());
}
