// Copyright 2020 Tyler Neely
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod util;

use pretty_assertions::assert_eq;

use rocksdb::{checkpoint::Checkpoint, OptimisticTransactionDB, Options, DB};
use std::fs;
use util::DBPath;

#[test]
pub fn test_single_checkpoint() {
    const PATH_PREFIX: &str = "_rust_rocksdb_cp_single_";

    // Create DB with some data
    let db_path = DBPath::new(&format!("{PATH_PREFIX}db1"));

    let mut opts = Options::default();
    opts.create_if_missing(true);
    let db = DB::open(&opts, &db_path).unwrap();

    db.put(b"k1", b"v1").unwrap();
    db.put(b"k2", b"v2").unwrap();
    db.put(b"k3", b"v3").unwrap();
    db.put(b"k4", b"v4").unwrap();

    // Create checkpoint
    let cp1 = Checkpoint::new(&db).unwrap();
    let cp1_path = DBPath::new(&format!("{PATH_PREFIX}cp1"));
    cp1.create_checkpoint(&cp1_path).unwrap();

    // Verify checkpoint
    let cp = DB::open_default(&cp1_path).unwrap();

    assert_eq!(cp.get(b"k1").unwrap().unwrap(), b"v1");
    assert_eq!(cp.get(b"k2").unwrap().unwrap(), b"v2");
    assert_eq!(cp.get(b"k3").unwrap().unwrap(), b"v3");
    assert_eq!(cp.get(b"k4").unwrap().unwrap(), b"v4");
}

#[test]
pub fn test_multi_checkpoints() {
    const PATH_PREFIX: &str = "_rust_rocksdb_cp_multi_";

    // Create DB with some data
    let db_path = DBPath::new(&format!("{PATH_PREFIX}db1"));

    let mut opts = Options::default();
    opts.create_if_missing(true);
    let db = DB::open(&opts, &db_path).unwrap();

    db.put(b"k1", b"v1").unwrap();
    db.put(b"k2", b"v2").unwrap();
    db.put(b"k3", b"v3").unwrap();
    db.put(b"k4", b"v4").unwrap();

    // Create first checkpoint
    let cp1 = Checkpoint::new(&db).unwrap();
    let cp1_path = DBPath::new(&format!("{PATH_PREFIX}cp1"));
    cp1.create_checkpoint(&cp1_path).unwrap();

    // Verify checkpoint
    let cp = DB::open_default(&cp1_path).unwrap();

    assert_eq!(cp.get(b"k1").unwrap().unwrap(), b"v1");
    assert_eq!(cp.get(b"k2").unwrap().unwrap(), b"v2");
    assert_eq!(cp.get(b"k3").unwrap().unwrap(), b"v3");
    assert_eq!(cp.get(b"k4").unwrap().unwrap(), b"v4");

    // Change some existing keys
    db.put(b"k1", b"modified").unwrap();
    db.put(b"k2", b"changed").unwrap();

    // Add some new keys
    db.put(b"k5", b"v5").unwrap();
    db.put(b"k6", b"v6").unwrap();

    // Create another checkpoint
    let cp2 = Checkpoint::new(&db).unwrap();
    let cp2_path = DBPath::new(&format!("{PATH_PREFIX}cp2"));
    cp2.create_checkpoint(&cp2_path).unwrap();

    // Verify second checkpoint
    let cp = DB::open_default(&cp2_path).unwrap();

    assert_eq!(cp.get(b"k1").unwrap().unwrap(), b"modified");
    assert_eq!(cp.get(b"k2").unwrap().unwrap(), b"changed");
    assert_eq!(cp.get(b"k5").unwrap().unwrap(), b"v5");
    assert_eq!(cp.get(b"k6").unwrap().unwrap(), b"v6");
}

