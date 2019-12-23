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

use std::fs;

use crc::crc32::{self, Digest, Hasher32};
use rocksdb::*;

use super::tempdir_with_prefix;

fn gen_sst(opt: ColumnFamilyOptions, cf: Option<&CFHandle>, path: &str) {
    let _ = fs::remove_file(path);
    let env_opt = EnvOptions::new();
    let mut writer = if cf.is_some() {
        SstFileWriter::new_cf(env_opt, opt, cf.unwrap())
    } else {
        SstFileWriter::new(env_opt, opt)
    };
    writer.open(path).unwrap();
    writer.put(b"key1", b"value1").unwrap();
    writer.put(b"key2", b"value2").unwrap();
    writer.put(b"key3", b"value3").unwrap();
    writer.put(b"key4", b"value4").unwrap();
    writer.finish().unwrap();
}

fn gen_sst_from_db(opt: ColumnFamilyOptions, cf: Option<&CFHandle>, path: &str, db: &DB) {
    let _ = fs::remove_file(path);
    let env_opt = EnvOptions::new();
    let mut writer = if cf.is_some() {
        SstFileWriter::new_cf(env_opt, opt, cf.unwrap())
    } else {
        SstFileWriter::new(env_opt, opt)
    };
    writer.open(path).unwrap();
    let mut iter = db.iter();
    iter.seek(SeekKey::Start).unwrap();
    while iter.valid().unwrap() {
        writer.put(iter.key(), iter.value()).unwrap();
        iter.next().unwrap();
    }
    writer.finish().unwrap();
}

fn gen_crc32_from_db(db: &DB) -> u32 {
    let mut digest = Digest::new(crc32::IEEE);
    let mut iter = db.iter();
    iter.seek(SeekKey::Start).unwrap();
    while iter.valid().unwrap() {
        digest.write(iter.key());
        digest.write(iter.value());
        iter.next().unwrap();
    }
    digest.sum32()
}

fn gen_crc32_from_db_in_range(db: &DB, start_key: &[u8], end_key: &[u8]) -> u32 {
    let mut digest = Digest::new(crc32::IEEE);
    let mut iter = db.iter();
    iter.seek(SeekKey::Key(start_key)).unwrap();
    while iter.valid().unwrap() {
        if iter.key() >= end_key {
            break;
        }
        digest.write(iter.key());
        digest.write(iter.value());
        iter.next().unwrap();
    }
    digest.sum32()
}

#[test]
fn test_delete_range_case_1() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_case_1");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let db = DB::open(opts, path_str).unwrap();
    let samples_a = vec![
        (b"key1", b"value1"),
        (b"key2", b"value2"),
        (b"key3", b"value3"),
        (b"key4", b"value4"),
    ];
    for (k, v) in samples_a {
        db.put(k, v).unwrap();
        assert_eq!(v, &*db.get(k).unwrap().unwrap());
    }
    let before = gen_crc32_from_db(&db);

    let gen_path = tempdir_with_prefix("_rust_rocksdb_case_1_ingest_sst_gen");
    let test_sstfile = gen_path.path().join("test_sst_file");
    let test_sstfile_str = test_sstfile.to_str().unwrap();
    let ingest_opt = IngestExternalFileOptions::new();

    let default_options = db.get_options();
    gen_sst_from_db(
        default_options,
        db.cf_handle("default"),
        test_sstfile_str,
        &db,
    );

    db.delete_range(b"key1", b"key5").unwrap();
    check_kv(
        &db,
        db.cf_handle("default"),
        &[
            (b"key1", None),
            (b"key2", None),
            (b"key3", None),
            (b"key4", None),
        ],
    );

    db.ingest_external_file(&ingest_opt, &[test_sstfile_str])
        .unwrap();
    check_kv(
        &db,
        db.cf_handle("default"),
        &[
            (b"key1", Some(b"value1")),
            (b"key2", Some(b"value2")),
            (b"key3", Some(b"value3")),
            (b"key4", Some(b"value4")),
        ],
    );

    let after = gen_crc32_from_db(&db);
    assert_eq!(before, after);
}

#[test]
fn test_delete_range_case_2() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_case_2_1");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let db = DB::open(opts, path_str).unwrap();
    let samples_a = vec![
        (b"key1", b"value1"),
        (b"key2", b"value2"),
        (b"key3", b"value3"),
        (b"key4", b"value4"),
    ];
    for (k, v) in samples_a {
        db.put(k, v).unwrap();
        assert_eq!(v, &*db.get(k).unwrap().unwrap());
    }
    let before = gen_crc32_from_db(&db);

    let gen_path = tempdir_with_prefix("_rust_rocksdb_case_2_ingest_sst_gen");
    let test_sstfile = gen_path.path().join("test_sst_file");
    let test_sstfile_str = test_sstfile.to_str().unwrap();
    let ingest_opt = IngestExternalFileOptions::new();

    let default_options = db.get_options();
    gen_sst_from_db(
        default_options,
        db.cf_handle("default"),
        test_sstfile_str,
        &db,
    );

    db.delete_range(b"key1", b"key5").unwrap();
    check_kv(
        &db,
        db.cf_handle("default"),
        &[
            (b"key1", None),
            (b"key2", None),
            (b"key3", None),
            (b"key4", None),
        ],
    );

    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_case_2_2");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let db2 = DB::open(opts, path_str).unwrap();

    db2.ingest_external_file(&ingest_opt, &[test_sstfile_str])
        .unwrap();
    check_kv(
        &db2,
        db2.cf_handle("default"),
        &[
            (b"key1", Some(b"value1")),
            (b"key2", Some(b"value2")),
            (b"key3", Some(b"value3")),
            (b"key4", Some(b"value4")),
        ],
    );

    let after = gen_crc32_from_db(&db2);
    assert_eq!(before, after);
}

