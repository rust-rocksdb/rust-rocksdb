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

use rocksdb::{
    ColumnFamilyOptions, CompactOptions, DBBottommostLevelCompaction, DBOptions, Range, Writable,
    DB,
};

use super::tempdir_with_prefix;

#[test]
fn test_compact_range() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_compact_range");
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    let samples = vec![
        (b"k1".to_vec(), b"value--------1".to_vec()),
        (b"k2".to_vec(), b"value--------2".to_vec()),
        (b"k3".to_vec(), b"value--------3".to_vec()),
        (b"k4".to_vec(), b"value--------4".to_vec()),
        (b"k5".to_vec(), b"value--------5".to_vec()),
    ];
    for &(ref k, ref v) in &samples {
        db.put(k, v).unwrap();
        assert_eq!(v.as_slice(), &*db.get(k).unwrap().unwrap());
    }

    // flush memtable to sst file
    db.flush(true).unwrap();
    let old_size = db.get_approximate_sizes(&[Range::new(b"k0", b"k6")])[0];

    // delete all and compact whole range
    for &(ref k, _) in &samples {
        db.delete(k).unwrap()
    }
    db.compact_range(None, None);
    let new_size = db.get_approximate_sizes(&[Range::new(b"k0", b"k6")])[0];
    assert!(old_size > new_size);
}

#[test]
fn test_compact_range_change_level() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_compact_range_change_level");
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let mut cf_opts = ColumnFamilyOptions::new();
    cf_opts.set_level_zero_file_num_compaction_trigger(10);
    let db = DB::open_cf(
        opts,
        path.path().to_str().unwrap(),
        vec![("default", cf_opts)],
    )
    .unwrap();
    let samples = vec![
        (b"k1".to_vec(), b"value--------1".to_vec()),
        (b"k2".to_vec(), b"value--------2".to_vec()),
        (b"k3".to_vec(), b"value--------3".to_vec()),
        (b"k4".to_vec(), b"value--------4".to_vec()),
        (b"k5".to_vec(), b"value--------5".to_vec()),
    ];
    for &(ref k, ref v) in &samples {
        db.put(k, v).unwrap();
        db.flush(true).unwrap();
    }

    let compact_level = 1;
    let mut compact_opts = CompactOptions::new();
    compact_opts.set_change_level(true);
    compact_opts.set_target_level(compact_level);
    let handle = db.cf_handle("default").unwrap();
    db.compact_range_cf_opt(handle, &compact_opts, None, None);
    let name = format!("rocksdb.num-files-at-level{}", compact_level);
    assert_eq!(db.get_property_int(&name).unwrap(), samples.len() as u64);
}

#[test]
fn test_compact_range_bottommost_level_compaction() {
    let path = tempdir_with_prefix("test_compact_range_bottommost_level_compaction");
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);

    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    db.put(&[0], &[0]).unwrap();
    db.flush(true).unwrap();

    // Compact to bottommost level
    let cf_handle = db.cf_handle("default").unwrap();
    let cf_opts = db.get_options_cf(cf_handle);
    let bottommost_level = (cf_opts.get_num_levels() - 1) as i32;
    let mut compact_opts = CompactOptions::new();
    compact_opts.set_change_level(true);
    compact_opts.set_target_level(bottommost_level);
    db.compact_range_cf_opt(cf_handle, &compact_opts, None, None);

    let metadata = db.get_column_family_meta_data(cf_handle);
    let bottommost_files = metadata.get_levels().last().unwrap().get_files();
    assert_eq!(bottommost_files.len(), 1);
    let bottommost_filename = bottommost_files[0].get_name();

    // Skip bottommost level compaction
    compact_opts.set_bottommost_level_compaction(DBBottommostLevelCompaction::Skip);
    db.compact_range_cf_opt(cf_handle, &compact_opts, None, None);
    let metadata = db.get_column_family_meta_data(cf_handle);
    let bottommost_files = metadata.get_levels().last().unwrap().get_files();
    assert_eq!(bottommost_files.len(), 1);
    assert_eq!(bottommost_filename, bottommost_files[0].get_name());

    // Force bottommost level compaction
    compact_opts.set_bottommost_level_compaction(DBBottommostLevelCompaction::Force);
    db.compact_range_cf_opt(cf_handle, &compact_opts, None, None);
    let metadata = db.get_column_family_meta_data(cf_handle);
    let bottommost_files = metadata.get_levels().last().unwrap().get_files();
    assert_eq!(bottommost_files.len(), 1);
    assert_ne!(bottommost_filename, bottommost_files[0].get_name());
}
