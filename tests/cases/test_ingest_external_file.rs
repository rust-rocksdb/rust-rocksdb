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
use std::io::{Read, Write};
use std::sync::Arc;

use rocksdb::*;

use super::tempdir_with_prefix;

pub fn gen_sst(
    opt: ColumnFamilyOptions,
    cf: Option<&CFHandle>,
    path: &str,
    data: &[(&[u8], &[u8])],
) {
    let _ = fs::remove_file(path);
    let env_opt = EnvOptions::new();
    let mut writer = if cf.is_some() {
        SstFileWriter::new_cf(env_opt, opt, cf.unwrap())
    } else {
        SstFileWriter::new(env_opt, opt)
    };
    writer.open(path).unwrap();
    for &(k, v) in data {
        writer.put(k, v).unwrap();
    }

    writer.finish().unwrap();
}

fn gen_sst_put(opt: ColumnFamilyOptions, cf: Option<&CFHandle>, path: &str) {
    let _ = fs::remove_file(path);
    let env_opt = EnvOptions::new();
    let mut writer = if cf.is_some() {
        SstFileWriter::new_cf(env_opt, opt, cf.unwrap())
    } else {
        SstFileWriter::new(env_opt, opt)
    };
    writer.open(path).unwrap();
    writer.put(b"k1", b"a").unwrap();
    writer.put(b"k2", b"b").unwrap();
    writer.put(b"k3", b"c").unwrap();
    writer.finish().unwrap();
}

fn gen_sst_merge(opt: ColumnFamilyOptions, cf: Option<&CFHandle>, path: &str) {
    let _ = fs::remove_file(path);
    let env_opt = EnvOptions::new();
    let mut writer = if cf.is_some() {
        SstFileWriter::new_cf(env_opt, opt, cf.unwrap())
    } else {
        SstFileWriter::new(env_opt, opt)
    };
    writer.open(path).unwrap();
    writer.merge(b"k3", b"d").unwrap();
    writer.finish().unwrap();
}

fn gen_sst_delete(opt: ColumnFamilyOptions, cf: Option<&CFHandle>, path: &str) {
    let _ = fs::remove_file(path);
    let env_opt = EnvOptions::new();
    let mut writer = if cf.is_some() {
        SstFileWriter::new_cf(env_opt, opt, cf.unwrap())
    } else {
        SstFileWriter::new(env_opt, opt)
    };
    writer.open(path).unwrap();
    writer.delete(b"k3").unwrap();
    writer.finish().unwrap();
}

fn concat_merge(_: &[u8], existing_val: Option<&[u8]>, operands: &mut MergeOperands) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::with_capacity(operands.size_hint().0);
    match existing_val {
        Some(v) => {
            for e in v {
                result.push(*e)
            }
        }
        None => (),
    }
    for op in operands {
        for e in op {
            result.push(*e);
        }
    }
    result
}

#[test]
fn test_ingest_external_file() {
    let path = tempdir_with_prefix("_rust_rocksdb_ingest_sst");
    let mut db = create_default_database(&path);
    db.create_cf("cf1").unwrap();
    let handle = db.cf_handle("cf1").unwrap();
    let gen_path = tempdir_with_prefix("_rust_rocksdb_ingest_sst_gen");
    let test_sstfile = gen_path.path().join("test_sst_file");
    let test_sstfile_str = test_sstfile.to_str().unwrap();
    let default_options = db.get_options();

    gen_sst(
        default_options,
        Some(db.cf_handle("default").unwrap()),
        test_sstfile_str,
        &[(b"k1", b"v1"), (b"k2", b"v2")],
    );
    let mut ingest_opt = IngestExternalFileOptions::new();
    db.ingest_external_file(&ingest_opt, &[test_sstfile_str])
        .unwrap();
    assert!(test_sstfile.exists());
    assert_eq!(db.get(b"k1").unwrap().unwrap(), b"v1");
    assert_eq!(db.get(b"k2").unwrap().unwrap(), b"v2");

    gen_sst(
        ColumnFamilyOptions::new(),
        None,
        test_sstfile_str,
        &[(b"k1", b"v3"), (b"k2", b"v4")],
    );
    db.ingest_external_file_cf(handle, &ingest_opt, &[test_sstfile_str])
        .unwrap();
    assert_eq!(db.get_cf(handle, b"k1").unwrap().unwrap(), b"v3");
    assert_eq!(db.get_cf(handle, b"k2").unwrap().unwrap(), b"v4");
    let snap = db.snapshot();

    gen_sst(
        ColumnFamilyOptions::new(),
        None,
        test_sstfile_str,
        &[(b"k2", b"v5"), (b"k3", b"v6")],
    );
    ingest_opt.move_files(true);
    db.ingest_external_file_cf(handle, &ingest_opt, &[test_sstfile_str])
        .unwrap();

    assert_eq!(db.get_cf(handle, b"k1").unwrap().unwrap(), b"v3");
    assert_eq!(db.get_cf(handle, b"k2").unwrap().unwrap(), b"v5");
    assert_eq!(db.get_cf(handle, b"k3").unwrap().unwrap(), b"v6");
    assert_eq!(snap.get_cf(handle, b"k1").unwrap().unwrap(), b"v3");
    assert_eq!(snap.get_cf(handle, b"k2").unwrap().unwrap(), b"v4");
    assert!(snap.get_cf(handle, b"k3").unwrap().is_none());
}