#[test]
fn test_delete_range_case_3() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_case_3");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let db = DB::open(opts, path_str).unwrap();
    let samples_a = vec![
        (b"key1", b"value1"),
        (b"key2", b"value2"),
        (b"key3", b"value3"),
        (b"key4", b"value4"),
        (b"key5", b"value5"),
    ];
    for (k, v) in samples_a {
        db.put(k, v).unwrap();
        assert_eq!(v, &*db.get(k).unwrap().unwrap());
    }
    let before = gen_crc32_from_db(&db);

    db.delete_range(b"key2", b"key4").unwrap();

    check_kv(
        &db,
        db.cf_handle("default"),
        &[
            (b"key1", Some(b"value1")),
            (b"key2", None),
            (b"key3", None),
            (b"key4", Some(b"value4")),
            (b"key5", Some(b"value5")),
        ],
    );

    let path2 = tempdir_with_prefix("_rust_rocksdb_test_delete_range_case_3_2");
    let path_str2 = path2.path().to_str().unwrap();
    let mut opts2 = DBOptions::new();
    opts2.create_if_missing(true);
    let db2 = DB::open(opts2, path_str2).unwrap();

    let samples_b = vec![(b"key2", b"value2"), (b"key3", b"value3")];
    for (k, v) in samples_b {
        db2.put(k, v).unwrap();
        assert_eq!(v, &*db2.get(k).unwrap().unwrap());
    }

    let gen_path = tempdir_with_prefix("_rust_rocksdb_case_3_ingest_sst_gen");
    let test_sstfile = gen_path.path().join("test_sst_file");
    let test_sstfile_str = test_sstfile.to_str().unwrap();
    let ingest_opt = IngestExternalFileOptions::new();

    let default_options = db2.get_options();
    gen_sst_from_db(
        default_options,
        db2.cf_handle("default"),
        test_sstfile_str,
        &db2,
    );

    db.ingest_external_file(&ingest_opt, &[test_sstfile_str])
        .unwrap();
    check_kv(
        &db,
        db.cf_handle("default"),
        &[
            (b"key1", Some(b"value1")),
            (b"key2", Some(b"value2")),
            (b"key3", Some(b"value3")),
            (b"key4", Some(b"value4")),
            (b"key5", Some(b"value5")),
        ],
    );

    let after = gen_crc32_from_db(&db);
    assert_eq!(before, after);
}

#[test]
fn test_delete_range_case_4() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_case_4");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let db = DB::open(opts, path_str).unwrap();
    let samples_a = vec![
        (b"key1", b"value1"),
        (b"key2", b"value2"),
        (b"key3", b"value3"),
        (b"key4", b"value4"),
        (b"key5", b"value5"),
    ];
    for (k, v) in samples_a {
        db.put(k, v).unwrap();
        assert_eq!(v, &*db.get(k).unwrap().unwrap());
    }
    let before = gen_crc32_from_db(&db);

    db.delete_range(b"key4", b"key6").unwrap();

    check_kv(
        &db,
        db.cf_handle("default"),
        &[
            (b"key1", Some(b"value1")),
            (b"key2", Some(b"value2")),
            (b"key3", Some(b"value3")),
            (b"key4", None),
            (b"key5", None),
        ],
    );

    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_case_4_2");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let db2 = DB::open(opts, path_str).unwrap();

    let samples_b = vec![(b"key4", b"value4"), (b"key5", b"value5")];
    for (k, v) in samples_b {
        db2.put(k, v).unwrap();
        assert_eq!(v, &*db2.get(k).unwrap().unwrap());
    }

    let gen_path = tempdir_with_prefix("_rust_rocksdb_case_4_ingest_sst_gen");
    let test_sstfile = gen_path.path().join("test_sst_file");
    let test_sstfile_str = test_sstfile.to_str().unwrap();
    let ingest_opt = IngestExternalFileOptions::new();

    let default_options = db2.get_options();
    gen_sst_from_db(
        default_options,
        db2.cf_handle("default"),
        test_sstfile_str,
        &db2,
    );

    db.ingest_external_file(&ingest_opt, &[test_sstfile_str])
        .unwrap();
    check_kv(
        &db,
        db.cf_handle("default"),
        &[
            (b"key1", Some(b"value1")),
            (b"key2", Some(b"value2")),
            (b"key3", Some(b"value3")),
            (b"key4", Some(b"value4")),
            (b"key5", Some(b"value5")),
        ],
    );

    let after = gen_crc32_from_db(&db);
    assert_eq!(before, after);
}

#[test]
fn test_delete_range_case_5() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_case_5");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let db = DB::open(opts, path_str).unwrap();
    let samples_a = vec![
        (b"key1", b"value1"),
        (b"key2", b"value2"),
        (b"key3", b"value3"),
        (b"key4", b"value4"),
        (b"key5", b"value5"),
    ];
    for (k, v) in samples_a {
        db.put(k, v).unwrap();
        assert_eq!(v, &*db.get(k).unwrap().unwrap());
    }

    db.delete_range(b"key1", b"key6").unwrap();

    check_kv(
        &db,
        db.cf_handle("default"),
        &[
            (b"key1", None),
            (b"key2", None),
            (b"key3", None),
            (b"key4", None),
            (b"key5", None),
        ],
    );

    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_case_5_2");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let db2 = DB::open(opts, path_str).unwrap();

    let samples_b = vec![(b"key4", b"value4"), (b"key5", b"value5")];
    for (k, v) in samples_b {
        db2.put(k, v).unwrap();
        assert_eq!(v, &*db2.get(k).unwrap().unwrap());
    }
    let before = gen_crc32_from_db(&db2);

    let gen_path = tempdir_with_prefix("_rust_rocksdb_case_5_ingest_sst_gen");
    let test_sstfile = gen_path.path().join("test_sst_file");
    let test_sstfile_str = test_sstfile.to_str().unwrap();
    let ingest_opt = IngestExternalFileOptions::new();

    let default_options = db2.get_options();
    gen_sst_from_db(
        default_options,
        db2.cf_handle("default"),
        test_sstfile_str,
        &db2,
    );

    db.ingest_external_file(&ingest_opt, &[test_sstfile_str])
        .unwrap();
    check_kv(
        &db,
        db.cf_handle("default"),
        &[(b"key4", Some(b"value4")), (b"key5", Some(b"value5"))],
    );

    let after = gen_crc32_from_db(&db);
    assert_eq!(before, after);
}

