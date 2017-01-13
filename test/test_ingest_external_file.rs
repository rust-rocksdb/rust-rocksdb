// Copyright 2016 PingCAP, Inc.
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
use tempdir::TempDir;

use std::fs;

fn gen_sst(opt: &Options, path: &str, data: &[(&[u8], &[u8])]) {
    let _ = fs::remove_file(path);
    let env_opt = EnvOptions::new();
    let mut writer = SstFileWriter::new(&env_opt, opt);
    writer.open(path).unwrap();
    for &(k, v) in data {
        writer.add(k, v).unwrap();
    }
    writer.finish().unwrap();
}

#[test]
fn test_ingest_external_file() {
    let path = TempDir::new("_rust_rocksdb_ingest_sst").expect("");
    let path_str = path.path().to_str().unwrap();

    let mut opts = Options::new();
    opts.create_if_missing(true);
    let mut db = DB::open(opts, path_str).unwrap();
    let cf_opts = Options::new();
    db.create_cf("cf1", &cf_opts).unwrap();
    let handle = db.cf_handle("cf1").unwrap();

    let gen_path = TempDir::new("_rust_rocksdb_ingest_sst_gen").expect("");
    let test_sstfile = gen_path.path().join("test_sst_file");
    let test_sstfile_str = test_sstfile.to_str().unwrap();

    gen_sst(db.get_options(), test_sstfile_str, &[(b"k1", b"v1"), (b"k2", b"v2")]);
    
    let mut ingest_opt = IngestExternalFileOptions::new();
    db.ingest_external_file(&ingest_opt, &[test_sstfile_str]).unwrap();
    assert!(test_sstfile.exists());
    assert_eq!(db.get(b"k1").unwrap().unwrap(), b"v1");
    assert_eq!(db.get(b"k2").unwrap().unwrap(), b"v2");

    gen_sst(&cf_opts, test_sstfile_str, &[(b"k1", b"v3"), (b"k2", b"v4")]);
    db.ingest_external_file_cf(handle, &ingest_opt, &[test_sstfile_str]).unwrap();
    assert_eq!(db.get_cf(handle, b"k1").unwrap().unwrap(), b"v3");
    assert_eq!(db.get_cf(handle, b"k2").unwrap().unwrap(), b"v4");

    let snap = db.snapshot();

    gen_sst(db.get_options(), test_sstfile_str, &[(b"k2", b"v5"), (b"k3", b"v6")]);
    ingest_opt = ingest_opt.move_files(true);
    db.ingest_external_file_cf(handle, &ingest_opt, &[test_sstfile_str]).unwrap();
    
    assert_eq!(db.get_cf(handle, b"k1").unwrap().unwrap(), b"v3");
    assert_eq!(db.get_cf(handle, b"k2").unwrap().unwrap(), b"v5");
    assert_eq!(db.get_cf(handle, b"k3").unwrap().unwrap(), b"v6");
    assert_eq!(snap.get_cf(handle, b"k1").unwrap().unwrap(), b"v3");
    assert_eq!(snap.get_cf(handle, b"k2").unwrap().unwrap(), b"v4");
    assert!(snap.get_cf(handle, b"k3").unwrap().is_none());
}