/// Test `create_checkpoint_with_log_size` with log_size_for_flush = 0.
/// A value of 0 forces RocksDB to flush memtables before creating the checkpoint,
/// ensuring all recent writes are included.
#[test]
pub fn test_checkpoint_with_log_size_zero_forces_flush() {
    const PATH_PREFIX: &str = "_rust_rocksdb_cp_log_size_zero_";

    let db_path = DBPath::new(&format!("{PATH_PREFIX}db"));

    let mut opts = Options::default();
    opts.create_if_missing(true);
    let db = DB::open(&opts, &db_path).unwrap();

    // Write some initial data and flush it explicitly to ensure we have
    // some materialized state in SST files
    db.put(b"flushed_key", b"flushed_value").unwrap();
    db.flush().unwrap();

    // Write additional data that will remain in the memtable (not flushed to SST)
    db.put(b"memtable_key", b"memtable_value").unwrap();

    // Create checkpoint with log_size_for_flush = 0 (forces flush)
    let cp = Checkpoint::new(&db).unwrap();
    let cp_path = DBPath::new(&format!("{PATH_PREFIX}cp"));
    cp.create_checkpoint_with_log_size(&cp_path, 0).unwrap();

    // Verify there is exactly one WAL file and it is empty (data was flushed to SST)
    let wal_files: Vec<_> = fs::read_dir((&cp_path).as_ref())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "log"))
        .collect();
    assert_eq!(
        wal_files.len(),
        1,
        "Checkpoint should contain exactly one WAL file"
    );
    let wal_metadata = wal_files[0].metadata().unwrap();
    assert_eq!(
        wal_metadata.len(),
        0,
        "WAL file should be empty when flush is forced"
    );

    // Verify checkpoint contains all data (flush was forced, so data is in SST files)
    let cp_db = DB::open_default(&cp_path).unwrap();

    assert_eq!(
        cp_db.get(b"flushed_key").unwrap().unwrap(),
        b"flushed_value"
    );
    assert_eq!(
        cp_db.get(b"memtable_key").unwrap().unwrap(),
        b"memtable_value"
    );
}

/// Test `create_checkpoint_with_log_size` with a large log_size_for_flush value.
/// A non-zero value means RocksDB skips flushing memtables if the WAL is smaller
/// than the threshold. However, the checkpoint still includes WAL files, so when
/// the checkpoint is opened, the WAL is replayed and memtable data becomes available.
#[test]
pub fn test_checkpoint_with_large_log_size_skips_flush() {
    const PATH_PREFIX: &str = "_rust_rocksdb_cp_log_size_large_";

    let db_path = DBPath::new(&format!("{PATH_PREFIX}db"));

    let mut opts = Options::default();
    opts.create_if_missing(true);
    let db = DB::open(&opts, &db_path).unwrap();

    // Write some initial data and flush it explicitly to ensure we have
    // some materialized state in SST files
    db.put(b"flushed_key", b"flushed_value").unwrap();
    db.flush().unwrap();

    // Write additional data that will remain in the memtable (not flushed to SST)
    db.put(b"memtable_key", b"memtable_value").unwrap();

    // Create checkpoint with a very large log_size_for_flush.
    // This tells RocksDB not to force a flush unless WAL exceeds this size.
    // Since we've written very little data, the WAL should be well under this
    // threshold, so no flush should be forced.
    let cp = Checkpoint::new(&db).unwrap();
    let cp_path = DBPath::new(&format!("{PATH_PREFIX}cp"));
    let large_log_size = u64::MAX;
    cp.create_checkpoint_with_log_size(&cp_path, large_log_size)
        .unwrap();

    // Verify there is exactly one WAL file and it is not empty
    let wal_files: Vec<_> = fs::read_dir((&cp_path).as_ref())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "log"))
        .collect();
    assert_eq!(
        wal_files.len(),
        1,
        "Checkpoint should contain exactly one WAL file"
    );
    let wal_metadata = wal_files[0].metadata().unwrap();
    assert!(wal_metadata.len() > 0, "WAL file should not be empty");

    // Verify the checkpoint can be opened and contains the flushed data
    let cp_db = DB::open_default(&cp_path).unwrap();

    // The flushed key should definitely be present (it was in an SST file)
    assert_eq!(
        cp_db.get(b"flushed_key").unwrap().unwrap(),
        b"flushed_value"
    );

    // The memtable_key IS present even though no flush was forced, because
    // the checkpoint includes WAL files. When the checkpoint DB is opened,
    // the WAL is replayed, restoring the memtable data.
    assert_eq!(
        cp_db.get(b"memtable_key").unwrap().unwrap(),
        b"memtable_value"
    );
}

