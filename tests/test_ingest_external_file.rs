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
        writer.add(k, v).unwrap();
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
        Some(v) => for e in v {
            result.push(*e)
        },
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
    let path = TempDir::new("_rust_rocksdb_ingest_sst").expect("");
    let mut db = create_default_database(&path);
    let cf_opts = ColumnFamilyOptions::new();
    db.create_cf("cf1", cf_opts).unwrap();
    let handle = db.cf_handle("cf1").unwrap();
    let gen_path = TempDir::new("_rust_rocksdb_ingest_sst_gen").expect("");
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
    let path = TempDir::new("_rust_rocksdb_ingest_sst_new").expect("");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let mut cf_opts = ColumnFamilyOptions::new();
    cf_opts.add_merge_operator("merge operator", concat_merge);
    let db = DB::open_cf(opts, path_str, vec![("default", cf_opts)]).unwrap();
    let gen_path = TempDir::new("_rust_rocksdb_ingest_sst_gen_new").expect("");
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
    let path = TempDir::new("_rust_rocksdb_ingest_sst_new_cf").expect("");
    let mut db = create_default_database(&path);
    let gen_path = TempDir::new("_rust_rocksdb_ingest_sst_gen_new_cf").expect("");
    let test_sstfile = gen_path.path().join("test_sst_file_new_cf");
    let test_sstfile_str = test_sstfile.to_str().unwrap();
    let mut cf_opts = ColumnFamilyOptions::new();
    cf_opts.add_merge_operator("merge operator", concat_merge);
    db.create_cf("cf1", cf_opts).unwrap();
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
    iter.seek(SeekKey::Start);
    while iter.valid() {
        writer.add(iter.key(), iter.value()).unwrap();
        iter.next();
    }
    writer.finish().unwrap();
}

fn create_default_database(path: &TempDir) -> DB {
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    DB::open(opts, path_str).unwrap()
}

fn create_cfs(db: &mut DB, cfs: &[&str]) {
    for cf in cfs {
        if *cf != "default" {
            let cf_opts = ColumnFamilyOptions::new();
            db.create_cf(cf, cf_opts).unwrap();
        }
    }
}

#[test]
fn test_ingest_simulate_real_world() {
    const ALL_CFS: [&str; 3] = ["lock", "write", "default"];
    let path = TempDir::new("_rust_rocksdb_ingest_real_world_1").expect("");
    let mut db = create_default_database(&path);
    let gen_path = TempDir::new("_rust_rocksdb_ingest_real_world_new_cf").expect("");
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

    let path2 = TempDir::new("_rust_rocksdb_ingest_real_world_2").expect("");
    let mut db2 = create_default_database(&path2);
    for cf in &ALL_CFS {
        if *cf != "default" {
            let cf_opts = ColumnFamilyOptions::new();
            db2.create_cf(cf, cf_opts).unwrap();
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
        ).unwrap();
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
        ).unwrap();
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