#[test]
fn test_delete_range_case_6() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_case_6");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let db = DB::open(opts, path_str).unwrap();
    let samples_a = vec![
        (b"key1", b"value1"),
        (b"key2", b"value2"),
        (b"key3", b"value3"),
        (b"key4", b"value4"),
        (b"key5", b"value5"),
    ];
    for (k, v) in samples_a {
        db.put(k, v).unwrap();
        assert_eq!(v, &*db.get(k).unwrap().unwrap());
    }

    let before = gen_crc32_from_db_in_range(&db, b"key4", b"key6");

    db.delete_range(b"key1", b"key4").unwrap();

    check_kv(
        &db,
        db.cf_handle("default"),
        &[
            (b"key1", None),
            (b"key2", None),
            (b"key3", None),
            (b"key4", Some(b"value4")),
            (b"key5", Some(b"value5")),
        ],
    );

    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_case_5_2");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let db2 = DB::open(opts, path_str).unwrap();

    let samples_b = vec![
        (b"key1", b"value1"),
        (b"key2", b"value2"),
        (b"key3", b"value3"),
    ];
    for (k, v) in samples_b {
        db2.put(k, v).unwrap();
        assert_eq!(v, &*db2.get(k).unwrap().unwrap());
    }

    let gen_path = tempdir_with_prefix("_rust_rocksdb_case_6_ingest_sst_gen");
    let test_sstfile = gen_path.path().join("test_sst_file");
    let test_sstfile_str = test_sstfile.to_str().unwrap();
    let ingest_opt = IngestExternalFileOptions::new();

    let default_options = db2.get_options();
    gen_sst_from_db(
        default_options,
        db2.cf_handle("default"),
        test_sstfile_str,
        &db2,
    );

    db.ingest_external_file(&ingest_opt, &[test_sstfile_str])
        .unwrap();
    check_kv(
        &db,
        db.cf_handle("default"),
        &[
            (b"key1", Some(b"value1")),
            (b"key2", Some(b"value2")),
            (b"key3", Some(b"value3")),
            (b"key4", Some(b"value4")),
            (b"key5", Some(b"value5")),
        ],
    );

    let after = gen_crc32_from_db_in_range(&db, b"key4", b"key6");
    assert_eq!(before, after);
}

#[test]
fn test_delete_range_compact() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_case_6");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let db = DB::open(opts, path_str).unwrap();
    let samples_a = vec![
        (b"key1", b"value1"),
        (b"key2", b"value2"),
        (b"key3", b"value3"),
        (b"key4", b"value4"),
        (b"key5", b"value5"),
    ];
    for (k, v) in samples_a {
        db.put(k, v).unwrap();
        assert_eq!(v, &*db.get(k).unwrap().unwrap());
    }

    let before = gen_crc32_from_db_in_range(&db, b"key4", b"key6");

    db.delete_range(b"key1", b"key4").unwrap();

    check_kv(
        &db,
        db.cf_handle("default"),
        &[
            (b"key1", None),
            (b"key2", None),
            (b"key3", None),
            (b"key4", Some(b"value4")),
            (b"key5", Some(b"value5")),
        ],
    );

    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_case_5_2");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let db2 = DB::open(opts, path_str).unwrap();

    let samples_b = vec![
        (b"key1", b"value1"),
        (b"key2", b"value2"),
        (b"key3", b"value3"),
    ];
    for (k, v) in samples_b {
        db2.put(k, v).unwrap();
        assert_eq!(v, &*db2.get(k).unwrap().unwrap());
    }

    let gen_path = tempdir_with_prefix("_rust_rocksdb_case_6_ingest_sst_gen");
    let test_sstfile = gen_path.path().join("test_sst_file");
    let test_sstfile_str = test_sstfile.to_str().unwrap();
    let ingest_opt = IngestExternalFileOptions::new();

    let default_options = db2.get_options();
    gen_sst_from_db(
        default_options,
        db2.cf_handle("default"),
        test_sstfile_str,
        &db2,
    );

    db.ingest_external_file(&ingest_opt, &[test_sstfile_str])
        .unwrap();
    check_kv(
        &db,
        db.cf_handle("default"),
        &[
            (b"key1", Some(b"value1")),
            (b"key2", Some(b"value2")),
            (b"key3", Some(b"value3")),
            (b"key4", Some(b"value4")),
            (b"key5", Some(b"value5")),
        ],
    );

    db.compact_range(None, None);
    let after = gen_crc32_from_db_in_range(&db, b"key4", b"key6");
    assert_eq!(before, after);
}

pub struct FixedSuffixSliceTransform {
    pub suffix_len: usize,
}

impl FixedSuffixSliceTransform {
    pub fn new(suffix_len: usize) -> FixedSuffixSliceTransform {
        FixedSuffixSliceTransform {
            suffix_len: suffix_len,
        }
    }
}

impl SliceTransform for FixedSuffixSliceTransform {
    fn transform<'a>(&mut self, key: &'a [u8]) -> &'a [u8] {
        let mid = key.len() - self.suffix_len;
        let (left, _) = key.split_at(mid);
        left
    }

    fn in_domain(&mut self, key: &[u8]) -> bool {
        key.len() >= self.suffix_len
    }

    fn in_range(&mut self, _: &[u8]) -> bool {
        true
    }
}

pub fn get_cf_handle<'a>(db: &'a DB, cf: &str) -> Result<&'a CFHandle, String> {
    db.cf_handle(cf)
        .ok_or_else(|| format!("cf {} not found.", cf))
}

