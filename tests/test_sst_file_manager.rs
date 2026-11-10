// Copyright 2026 Arni Dagur
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

use std::sync::Barrier;
use std::thread;
use std::time::{Duration, Instant};

use pretty_assertions::assert_eq;

use rocksdb::{Env, Options, SstFileManager, DB};
use util::DBPath;

fn options_with(env: &Env, manager: &SstFileManager) -> Options {
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.set_env(env);
    opts.set_sst_file_manager(manager);
    opts
}

#[test]
fn sst_file_manager_tracks_the_sst_files_of_the_databases_it_is_attached_to() {
    let env = Env::new().unwrap();
    let manager = SstFileManager::new(&env);
    assert_eq!(manager.total_size(), 0);
    assert!(!manager.is_max_allowed_space_reached());

    let path_a = DBPath::new("_rust_rocksdb_sst_file_manager_a");
    let path_b = DBPath::new("_rust_rocksdb_sst_file_manager_b");
    let db_a = DB::open(&options_with(&env, &manager), &path_a).unwrap();
    let db_b = DB::open(&options_with(&env, &manager), &path_b).unwrap();

    db_a.put(b"k", vec![1u8; 64 << 10]).unwrap();
    db_a.flush().unwrap();
    let after_a = manager.total_size();
    assert!(
        after_a > 0,
        "the flushed SST file of the first database is tracked"
    );

    db_b.put(b"k", vec![2u8; 64 << 10]).unwrap();
    db_b.flush().unwrap();
    assert!(
        manager.total_size() > after_a,
        "one manager tracks several databases"
    );

    drop(db_a);
    drop(db_b);
}

#[test]
fn the_db_stops_on_the_space_cap_and_recovers_on_its_own_once_it_is_lifted() {
    let env = Env::new().unwrap();
    let manager = SstFileManager::new(&env);
    let path = DBPath::new("_rust_rocksdb_sst_file_manager_cap");
    let db = DB::open(&options_with(&env, &manager), &path).unwrap();

    db.put(b"k1", vec![1u8; 256 << 10]).unwrap();
    db.flush().unwrap();
    let used = manager.total_size();
    assert!(used > 0);

    // A cap below the current usage: the next flush fails with a space-limit error and
    // the DB stops taking writes (a hard background error).
    manager.set_max_allowed_space_usage(used / 2);
    assert!(manager.is_max_allowed_space_reached());
    assert!(manager.is_max_allowed_space_reached_including_compactions());
    db.put(b"k2", vec![2u8; 256 << 10]).unwrap();
    let refused = db.flush();
    assert!(
        refused.is_err(),
        "a flush beyond the cap is refused: {refused:?}"
    );
    let stopped = db.put(b"k3", b"v");
    assert!(stopped.is_err(), "writes stop after the error: {stopped:?}");
    assert_eq!(db.get(b"k1").unwrap().unwrap(), vec![1u8; 256 << 10]);

    // With the manager attached, RocksDB's error handler polls and resumes the DB on its
    // own once the cap is lifted (no reopen, no manual resume).
    manager.set_max_allowed_space_usage(0);
    assert!(!manager.is_max_allowed_space_reached());
    let deadline = Instant::now() + Duration::from_secs(60);
    let recovered = loop {
        match db.put(b"k3", b"v") {
            Ok(()) => break true,
            Err(_) if Instant::now() < deadline => thread::sleep(Duration::from_millis(200)),
            Err(e) => panic!("the DB did not recover after the cap was lifted: {e}"),
        }
    };
    assert!(recovered);
    assert_eq!(db.get(b"k2").unwrap().unwrap(), vec![2u8; 256 << 10]);
    assert_eq!(db.get(b"k3").unwrap().unwrap(), b"v");
    db.flush().unwrap();
}

#[test]
fn deletion_rate_and_trash_ratio_round_trip() {
    let env = Env::new().unwrap();
    let manager = SstFileManager::new(&env);
    assert_eq!(manager.delete_rate_bytes_per_second(), 0);
    manager.set_delete_rate_bytes_per_second(4 << 20);
    assert_eq!(manager.delete_rate_bytes_per_second(), 4 << 20);
    manager.set_max_trash_db_ratio(0.5);
    assert!((manager.max_trash_db_ratio() - 0.5).abs() < f64::EPSILON);
    assert_eq!(manager.total_trash_size(), 0);
    manager.set_compaction_buffer_size(1 << 20);
}

#[test]
fn concurrent_callers_can_enable_deletion_rate_limiting() {
    let env = Env::new().unwrap();
    // RocksDB starts its deletion thread on the first positive rate, so every round needs a
    // manager that has not seen one yet.
    for _ in 0..100 {
        let manager = SstFileManager::new(&env);
        let barrier = Barrier::new(8);
        thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| {
                    barrier.wait();
                    manager.set_delete_rate_bytes_per_second(4 << 20);
                });
            }
        });
        assert_eq!(manager.delete_rate_bytes_per_second(), 4 << 20);
    }
}

#[test]
fn the_options_keep_the_manager_alive_for_the_database() {
    let env = Env::new().unwrap();
    let path = DBPath::new("_rust_rocksdb_sst_file_manager_outlive");
    let opts = {
        let manager = SstFileManager::new(&env);
        options_with(&env, &manager)
        // `manager` is dropped here; the options hold the last reference.
    };
    let db = DB::open(&opts, &path).unwrap();
    drop(opts);
    db.put(b"k", b"v").unwrap();
    db.flush().unwrap();
    assert_eq!(db.get(b"k").unwrap().unwrap(), b"v");
}