#[test]
fn test_ingest_external_file_new() {
    let path = tempdir_with_prefix("_rust_rocksdb_ingest_sst_new");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let mut cf_opts = ColumnFamilyOptions::new();
    cf_opts.add_merge_operator("merge operator", concat_merge);
    let db = DB::open_cf(opts, path_str, vec![("default", cf_opts)]).unwrap();
    let gen_path = tempdir_with_prefix("_rust_rocksdb_ingest_sst_gen_new");
    let test_sstfile = gen_path.path().join("test_sst_file_new");
    let test_sstfile_str = test_sstfile.to_str().unwrap();
    let default_options = db.get_options();

    gen_sst_put(
        default_options,
        Some(db.cf_handle("default").unwrap()),
        test_sstfile_str,
    );
    let mut ingest_opt = IngestExternalFileOptions::new();
    db.ingest_external_file(&ingest_opt, &[test_sstfile_str])
        .unwrap();

    assert!(test_sstfile.exists());
    assert_eq!(db.get(b"k1").unwrap().unwrap(), b"a");
    assert_eq!(db.get(b"k2").unwrap().unwrap(), b"b");
    assert_eq!(db.get(b"k3").unwrap().unwrap(), b"c");

    let snap = db.snapshot();
    let default_options = db.get_options();
    gen_sst_merge(
        default_options,
        Some(db.cf_handle("default").unwrap()),
        test_sstfile_str,
    );
    db.ingest_external_file(&ingest_opt, &[test_sstfile_str])
        .unwrap();

    assert_eq!(db.get(b"k1").unwrap().unwrap(), b"a");
    assert_eq!(db.get(b"k2").unwrap().unwrap(), b"b");
    assert_eq!(db.get(b"k3").unwrap().unwrap(), b"cd");

    let default_options = db.get_options();
    gen_sst_delete(
        default_options,
        Some(db.cf_handle("default").unwrap()),
        test_sstfile_str,
    );
    ingest_opt.move_files(true);
    db.ingest_external_file(&ingest_opt, &[test_sstfile_str])
        .unwrap();

    assert_eq!(db.get(b"k1").unwrap().unwrap(), b"a");
    assert_eq!(db.get(b"k2").unwrap().unwrap(), b"b");
    assert!(db.get(b"k3").unwrap().is_none());
    assert_eq!(snap.get(b"k1").unwrap().unwrap(), b"a");
    assert_eq!(snap.get(b"k2").unwrap().unwrap(), b"b");
    assert_eq!(snap.get(b"k3").unwrap().unwrap(), b"c");
}