#[test]
fn test_delete_range_prefix_bloom_case_1() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_prefix_bloom_case_1");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let mut cf_opts = ColumnFamilyOptions::new();
    // Prefix extractor(trim the timestamp at tail) for write cf.
    cf_opts
        .set_prefix_extractor(
            "FixedSuffixSliceTransform",
            Box::new(FixedSuffixSliceTransform::new(3)),
        )
        .unwrap_or_else(|err| panic!(format!("{:?}", err)));
    // Create prefix bloom filter for memtable.
    cf_opts.set_memtable_prefix_bloom_size_ratio(0.1 as f64);
    let cf = "default";
    let db = DB::open_cf(opts, path_str, vec![(cf, cf_opts)]).unwrap();

    let samples_a = vec![
        (b"keya11111", b"value1"),
        (b"keyb22222", b"value2"),
        (b"keyc33333", b"value3"),
        (b"keyd44444", b"value4"),
    ];
    let handle = get_cf_handle(&db, cf).unwrap();
    for (k, v) in samples_a {
        db.put_cf(handle, k, v).unwrap();
        assert_eq!(v, &*db.get(k).unwrap().unwrap());
    }
    let before = gen_crc32_from_db(&db);

    let gen_path = tempdir_with_prefix("_rust_rocksdb_case_prefix_bloom_1_ingest_sst_gen");
    let test_sstfile = gen_path.path().join("test_sst_file");
    let test_sstfile_str = test_sstfile.to_str().unwrap();
    let ingest_opt = IngestExternalFileOptions::new();

    let default_options = db.get_options();
    gen_sst_from_db(
        default_options,
        db.cf_handle("default"),
        test_sstfile_str,
        &db,
    );

    db.delete_range_cf(handle, b"keya11111", b"keye55555")
        .unwrap();
    check_kv(
        &db,
        db.cf_handle(cf),
        &[
            (b"keya11111", None),
            (b"keyb22222", None),
            (b"keyc33333", None),
            (b"keyd44444", None),
        ],
    );

    db.ingest_external_file_cf(handle, &ingest_opt, &[test_sstfile_str])
        .unwrap();
    check_kv(
        &db,
        db.cf_handle(cf),
        &[
            (b"keya11111", Some(b"value1")),
            (b"keyb22222", Some(b"value2")),
            (b"keyc33333", Some(b"value3")),
            (b"keyd44444", Some(b"value4")),
        ],
    );

    let after = gen_crc32_from_db(&db);
    assert_eq!(before, after);
}

#[test]
fn test_delete_range_prefix_bloom_case_2() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_prefix_bloom_case_2");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let mut cf_opts = ColumnFamilyOptions::new();
    // Prefix extractor(trim the timestamp at tail) for write cf.
    cf_opts
        .set_prefix_extractor(
            "FixedSuffixSliceTransform",
            Box::new(FixedSuffixSliceTransform::new(3)),
        )
        .unwrap_or_else(|err| panic!(format!("{:?}", err)));
    // Create prefix bloom filter for memtable.
    cf_opts.set_memtable_prefix_bloom_size_ratio(0.1 as f64);
    let cf = "default";
    let db = DB::open_cf(opts, path_str, vec![(cf, cf_opts)]).unwrap();
    let handle = get_cf_handle(&db, cf).unwrap();

    let samples_a = vec![
        (b"keya11111", b"value1"),
        (b"keyb22222", b"value2"),
        (b"keyc33333", b"value3"),
        (b"keyd44444", b"value4"),
    ];
    for (k, v) in samples_a {
        db.put_cf(handle, k, v).unwrap();
        assert_eq!(v, &*db.get(k).unwrap().unwrap());
    }
    let before = gen_crc32_from_db(&db);

    let gen_path = tempdir_with_prefix("_rust_rocksdb_case_prefix_bloom_1_ingest_sst_gen");
    let test_sstfile = gen_path.path().join("test_sst_file");
    let test_sstfile_str = test_sstfile.to_str().unwrap();
    let ingest_opt = IngestExternalFileOptions::new();

    let default_options = db.get_options();
    gen_sst_from_db(
        default_options,
        db.cf_handle("default"),
        test_sstfile_str,
        &db,
    );

    db.delete_range_cf(handle, b"keya11111", b"keye55555")
        .unwrap();
    check_kv(
        &db,
        db.cf_handle("default"),
        &[
            (b"keya11111", None),
            (b"keyb22222", None),
            (b"keyc33333", None),
            (b"keyd44444", None),
        ],
    );

    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_prefix_bloom_case_2_2");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let mut cf_opts = ColumnFamilyOptions::new();
    // Prefix extractor(trim the timestamp at tail) for write cf.
    cf_opts
        .set_prefix_extractor(
            "FixedSuffixSliceTransform",
            Box::new(FixedSuffixSliceTransform::new(3)),
        )
        .unwrap_or_else(|err| panic!(format!("{:?}", err)));
    // Create prefix bloom filter for memtable.
    cf_opts.set_memtable_prefix_bloom_size_ratio(0.1 as f64);
    let cf = "default";
    let db2 = DB::open_cf(opts, path_str, vec![(cf, cf_opts)]).unwrap();
    let handle2 = get_cf_handle(&db2, cf).unwrap();

    db2.ingest_external_file_cf(handle2, &ingest_opt, &[test_sstfile_str])
        .unwrap();
    check_kv(
        &db2,
        db2.cf_handle(cf),
        &[
            (b"keya11111", Some(b"value1")),
            (b"keyb22222", Some(b"value2")),
            (b"keyc33333", Some(b"value3")),
            (b"keyd44444", Some(b"value4")),
        ],
    );

    let after = gen_crc32_from_db(&db2);
    assert_eq!(before, after);
}