/// Test `create_checkpoint_with_log_size` on OptimisticTransactionDB with log_size_for_flush = 0.
/// A value of 0 forces RocksDB to flush memtables before creating the checkpoint,
/// ensuring all recent writes are included.
#[test]
pub fn test_optimistic_transaction_db_checkpoint_with_log_size_zero_forces_flush() {
    const PATH_PREFIX: &str = "_rust_rocksdb_otxn_cp_log_size_zero_";

    let db_path = DBPath::new(&format!("{PATH_PREFIX}db"));

    let mut opts = Options::default();
    opts.create_if_missing(true);
    let db: OptimisticTransactionDB = OptimisticTransactionDB::open(&opts, &db_path).unwrap();

    // Write some initial data and flush it explicitly to ensure we have
    // some materialized state in SST files
    db.put(b"flushed_key", b"flushed_value").unwrap();
    db.flush().unwrap();

    // Write additional data that will remain in the memtable (not flushed to SST)
    db.put(b"memtable_key", b"memtable_value").unwrap();

    // Create checkpoint with log_size_for_flush = 0 (forces flush)
    let cp = Checkpoint::new(&db).unwrap();
    let cp_path = DBPath::new(&format!("{PATH_PREFIX}cp"));
    cp.create_checkpoint_with_log_size(&cp_path, 0).unwrap();

    // Verify there is exactly one WAL file and it is empty (data was flushed to SST)
    let wal_files: Vec<_> = fs::read_dir((&cp_path).as_ref())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "log"))
        .collect();
    assert_eq!(
        wal_files.len(),
        1,
        "Checkpoint should contain exactly one WAL file"
    );
    let wal_metadata = wal_files[0].metadata().unwrap();
    assert_eq!(
        wal_metadata.len(),
        0,
        "WAL file should be empty when flush is forced"
    );

    // Verify checkpoint contains all data (flush was forced, so data is in SST files)
    let cp_db: OptimisticTransactionDB = OptimisticTransactionDB::open_default(&cp_path).unwrap();

    assert_eq!(
        cp_db.get(b"flushed_key").unwrap().unwrap(),
        b"flushed_value"
    );
    assert_eq!(
        cp_db.get(b"memtable_key").unwrap().unwrap(),
        b"memtable_value"
    );
}

/// Test `create_checkpoint_with_log_size` on OptimisticTransactionDB with a large log_size_for_flush value.
/// A non-zero value means RocksDB skips flushing memtables if the WAL is smaller
/// than the threshold. However, the checkpoint still includes WAL files, so when
/// the checkpoint is opened, the WAL is replayed and memtable data becomes available.
#[test]
pub fn test_optimistic_transaction_db_checkpoint_with_large_log_size_skips_flush() {
    const PATH_PREFIX: &str = "_rust_rocksdb_otxn_cp_log_size_large_";

    let db_path = DBPath::new(&format!("{PATH_PREFIX}db"));

    let mut opts = Options::default();
    opts.create_if_missing(true);
    let db: OptimisticTransactionDB = OptimisticTransactionDB::open(&opts, &db_path).unwrap();

    // Write some initial data and flush it explicitly to ensure we have
    // some materialized state in SST files
    db.put(b"flushed_key", b"flushed_value").unwrap();
    db.flush().unwrap();

    // Write additional data that will remain in the memtable (not flushed to SST)
    db.put(b"memtable_key", b"memtable_value").unwrap();

    // Create checkpoint with a very large log_size_for_flush.
    // This tells RocksDB not to force a flush unless WAL exceeds this size.
    // Since we've written very little data, the WAL should be well under this
    // threshold, so no flush should be forced.
    let cp = Checkpoint::new(&db).unwrap();
    let cp_path = DBPath::new(&format!("{PATH_PREFIX}cp"));
    let large_log_size = u64::MAX;
    cp.create_checkpoint_with_log_size(&cp_path, large_log_size)
        .unwrap();

    // Verify there is exactly one WAL file and it is not empty
    let wal_files: Vec<_> = fs::read_dir((&cp_path).as_ref())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "log"))
        .collect();
    assert_eq!(
        wal_files.len(),
        1,
        "Checkpoint should contain exactly one WAL file"
    );
    let wal_metadata = wal_files[0].metadata().unwrap();
    assert!(wal_metadata.len() > 0, "WAL file should not be empty");

    // Verify the checkpoint can be opened and contains the flushed data
    let cp_db: OptimisticTransactionDB = OptimisticTransactionDB::open_default(&cp_path).unwrap();

    // The flushed key should definitely be present (it was in an SST file)
    assert_eq!(
        cp_db.get(b"flushed_key").unwrap().unwrap(),
        b"flushed_value"
    );

    // The memtable_key IS present even though no flush was forced, because
    // the checkpoint includes WAL files. When the checkpoint DB is opened,
    // the WAL is replayed, restoring the memtable data.
    assert_eq!(
        cp_db.get(b"memtable_key").unwrap().unwrap(),
        b"memtable_value"
    );
}

#[test]
fn test_checkpoint_outlive_db() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail/checkpoint_outlive_db.rs");
}