#[test]
fn test_ingest_external_file_new_cf() {
    let path = tempdir_with_prefix("_rust_rocksdb_ingest_sst_new_cf");
    let mut db = create_default_database(&path);
    let gen_path = tempdir_with_prefix("_rust_rocksdb_ingest_sst_gen_new_cf");
    let test_sstfile = gen_path.path().join("test_sst_file_new_cf");
    let test_sstfile_str = test_sstfile.to_str().unwrap();
    let mut cf_opts = ColumnFamilyOptions::new();
    cf_opts.add_merge_operator("merge operator", concat_merge);
    db.create_cf(("cf1", cf_opts)).unwrap();
    let handle = db.cf_handle("cf1").unwrap();

    let mut ingest_opt = IngestExternalFileOptions::new();
    gen_sst_put(ColumnFamilyOptions::new(), None, test_sstfile_str);

    db.ingest_external_file_cf(handle, &ingest_opt, &[test_sstfile_str])
        .unwrap();
    assert!(test_sstfile.exists());
    assert_eq!(db.get_cf(handle, b"k1").unwrap().unwrap(), b"a");
    assert_eq!(db.get_cf(handle, b"k2").unwrap().unwrap(), b"b");
    assert_eq!(db.get_cf(handle, b"k3").unwrap().unwrap(), b"c");

    let snap = db.snapshot();
    ingest_opt.move_files(true);
    gen_sst_merge(ColumnFamilyOptions::new(), None, test_sstfile_str);
    db.ingest_external_file_cf(handle, &ingest_opt, &[test_sstfile_str])
        .unwrap();
    assert_eq!(db.get_cf(handle, b"k1").unwrap().unwrap(), b"a");
    assert_eq!(db.get_cf(handle, b"k2").unwrap().unwrap(), b"b");
    assert_eq!(db.get_cf(handle, b"k3").unwrap().unwrap(), b"cd");

    gen_sst_delete(ColumnFamilyOptions::new(), None, test_sstfile_str);
    db.ingest_external_file_cf(handle, &ingest_opt, &[test_sstfile_str])
        .unwrap();

    assert_eq!(db.get_cf(handle, b"k1").unwrap().unwrap(), b"a");
    assert_eq!(db.get_cf(handle, b"k2").unwrap().unwrap(), b"b");
    assert!(db.get_cf(handle, b"k3").unwrap().is_none());
    assert_eq!(snap.get_cf(handle, b"k1").unwrap().unwrap(), b"a");
    assert_eq!(snap.get_cf(handle, b"k2").unwrap().unwrap(), b"b");
    assert_eq!(snap.get_cf(handle, b"k3").unwrap().unwrap(), b"c");
}

fn check_kv(db: &DB, cf: Option<&CFHandle>, data: &[(&[u8], Option<&[u8]>)]) {
    for &(k, v) in data {
        let handle = cf.unwrap_or(db.cf_handle("default").unwrap());
        if v.is_none() {
            assert!(db.get_cf(handle, k).unwrap().is_none());
        } else {
            assert_eq!(db.get_cf(handle, k).unwrap().unwrap(), v.unwrap());
        }
    }
}

fn put_delete_and_generate_sst_cf(opt: ColumnFamilyOptions, db: &DB, cf: &CFHandle, path: &str) {
    db.put_cf(cf, b"k1", b"v1").unwrap();
    db.put_cf(cf, b"k2", b"v2").unwrap();
    db.put_cf(cf, b"k3", b"v3").unwrap();
    db.put_cf(cf, b"k4", b"v4").unwrap();
    db.delete_cf(cf, b"k1").unwrap();
    db.delete_cf(cf, b"k3").unwrap();
    gen_sst_from_cf(opt, db, cf, path);
}

fn gen_sst_from_cf(opt: ColumnFamilyOptions, db: &DB, cf: &CFHandle, path: &str) {
    let env_opt = EnvOptions::new();
    let mut writer = SstFileWriter::new_cf(env_opt, opt, cf);
    writer.open(path).unwrap();
    let mut iter = db.iter_cf(cf);
    iter.seek(SeekKey::Start).unwrap();
    while iter.valid().unwrap() {
        writer.put(iter.key(), iter.value()).unwrap();
        iter.next().unwrap();
    }
    let info = writer.finish().unwrap();
    assert_eq!(info.file_path().to_str().unwrap(), path);
    iter.seek(SeekKey::Start).unwrap();
    assert_eq!(info.smallest_key(), iter.key());
    iter.seek(SeekKey::End).unwrap();
    assert_eq!(info.largest_key(), iter.key());
    assert_eq!(info.sequence_number(), 0);
    assert!(info.file_size() > 0);
    assert!(info.num_entries() > 0);
}

fn create_default_database(path: &tempfile::TempDir) -> DB {
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    DB::open(opts, path_str).unwrap()
}

fn create_cfs(db: &mut DB, cfs: &[&str]) {
    for cf in cfs {
        if *cf != "default" {
            db.create_cf(*cf).unwrap();
        }
    }
}