#[test]
fn test_delete_range_prefix_bloom_case_3() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_prefix_bloom_case_3");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let mut cf_opts = ColumnFamilyOptions::new();
    // Prefix extractor(trim the timestamp at tail) for write cf.
    cf_opts
        .set_prefix_extractor(
            "FixedSuffixSliceTransform",
            Box::new(FixedSuffixSliceTransform::new(3)),
        )
        .unwrap_or_else(|err| panic!(format!("{:?}", err)));
    // Create prefix bloom filter for memtable.
    cf_opts.set_memtable_prefix_bloom_size_ratio(0.1 as f64);
    let cf = "default";
    let db = DB::open_cf(opts, path_str, vec![(cf, cf_opts)]).unwrap();
    let handle = get_cf_handle(&db, cf).unwrap();
    let samples_a = vec![
        (b"keya11111", b"value1"),
        (b"keyb22222", b"value2"),
        (b"keyc33333", b"value3"),
        (b"keyd44444", b"value4"),
        (b"keye55555", b"value5"),
    ];
    for (k, v) in samples_a {
        db.put_cf(handle, k, v).unwrap();
        assert_eq!(v, &*db.get(k).unwrap().unwrap());
    }
    let before = gen_crc32_from_db(&db);

    db.delete_range_cf(handle, b"keyb22222", b"keyd44444")
        .unwrap();

    check_kv(
        &db,
        db.cf_handle(cf),
        &[
            (b"keya11111", Some(b"value1")),
            (b"keyb22222", None),
            (b"keyc33333", None),
            (b"keyd44444", Some(b"value4")),
            (b"keye55555", Some(b"value5")),
        ],
    );

    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_prefix_bloom_case_3_2");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let mut cf_opts = ColumnFamilyOptions::new();
    // Prefix extractor(trim the timestamp at tail) for write cf.
    cf_opts
        .set_prefix_extractor(
            "FixedSuffixSliceTransform",
            Box::new(FixedSuffixSliceTransform::new(3)),
        )
        .unwrap_or_else(|err| panic!(format!("{:?}", err)));
    // Create prefix bloom filter for memtable.
    cf_opts.set_memtable_prefix_bloom_size_ratio(0.1 as f64);
    let cf = "default";
    let db2 = DB::open_cf(opts, path_str, vec![(cf, cf_opts)]).unwrap();
    let handle2 = get_cf_handle(&db2, cf).unwrap();
    let samples_b = vec![(b"keyb22222", b"value2"), (b"keyc33333", b"value3")];
    for (k, v) in samples_b {
        db2.put_cf(handle2, k, v).unwrap();
        assert_eq!(v, &*db2.get(k).unwrap().unwrap());
    }

    let gen_path = tempdir_with_prefix("_rust_rocksdb_prefix_bloom_case_3_ingest_sst_gen");
    let test_sstfile = gen_path.path().join("test_sst_file");
    let test_sstfile_str = test_sstfile.to_str().unwrap();
    let ingest_opt = IngestExternalFileOptions::new();

    let default_options = db2.get_options();
    gen_sst_from_db(default_options, db2.cf_handle(cf), test_sstfile_str, &db2);

    db.ingest_external_file_cf(handle, &ingest_opt, &[test_sstfile_str])
        .unwrap();
    check_kv(
        &db,
        db.cf_handle(cf),
        &[
            (b"keya11111", Some(b"value1")),
            (b"keyb22222", Some(b"value2")),
            (b"keyc33333", Some(b"value3")),
            (b"keyd44444", Some(b"value4")),
            (b"keye55555", Some(b"value5")),
        ],
    );

    let after = gen_crc32_from_db(&db);
    assert_eq!(before, after);
}

#[test]
fn test_delete_range_prefix_bloom_case_4() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_prefix_bloom_case_4");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let mut cf_opts = ColumnFamilyOptions::new();
    // Prefix extractor(trim the timestamp at tail) for write cf.
    cf_opts
        .set_prefix_extractor(
            "FixedSuffixSliceTransform",
            Box::new(FixedSuffixSliceTransform::new(3)),
        )
        .unwrap_or_else(|err| panic!(format!("{:?}", err)));
    // Create prefix bloom filter for memtable.
    cf_opts.set_memtable_prefix_bloom_size_ratio(0.1 as f64);
    let cf = "default";
    let db = DB::open_cf(opts, path_str, vec![(cf, cf_opts)]).unwrap();
    let handle = get_cf_handle(&db, cf).unwrap();
    let samples_a = vec![
        (b"keya11111", b"value1"),
        (b"keyb22222", b"value2"),
        (b"keyc33333", b"value3"),
        (b"keyd44444", b"value4"),
        (b"keye55555", b"value5"),
    ];
    for (k, v) in samples_a {
        db.put_cf(handle, k, v).unwrap();
        assert_eq!(v, &*db.get(k).unwrap().unwrap());
    }
    let before = gen_crc32_from_db(&db);

    db.delete_range_cf(handle, b"keyd44444", b"keyf66666")
        .unwrap();

    check_kv(
        &db,
        db.cf_handle(cf),
        &[
            (b"keya11111", Some(b"value1")),
            (b"keyb22222", Some(b"value2")),
            (b"keyc33333", Some(b"value3")),
            (b"keyd44444", None),
            (b"keye55555", None),
        ],
    );

    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_prefix_bloom_case_4_2");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let mut cf_opts = ColumnFamilyOptions::new();
    // Prefix extractor(trim the timestamp at tail) for write cf.
    cf_opts
        .set_prefix_extractor(
            "FixedSuffixSliceTransform",
            Box::new(FixedSuffixSliceTransform::new(3)),
        )
        .unwrap_or_else(|err| panic!(format!("{:?}", err)));
    // Create prefix bloom filter for memtable.
    cf_opts.set_memtable_prefix_bloom_size_ratio(0.1 as f64);
    let cf = "default";
    let db2 = DB::open_cf(opts, path_str, vec![(cf, cf_opts)]).unwrap();
    let handle2 = get_cf_handle(&db2, cf).unwrap();

    let samples_b = vec![(b"keyd44444", b"value4"), (b"keye55555", b"value5")];
    for (k, v) in samples_b {
        db2.put_cf(handle2, k, v).unwrap();
        assert_eq!(v, &*db2.get(k).unwrap().unwrap());
    }

    let gen_path = tempdir_with_prefix("_rust_rocksdb_case_prefix_bloom_4_ingest_sst_gen");
    let test_sstfile = gen_path.path().join("test_sst_file");
    let test_sstfile_str = test_sstfile.to_str().unwrap();
    let ingest_opt = IngestExternalFileOptions::new();

    let default_options = db2.get_options();
    gen_sst_from_db(default_options, db2.cf_handle(cf), test_sstfile_str, &db2);

    db.ingest_external_file_cf(handle, &ingest_opt, &[test_sstfile_str])
        .unwrap();
    check_kv(
        &db,
        db.cf_handle(cf),
        &[
            (b"keya11111", Some(b"value1")),
            (b"keyb22222", Some(b"value2")),
            (b"keyc33333", Some(b"value3")),
            (b"keyd44444", Some(b"value4")),
            (b"keye55555", Some(b"value5")),
        ],
    );

    let after = gen_crc32_from_db(&db);
    assert_eq!(before, after);
}

