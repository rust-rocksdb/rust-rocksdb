// Copyright 2022 TiKV Project Authors. Licensed under Apache-2.0.

use super::tempdir_with_prefix;
use rocksdb::{DBOptions, TitanDBOptions, Writable, DB};
use std::path::PathBuf;

fn check_checkpint_basic(path_str: &str, opts: DBOptions) {
    let db = DB::open(opts.clone(), path_str).unwrap();
    db.put(b"k1", b"v1").unwrap();
    db.put(b"k2", b"v2").unwrap();
    db.put(b"k3", b"v3").unwrap();
    let mut checkpoint = db.new_checkpointer().unwrap();
    let checkpoint_path = PathBuf::from(path_str).join("snap_1");
    checkpoint
        .create_at(checkpoint_path.as_path(), None, 0)
        .unwrap();
    // put after checkpoint
    db.put(b"k4", b"v4").unwrap();
    drop(db);

    let snap = DB::open(opts, checkpoint_path.to_str().unwrap()).unwrap();
    assert_eq!(snap.get(b"k1").unwrap().unwrap(), b"v1");
    assert_eq!(snap.get(b"k2").unwrap().unwrap(), b"v2");
    assert_eq!(snap.get(b"k3").unwrap().unwrap(), b"v3");
    assert_eq!(snap.get(b"k4").unwrap().is_none(), true);
}

#[test]
fn test_checkpoint() {
    let path = tempdir_with_prefix("_test_checkpoint_basic_case");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    check_checkpint_basic(path_str, opts);
}

#[test]
fn test_checkpoint_with_titan() {
    let path = tempdir_with_prefix("_test_checkpoint_basic_case");
    let path_str = path.path().to_str().unwrap();
    let tdb_path = path.path().join("titandb");
    let mut tdb_opts = TitanDBOptions::new();
    tdb_opts.set_min_blob_size(0);
    tdb_opts.set_dirname(tdb_path.to_str().unwrap());
    let mut opts = DBOptions::new();
    opts.set_titandb_options(&tdb_opts);
    opts.create_if_missing(true);
    check_checkpint_basic(path_str, opts);
}