#[test]
fn test_ingest_simulate_real_world() {
    const ALL_CFS: [&str; 3] = ["lock", "write", "default"];
    let path = tempdir_with_prefix("_rust_rocksdb_ingest_real_world_1");
    let mut db = create_default_database(&path);
    let gen_path = tempdir_with_prefix("_rust_rocksdb_ingest_real_world_new_cf");
    create_cfs(&mut db, &ALL_CFS);
    for cf in &ALL_CFS {
        let handle = db.cf_handle(cf).unwrap();
        let cf_opts = ColumnFamilyOptions::new();
        put_delete_and_generate_sst_cf(
            cf_opts,
            &db,
            &handle,
            gen_path.path().join(cf).to_str().unwrap(),
        );
    }

    let path2 = tempdir_with_prefix("_rust_rocksdb_ingest_real_world_2");
    let mut db2 = create_default_database(&path2);
    for cf in &ALL_CFS {
        if *cf != "default" {
            db2.create_cf(*cf).unwrap();
        }
    }
    for cf in &ALL_CFS {
        let handle = db2.cf_handle(cf).unwrap();
        let mut ingest_opt = IngestExternalFileOptions::new();
        ingest_opt.move_files(true);
        db2.ingest_external_file_cf(
            handle,
            &ingest_opt,
            &[gen_path.path().join(cf).to_str().unwrap()],
        )
        .unwrap();
        check_kv(
            &db,
            db.cf_handle(cf),
            &[
                (b"k1", None),
                (b"k2", Some(b"v2")),
                (b"k3", None),
                (b"k4", Some(b"v4")),
            ],
        );
        let cf_opts = ColumnFamilyOptions::new();
        gen_sst_from_cf(
            cf_opts,
            &db2,
            &handle,
            gen_path.path().join(cf).to_str().unwrap(),
        );
    }

    for cf in &ALL_CFS {
        let handle = db.cf_handle(cf).unwrap();
        let ingest_opt = IngestExternalFileOptions::new();
        db.ingest_external_file_cf(
            handle,
            &ingest_opt,
            &[gen_path.path().join(cf).to_str().unwrap()],
        )
        .unwrap();
        check_kv(
            &db,
            db.cf_handle(cf),
            &[
                (b"k1", None),
                (b"k2", Some(b"v2")),
                (b"k3", None),
                (b"k4", Some(b"v4")),
            ],
        );
    }
}

#[test]
fn test_mem_sst_file_writer() {
    let path = tempdir_with_prefix("_rust_mem_sst_file_writer");
    let db = create_default_database(&path);

    let env = Arc::new(Env::new_mem());
    let mut opts = db.get_options().clone();
    opts.set_env(env.clone());

    let mem_sst_path = path.path().join("mem_sst");
    let mem_sst_str = mem_sst_path.to_str().unwrap();
    gen_sst(
        opts,
        None,
        mem_sst_str,
        &[(b"k1", b"v1"), (b"k2", b"v2"), (b"k3", b"v3")],
    );
    // Check that the file is not on disk.
    assert!(!mem_sst_path.exists());

    let mut buf = Vec::new();
    let mut sst = env
        .new_sequential_file(mem_sst_str, EnvOptions::new())
        .unwrap();
    sst.read_to_end(&mut buf).unwrap();

    // Write the data to a temp file.
    let sst_path = path.path().join("temp_sst_path");
    fs::File::create(&sst_path)
        .unwrap()
        .write_all(&buf)
        .unwrap();
    // Ingest the temp file to check the test kvs.
    let ingest_opts = IngestExternalFileOptions::new();
    db.ingest_external_file(&ingest_opts, &[sst_path.to_str().unwrap()])
        .unwrap();
    check_kv(
        &db,
        None,
        &[
            (b"k1", Some(b"v1")),
            (b"k2", Some(b"v2")),
            (b"k3", Some(b"v3")),
        ],
    );

    assert!(env.file_exists(mem_sst_str).is_ok());
    assert!(env.delete_file(mem_sst_str).is_ok());
    assert!(env.file_exists(mem_sst_str).is_err());
}