#[test]
fn test_delete_range_prefix_bloom_case_5() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_prefix_bloom_case_5");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let mut cf_opts = ColumnFamilyOptions::new();
    // Prefix extractor(trim the timestamp at tail) for write cf.
    cf_opts
        .set_prefix_extractor(
            "FixedSuffixSliceTransform",
            Box::new(FixedSuffixSliceTransform::new(3)),
        )
        .unwrap_or_else(|err| panic!(format!("{:?}", err)));
    // Create prefix bloom filter for memtable.
    cf_opts.set_memtable_prefix_bloom_size_ratio(0.1 as f64);
    let cf = "default";
    let db = DB::open_cf(opts, path_str, vec![(cf, cf_opts)]).unwrap();
    let handle = get_cf_handle(&db, cf).unwrap();
    let samples_a = vec![
        (b"keya11111", b"value1"),
        (b"keyb22222", b"value2"),
        (b"keyc33333", b"value3"),
        (b"keyd44444", b"value4"),
        (b"keye55555", b"value5"),
    ];
    for (k, v) in samples_a {
        db.put_cf(handle, k, v).unwrap();
        assert_eq!(v, &*db.get(k).unwrap().unwrap());
    }

    db.delete_range(b"keya11111", b"keyf66666").unwrap();

    check_kv(
        &db,
        db.cf_handle(cf),
        &[
            (b"keya11111", None),
            (b"keyb22222", None),
            (b"keyc33333", None),
            (b"keyd44444", None),
            (b"keye55555", None),
        ],
    );

    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_prefix_bloom_case_5_2");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let mut cf_opts = ColumnFamilyOptions::new();
    // Prefix extractor(trim the timestamp at tail) for write cf.
    cf_opts
        .set_prefix_extractor(
            "FixedSuffixSliceTransform",
            Box::new(FixedSuffixSliceTransform::new(3)),
        )
        .unwrap_or_else(|err| panic!(format!("{:?}", err)));
    // Create prefix bloom filter for memtable.
    cf_opts.set_memtable_prefix_bloom_size_ratio(0.1 as f64);
    let db2 = DB::open_cf(opts, path_str, vec![(cf, cf_opts)]).unwrap();
    let handle2 = get_cf_handle(&db2, cf).unwrap();

    let samples_b = vec![(b"keyd44444", b"value4"), (b"keye55555", b"value5")];
    for (k, v) in samples_b {
        db2.put_cf(handle2, k, v).unwrap();
        assert_eq!(v, &*db2.get(k).unwrap().unwrap());
    }
    let before = gen_crc32_from_db(&db2);

    let gen_path = tempdir_with_prefix("_rust_rocksdb_case_prefix_bloom_5_ingest_sst_gen");
    let test_sstfile = gen_path.path().join("test_sst_file");
    let test_sstfile_str = test_sstfile.to_str().unwrap();
    let ingest_opt = IngestExternalFileOptions::new();

    let default_options = db2.get_options();
    gen_sst_from_db(default_options, db2.cf_handle(cf), test_sstfile_str, &db2);

    db.ingest_external_file_cf(handle, &ingest_opt, &[test_sstfile_str])
        .unwrap();
    check_kv(
        &db,
        db.cf_handle(cf),
        &[
            (b"keyd44444", Some(b"value4")),
            (b"keye55555", Some(b"value5")),
        ],
    );

    let after = gen_crc32_from_db(&db);
    assert_eq!(before, after);
}

#[test]
fn test_delete_range_prefix_bloom_case_6() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_prefix_bloom_case_6");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let mut cf_opts = ColumnFamilyOptions::new();
    // Prefix extractor(trim the timestamp at tail) for write cf.
    cf_opts
        .set_prefix_extractor(
            "FixedSuffixSliceTransform",
            Box::new(FixedSuffixSliceTransform::new(3)),
        )
        .unwrap_or_else(|err| panic!(format!("{:?}", err)));
    // Create prefix bloom filter for memtable.
    cf_opts.set_memtable_prefix_bloom_size_ratio(0.1 as f64);
    let cf = "default";
    let db = DB::open_cf(opts, path_str, vec![(cf, cf_opts)]).unwrap();
    let handle = get_cf_handle(&db, cf).unwrap();
    let samples_a = vec![
        (b"keya11111", b"value1"),
        (b"keyb22222", b"value2"),
        (b"keyc33333", b"value3"),
        (b"keyd44444", b"value4"),
        (b"keye55555", b"value5"),
    ];
    for (k, v) in samples_a {
        db.put_cf(handle, k, v).unwrap();
        assert_eq!(v, &*db.get(k).unwrap().unwrap());
    }

    let before = gen_crc32_from_db_in_range(&db, b"key4", b"key6");

    db.delete_range(b"keya11111", b"keyd44444").unwrap();

    check_kv(
        &db,
        db.cf_handle("default"),
        &[
            (b"keya11111", None),
            (b"keyb22222", None),
            (b"keyc33333", None),
            (b"keyd44444", Some(b"value4")),
            (b"keye55555", Some(b"value5")),
        ],
    );

    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_prefix_bloom_case_6_2");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let mut cf_opts = ColumnFamilyOptions::new();
    // Prefix extractor(trim the timestamp at tail) for write cf.
    cf_opts
        .set_prefix_extractor(
            "FixedSuffixSliceTransform",
            Box::new(FixedSuffixSliceTransform::new(3)),
        )
        .unwrap_or_else(|err| panic!(format!("{:?}", err)));
    // Create prefix bloom filter for memtable.
    cf_opts.set_memtable_prefix_bloom_size_ratio(0.1 as f64);
    let db2 = DB::open_cf(opts, path_str, vec![(cf, cf_opts)]).unwrap();
    let handle2 = get_cf_handle(&db2, cf).unwrap();

    let samples_b = vec![
        (b"keya11111", b"value1"),
        (b"keyb22222", b"value2"),
        (b"keyc33333", b"value3"),
    ];
    for (k, v) in samples_b {
        db2.put_cf(handle2, k, v).unwrap();
        assert_eq!(v, &*db2.get(k).unwrap().unwrap());
    }

    let gen_path = tempdir_with_prefix("_rust_rocksdb_case_6_ingest_sst_gen");
    let test_sstfile = gen_path.path().join("test_sst_file");
    let test_sstfile_str = test_sstfile.to_str().unwrap();
    let ingest_opt = IngestExternalFileOptions::new();

    let default_options = db2.get_options();
    gen_sst_from_db(default_options, db2.cf_handle(cf), test_sstfile_str, &db2);

    db.ingest_external_file_cf(handle, &ingest_opt, &[test_sstfile_str])
        .unwrap();
    check_kv(
        &db,
        db.cf_handle(cf),
        &[
            (b"keya11111", Some(b"value1")),
            (b"keyb22222", Some(b"value2")),
            (b"keyc33333", Some(b"value3")),
            (b"keyd44444", Some(b"value4")),
            (b"keye55555", Some(b"value5")),
        ],
    );

    let after = gen_crc32_from_db_in_range(&db, b"key4", b"key6");
    assert_eq!(before, after);
}

