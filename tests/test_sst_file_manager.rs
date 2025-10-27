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

use rocksdb::{Env, Options, SstFileManager, DB};
use util::DBPath;

#[test]
fn test_sst_file_manager_basic() {
    let path = DBPath::new("test_sst_file_manager_basic");
    let env = Env::new().unwrap();
    let sst_file_manager = SstFileManager::new(&env);

    // Set maximum allowed space to 100MB
    sst_file_manager.set_max_allowed_space_usage(100 * 1024 * 1024);

    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.set_sst_file_manager(&sst_file_manager);

    let db = DB::open(&opts, &path).unwrap();

    // Write some data
    for i in 0..1000 {
        let key = format!("key_{}", i);
        let value = format!("value_{}", i);
        db.put(key.as_bytes(), value.as_bytes()).unwrap();
    }

    // Flush to create SST files
    db.flush().unwrap();

    // Get total size of SST files
    let total_size = sst_file_manager.get_total_size();
    assert!(total_size > 0, "Total SST size should be greater than 0");

    // Check if max allowed space is reached (should not be for this small dataset)
    assert!(
        !sst_file_manager.is_max_allowed_space_reached(),
        "Max allowed space should not be reached"
    );

    drop(db);
}

#[test]
fn test_sst_file_manager_delete_rate() {
    let path = DBPath::new("test_sst_file_manager_delete_rate");
    let env = Env::new().unwrap();
    let sst_file_manager = SstFileManager::new(&env);

    // Set delete rate to 1MB per second
    sst_file_manager.set_delete_rate_bytes_per_second(1024 * 1024);

    let delete_rate = sst_file_manager.get_delete_rate_bytes_per_second();
    assert_eq!(delete_rate, 1024 * 1024);

    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.set_sst_file_manager(&sst_file_manager);

    let _db = DB::open(&opts, &path).unwrap();
}

#[test]
fn test_sst_file_manager_compaction_buffer() {
    let path = DBPath::new("test_sst_file_manager_compaction_buffer");
    let env = Env::new().unwrap();
    let sst_file_manager = SstFileManager::new(&env);

    // Set compaction buffer size to 10MB
    sst_file_manager.set_compaction_buffer_size(10 * 1024 * 1024);

    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.set_sst_file_manager(&sst_file_manager);

    let _db = DB::open(&opts, &path).unwrap();
}

#[test]
fn test_sst_file_manager_trash_ratio() {
    let path = DBPath::new("test_sst_file_manager_trash_ratio");
    let env = Env::new().unwrap();
    let sst_file_manager = SstFileManager::new(&env);

    // Set max trash to DB ratio
    sst_file_manager.set_max_trash_db_ratio(0.25);

    let ratio = sst_file_manager.get_max_trash_db_ratio();
    assert!((ratio - 0.25).abs() < 0.01);

    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.set_sst_file_manager(&sst_file_manager);

    let _db = DB::open(&opts, &path).unwrap();
}
