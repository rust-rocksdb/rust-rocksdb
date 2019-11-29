// Copyright 2018 PingCAP, Inc.
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
    CFHandle, ColumnFamilyOptions, CompactionOptions, DBCompressionType, DBOptions, Writable, DB,
};

use super::tempdir_with_prefix;

#[test]
fn test_metadata() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_metadata");
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let mut cf_opts = ColumnFamilyOptions::new();
    cf_opts.set_disable_auto_compactions(true);
    let db = DB::open_cf(
        opts,
        path.path().to_str().unwrap(),
        vec![("default", cf_opts)],
    )
    .unwrap();
    let cf_handle = db.cf_handle("default").unwrap();

    let num_files = 5;
    for i in 0..num_files {
        db.put(&[i], &[i]).unwrap();
        db.flush(true).unwrap();
    }

    let cf_meta = db.get_column_family_meta_data(cf_handle);
    let cf_levels = cf_meta.get_levels();
    assert_eq!(cf_levels.len(), 7);
    for (i, cf_level) in cf_levels.iter().enumerate() {
        let files = cf_level.get_files();
        if i != 0 {
            assert_eq!(files.len(), 0);
            continue;
        }
        assert_eq!(files.len(), num_files as usize);
        for f in files {
            assert!(f.get_size() > 0);
            assert!(f.get_name().len() > 0);
            assert!(f.get_smallestkey().len() > 0);
            assert!(f.get_largestkey().len() > 0);
        }
    }
}

fn get_files_cf(db: &DB, cf: &CFHandle, max_level: usize) -> Vec<String> {
    let mut files = Vec::new();
    let cf_meta = db.get_column_family_meta_data(cf);
    for (i, level) in cf_meta.get_levels().iter().enumerate() {
        if i > max_level {
            break;
        }
        for f in level.get_files() {
            files.push(f.get_name());
        }
    }
    files
}

#[test]
fn test_compact_files() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_metadata");
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let mut cf_opts = ColumnFamilyOptions::new();
    cf_opts.set_disable_auto_compactions(true);
    let db = DB::open_cf(
        opts,
        path.path().to_str().unwrap(),
        vec![("default", cf_opts)],
    )
    .unwrap();
    let cf_handle = db.cf_handle("default").unwrap();

    let cf_opts = db.get_options_cf(cf_handle);
    let output_file_size = cf_opts.get_target_file_size_base();

    let mut opts = CompactionOptions::new();
    opts.set_compression(DBCompressionType::Zstd);
    opts.set_output_file_size_limit(output_file_size as usize);

    let num_files = 5;
    for i in 0..num_files {
        let b = &[i as u8];
        db.put(b, b).unwrap();
        db.flush(true).unwrap();
    }
    let input_files = get_files_cf(&db, cf_handle, 0);
    assert_eq!(input_files.len(), num_files);
    db.compact_files_cf(cf_handle, &opts, &input_files, 0)
        .unwrap();
    assert_eq!(get_files_cf(&db, cf_handle, 0).len(), 1);
}