fn check_kv(db: &DB, cf: Option<&CFHandle>, data: &[(&[u8], Option<&[u8]>)]) {
    for &(k, v) in data {
        if cf.is_some() {
            if v.is_none() {
                assert!(db.get_cf(cf.unwrap(), k).unwrap().is_none());
            } else {
                assert_eq!(db.get_cf(cf.unwrap(), k).unwrap().unwrap(), v.unwrap());
            }
        } else {
            if v.is_none() {
                assert!(db.get(k).unwrap().is_none());
            } else {
                assert_eq!(db.get(k).unwrap().unwrap(), v.unwrap());
            }
        }
    }
}

#[test]
fn test_delete_range_prefix_bloom_compact_case() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_prefix_bloom_case_6");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let mut cf_opts = ColumnFamilyOptions::new();
    // Prefix extractor(trim the timestamp at tail) for write cf.
    cf_opts
        .set_prefix_extractor(
            "FixedSuffixSliceTransform",
            Box::new(FixedSuffixSliceTransform::new(3)),
        )
        .unwrap_or_else(|err| panic!(format!("{:?}", err)));
    // Create prefix bloom filter for memtable.
    cf_opts.set_memtable_prefix_bloom_size_ratio(0.1 as f64);
    let cf = "default";
    let db = DB::open_cf(opts, path_str, vec![(cf, cf_opts)]).unwrap();
    let handle = get_cf_handle(&db, cf).unwrap();
    let samples_a = vec![
        (b"keya11111", b"value1"),
        (b"keyb22222", b"value2"),
        (b"keyc33333", b"value3"),
        (b"keyd44444", b"value4"),
        (b"keye55555", b"value5"),
    ];
    for (k, v) in samples_a {
        db.put_cf(handle, k, v).unwrap();
        assert_eq!(v, &*db.get(k).unwrap().unwrap());
    }

    let before = gen_crc32_from_db_in_range(&db, b"keyd44444", b"keyf66666");

    db.delete_range(b"keya11111", b"keyd44444").unwrap();

    check_kv(
        &db,
        db.cf_handle("default"),
        &[
            (b"keya11111", None),
            (b"keyb22222", None),
            (b"keyc33333", None),
            (b"keyd44444", Some(b"value4")),
            (b"keye55555", Some(b"value5")),
        ],
    );

    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_prefix_bloom_case_6_2");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let mut cf_opts = ColumnFamilyOptions::new();
    // Prefix extractor(trim the timestamp at tail) for write cf.
    cf_opts
        .set_prefix_extractor(
            "FixedSuffixSliceTransform",
            Box::new(FixedSuffixSliceTransform::new(3)),
        )
        .unwrap_or_else(|err| panic!(format!("{:?}", err)));
    // Create prefix bloom filter for memtable.
    cf_opts.set_memtable_prefix_bloom_size_ratio(0.1 as f64);
    let db2 = DB::open_cf(opts, path_str, vec![(cf, cf_opts)]).unwrap();
    let handle2 = get_cf_handle(&db2, cf).unwrap();

    let samples_b = vec![
        (b"keya11111", b"value1"),
        (b"keyb22222", b"value2"),
        (b"keyc33333", b"value3"),
    ];
    for (k, v) in samples_b {
        db2.put_cf(handle2, k, v).unwrap();
        assert_eq!(v, &*db2.get(k).unwrap().unwrap());
    }

    let gen_path = tempdir_with_prefix("_rust_rocksdb_case_6_ingest_sst_gen");
    let test_sstfile = gen_path.path().join("test_sst_file");
    let test_sstfile_str = test_sstfile.to_str().unwrap();
    let ingest_opt = IngestExternalFileOptions::new();

    let default_options = db2.get_options();
    gen_sst_from_db(default_options, db2.cf_handle(cf), test_sstfile_str, &db2);

    db.ingest_external_file_cf(handle, &ingest_opt, &[test_sstfile_str])
        .unwrap();
    db.compact_range_cf(handle, None, None);
    check_kv(
        &db,
        db.cf_handle(cf),
        &[
            (b"keya11111", Some(b"value1")),
            (b"keyb22222", Some(b"value2")),
            (b"keyc33333", Some(b"value3")),
            (b"keyd44444", Some(b"value4")),
            (b"keye55555", Some(b"value5")),
        ],
    );

    let after = gen_crc32_from_db_in_range(&db, b"keyd44444", b"keyf66666");
    assert_eq!(before, after);
}

