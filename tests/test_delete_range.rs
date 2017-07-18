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

use rocksdb::*;
use std::fs;
use tempdir::TempDir;

fn gen_sst(opt: Options, cf: Option<&CFHandle>, path: &str) {
    let _ = fs::remove_file(path);
    let env_opt = EnvOptions::new();
    let mut writer = if cf.is_some() {
        SstFileWriter::new_cf(env_opt, opt, cf.unwrap())
    } else {
        SstFileWriter::new(env_opt, opt)
    };
    writer.open(path).unwrap();
    writer.add(b"key1", b"value1").unwrap();
    writer.add(b"key2", b"value2").unwrap();
    writer.add(b"key3", b"value3").unwrap();
    writer.add(b"key4", b"value4").unwrap();
    writer.finish().unwrap();
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
fn test_delete_range() {
    // Test `DB::delete_range()`
    let path = TempDir::new("_rust_rocksdb_test_delete_range").expect("");
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
    assert!(db.write(batch).is_ok());
    check_data();

    // Test `WriteBatch::delete_range_cf()`
    prepare_data();
    let batch = WriteBatch::new();
    batch.delete_range_cf(cf_handle, b"a", b"c").unwrap();
    assert!(db.write(batch).is_ok());
    check_data();
}

#[test]
fn test_delete_range_sst_files() {
    let path = TempDir::new("_rust_rocksdb_test_delete_range_sst_files").expect("");
    let db = DB::open_default(path.path().to_str().unwrap()).unwrap();
    let samples_a = vec![(b"key1", b"value1"),
                         (b"key2", b"value2"),
                         (b"key3", b"value3"),
                         (b"key4", b"value4")];
    for (k, v) in samples_a {
        db.put(k, v).unwrap();
        assert_eq!(v, &*db.get(k).unwrap().unwrap());
    }
    db.flush(true).unwrap();

    let samples_b = vec![(b"key3", b"value5"),
                         (b"key6", b"value6"),
                         (b"key7", b"value7"),
                         (b"key8", b"value8")];
    for (k, v) in samples_b {
        db.put(k, v).unwrap();
        assert_eq!(v, &*db.get(k).unwrap().unwrap());
    }
    db.flush(true).unwrap();
    assert_eq!(db.get(b"key3").unwrap().unwrap(), b"value5");

    db.delete_range(b"key1", b"key1").unwrap();
    db.delete_range(b"key2", b"key7").unwrap();
    check_kv(&db,
             None,
             &[(b"key1", Some(b"value1")),
               (b"key2", None),
               (b"key3", None),
               (b"key4", None),
               (b"key5", None),
               (b"key6", None),
               (b"key7", Some(b"value7")),
               (b"key8", Some(b"value8"))]);

    db.delete_range(b"key1", b"key8").unwrap();
    check_kv(&db,
             None,
             &[(b"key1", None),
               (b"key2", None),
               (b"key3", None),
               (b"key4", None),
               (b"key5", None),
               (b"key6", None),
               (b"key7", None),
               (b"key8", Some(b"value8"))]);
}


#[test]
fn test_delete_range_ingest_file() {
    let path = TempDir::new("_rust_rocksdb_test_delete_range_ingest_file").expect("");
    let path_str = path.path().to_str().unwrap();
    let mut opts = Options::new();
    opts.create_if_missing(true);
    let mut db = DB::open(opts, path_str).unwrap();
    let gen_path = TempDir::new("_rust_rocksdb_ingest_sst_gen").expect("");
    let test_sstfile = gen_path.path().join("test_sst_file");
    let test_sstfile_str = test_sstfile.to_str().unwrap();
    let ingest_opt = IngestExternalFileOptions::new();

    let default_options = db.get_options();
    gen_sst(default_options, db.cf_handle("default"), test_sstfile_str);

    db.ingest_external_file(&ingest_opt, &[test_sstfile_str])
        .unwrap();
    assert!(test_sstfile.exists());
    check_kv(&db,
             db.cf_handle("default"),
             &[(b"key1", Some(b"value1")),
               (b"key2", Some(b"value2")),
               (b"key3", Some(b"value3")),
               (b"key4", Some(b"value4"))]);

    db.delete_range(b"key1", b"key4").unwrap();
    check_kv(&db,
             db.cf_handle("default"),
             &[(b"key1", None), (b"key2", None), (b"key3", None), (b"key4", Some(b"value4"))]);

    let cf_opts = Options::new();
    db.create_cf("cf1", &cf_opts).unwrap();
    let handle = db.cf_handle("cf1").unwrap();
    gen_sst(cf_opts, None, test_sstfile_str);

    db.ingest_external_file_cf(handle, &ingest_opt, &[test_sstfile_str])
        .unwrap();
    assert!(test_sstfile.exists());
    check_kv(&db,
             Some(handle),
             &[(b"key1", Some(b"value1")),
               (b"key2", Some(b"value2")),
               (b"key3", Some(b"value3")),
               (b"key4", Some(b"value4"))]);

    let snap = db.snapshot();

    db.delete_range_cf(handle, b"key1", b"key3").unwrap();
    check_kv(&db,
             Some(handle),
             &[(b"key1", None),
               (b"key2", None),
               (b"key3", Some(b"value3")),
               (b"key4", Some(b"value4"))]);
    assert_eq!(snap.get_cf(handle, b"key1").unwrap().unwrap(), b"value1");
    assert_eq!(snap.get_cf(handle, b"key4").unwrap().unwrap(), b"value4");
}