#[test]
fn test_set_external_sst_file_global_seq_no() {
    let db_path = tempdir_with_prefix("_rust_rocksdb_set_external_sst_file_global_seq_no_db");
    let db = create_default_database(&db_path);
    let path = tempdir_with_prefix("_rust_rocksdb_set_external_sst_file_global_seq_no");
    let file = path.path().join("sst_file");
    let sstfile_str = file.to_str().unwrap();
    gen_sst(
        ColumnFamilyOptions::new(),
        Some(db.cf_handle("default").unwrap()),
        sstfile_str,
        &[(b"k1", b"v1"), (b"k2", b"v2")],
    );

    let handle = db.cf_handle("default").unwrap();
    let seq_no = 1;
    // varify change seq_no
    let r1 = set_external_sst_file_global_seq_no(&db, &handle, sstfile_str, seq_no);
    assert!(r1.unwrap() != seq_no);
    // varify that seq_no are equal
    let r2 = set_external_sst_file_global_seq_no(&db, &handle, sstfile_str, seq_no);
    assert!(r2.unwrap() == seq_no);

    // change seq_no back to 0 so that it can be ingested
    assert!(set_external_sst_file_global_seq_no(&db, &handle, sstfile_str, 0).is_ok());

    db.ingest_external_file(&IngestExternalFileOptions::new(), &[sstfile_str])
        .unwrap();
    check_kv(&db, None, &[(b"k1", Some(b"v1")), (b"k2", Some(b"v2"))]);
}

#[test]
fn test_ingest_external_file_optimized() {
    let path = tempdir_with_prefix("_rust_rocksdb_ingest_sst_optimized");
    let db = create_default_database(&path);
    let gen_path = tempdir_with_prefix("_rust_rocksdb_ingest_sst_gen_new_cf");
    let test_sstfile = gen_path.path().join("test_sst_file_optimized");
    let test_sstfile_str = test_sstfile.to_str().unwrap();
    let handle = db.cf_handle("default").unwrap();

    let ingest_opt = IngestExternalFileOptions::new();
    gen_sst_put(ColumnFamilyOptions::new(), None, test_sstfile_str);

    db.put_cf(handle, b"k0", b"k0").unwrap();

    // No overlap with the memtable.
    let has_flush = db
        .ingest_external_file_optimized(handle, &ingest_opt, &[test_sstfile_str])
        .unwrap();
    assert!(!has_flush);
    assert!(test_sstfile.exists());
    assert_eq!(db.get_cf(handle, b"k1").unwrap().unwrap(), b"a");
    assert_eq!(db.get_cf(handle, b"k2").unwrap().unwrap(), b"b");
    assert_eq!(db.get_cf(handle, b"k3").unwrap().unwrap(), b"c");

    db.put_cf(handle, b"k1", b"k1").unwrap();

    // Overlap with the memtable.
    let has_flush = db
        .ingest_external_file_optimized(handle, &ingest_opt, &[test_sstfile_str])
        .unwrap();
    assert!(has_flush);
    assert!(test_sstfile.exists());
    assert_eq!(db.get_cf(handle, b"k1").unwrap().unwrap(), b"a");
    assert_eq!(db.get_cf(handle, b"k2").unwrap().unwrap(), b"b");
    assert_eq!(db.get_cf(handle, b"k3").unwrap().unwrap(), b"c");
}

#[test]
fn test_read_sst() {
    let dir = tempdir_with_prefix("_rust_rocksdb_test_read_sst");
    let sst_path = dir.path().join("sst");
    let sst_path_str = sst_path.to_str().unwrap();
    gen_sst_put(ColumnFamilyOptions::new(), None, sst_path_str);

    let mut reader = SstFileReader::new(ColumnFamilyOptions::default());
    reader.open(sst_path_str).unwrap();
    reader.verify_checksum().unwrap();
    reader.read_table_properties(|props| {
        assert_eq!(props.num_entries(), 3);
    });
    let mut it = reader.iter();
    it.seek(SeekKey::Start).unwrap();
    assert_eq!(
        it.collect::<Vec<_>>(),
        vec![
            (b"k1".to_vec(), b"a".to_vec()),
            (b"k2".to_vec(), b"b".to_vec()),
            (b"k3".to_vec(), b"c".to_vec()),
        ]
    );
}

#[test]
fn test_read_invalid_sst() {
    let dir = tempdir_with_prefix("_rust_rocksdb_test_read_invalid_sst");
    let sst_path = dir.path().join("sst");
    let sst_path_str = sst_path.to_str().unwrap();
    gen_sst_put(ColumnFamilyOptions::new(), None, sst_path_str);

    // corrupt one byte.
    {
        use std::io::{Seek, SeekFrom};

        let mut f = fs::OpenOptions::new().write(true).open(&sst_path).unwrap();
        f.seek(SeekFrom::Start(9)).unwrap();
        f.write(b"!").unwrap();
    }

    let mut reader = SstFileReader::new(ColumnFamilyOptions::default());
    reader.open(sst_path_str).unwrap();
    let error_message = reader.verify_checksum().unwrap_err();
    assert!(error_message.contains("checksum mismatch"));
}