#[test]
fn test_delete_range() {
    // Test `DB::delete_range()`
    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range");
    let db = DB::open_default(path.path().to_str().unwrap()).unwrap();

    // Prepare some data.
    let prepare_data = || {
        db.put(b"a", b"v1").unwrap();
        let a = db.get(b"a");
        assert_eq!(a.unwrap().unwrap(), b"v1");
        db.put(b"b", b"v2").unwrap();
        let b = db.get(b"b");
        assert_eq!(b.unwrap().unwrap(), b"v2");
        db.put(b"c", b"v3").unwrap();
        let c = db.get(b"c");
        assert_eq!(c.unwrap().unwrap(), b"v3");
    };
    prepare_data();

    // Ensure delete range interface works to delete the specified range `[b"a", b"c")`.
    db.delete_range(b"a", b"c").unwrap();

    let check_data = || {
        assert!(db.get(b"a").unwrap().is_none());
        assert!(db.get(b"b").unwrap().is_none());
        let c = db.get(b"c");
        assert_eq!(c.unwrap().unwrap(), b"v3");
    };
    check_data();

    // Test `DB::delete_range_cf()`
    prepare_data();
    let cf_handle = db.cf_handle("default").unwrap();
    db.delete_range_cf(cf_handle, b"a", b"c").unwrap();
    check_data();

    // Test `WriteBatch::delete_range()`
    prepare_data();
    let batch = WriteBatch::new();
    batch.delete_range(b"a", b"c").unwrap();
    assert!(db.write(&batch).is_ok());
    check_data();

    // Test `WriteBatch::delete_range_cf()`
    prepare_data();
    let batch = WriteBatch::new();
    batch.delete_range_cf(cf_handle, b"a", b"c").unwrap();
    assert!(db.write(&batch).is_ok());
    check_data();
}

#[test]
fn test_delete_range_sst_files() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_sst_files");
    let db = DB::open_default(path.path().to_str().unwrap()).unwrap();
    let samples_a = vec![
        (b"key1", b"value1"),
        (b"key2", b"value2"),
        (b"key3", b"value3"),
        (b"key4", b"value4"),
    ];
    for (k, v) in samples_a {
        db.put(k, v).unwrap();
        assert_eq!(v, &*db.get(k).unwrap().unwrap());
    }
    db.flush(true).unwrap();

    let samples_b = vec![
        (b"key3", b"value5"),
        (b"key6", b"value6"),
        (b"key7", b"value7"),
        (b"key8", b"value8"),
    ];
    for (k, v) in samples_b {
        db.put(k, v).unwrap();
        assert_eq!(v, &*db.get(k).unwrap().unwrap());
    }
    db.flush(true).unwrap();
    assert_eq!(db.get(b"key3").unwrap().unwrap(), b"value5");

    db.delete_range(b"key1", b"key1").unwrap();
    db.delete_range(b"key2", b"key7").unwrap();
    check_kv(
        &db,
        None,
        &[
            (b"key1", Some(b"value1")),
            (b"key2", None),
            (b"key3", None),
            (b"key4", None),
            (b"key5", None),
            (b"key6", None),
            (b"key7", Some(b"value7")),
            (b"key8", Some(b"value8")),
        ],
    );

    db.delete_range(b"key1", b"key8").unwrap();
    check_kv(
        &db,
        None,
        &[
            (b"key1", None),
            (b"key2", None),
            (b"key3", None),
            (b"key4", None),
            (b"key5", None),
            (b"key6", None),
            (b"key7", None),
            (b"key8", Some(b"value8")),
        ],
    );
}

#[test]
fn test_delete_range_ingest_file() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_delete_range_ingest_file");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let mut db = DB::open(opts, path_str).unwrap();
    let gen_path = tempdir_with_prefix("_rust_rocksdb_ingest_sst_gen");
    let test_sstfile = gen_path.path().join("test_sst_file");
    let test_sstfile_str = test_sstfile.to_str().unwrap();
    let ingest_opt = IngestExternalFileOptions::new();

    let default_options = db.get_options();
    gen_sst(default_options, db.cf_handle("default"), test_sstfile_str);

    db.ingest_external_file(&ingest_opt, &[test_sstfile_str])
        .unwrap();
    assert!(test_sstfile.exists());
    check_kv(
        &db,
        db.cf_handle("default"),
        &[
            (b"key1", Some(b"value1")),
            (b"key2", Some(b"value2")),
            (b"key3", Some(b"value3")),
            (b"key4", Some(b"value4")),
        ],
    );

    db.delete_range(b"key1", b"key4").unwrap();
    check_kv(
        &db,
        db.cf_handle("default"),
        &[
            (b"key1", None),
            (b"key2", None),
            (b"key3", None),
            (b"key4", Some(b"value4")),
        ],
    );

    db.create_cf("cf1").unwrap();
    let handle = db.cf_handle("cf1").unwrap();
    gen_sst(ColumnFamilyOptions::new(), None, test_sstfile_str);

    db.ingest_external_file_cf(handle, &ingest_opt, &[test_sstfile_str])
        .unwrap();
    assert!(test_sstfile.exists());
    check_kv(
        &db,
        Some(handle),
        &[
            (b"key1", Some(b"value1")),
            (b"key2", Some(b"value2")),
            (b"key3", Some(b"value3")),
            (b"key4", Some(b"value4")),
        ],
    );

    let snap = db.snapshot();

    db.delete_range_cf(handle, b"key1", b"key3").unwrap();
    check_kv(
        &db,
        Some(handle),
        &[
            (b"key1", None),
            (b"key2", None),
            (b"key3", Some(b"value3")),
            (b"key4", Some(b"value4")),
        ],
    );
    assert_eq!(snap.get_cf(handle, b"key1").unwrap().unwrap(), b"value1");
    assert_eq!(snap.get_cf(handle, b"key4").unwrap().unwrap(), b"value4");
}
