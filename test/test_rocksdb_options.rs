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

use rocksdb::{DB, Options};
use rocksdb::crocksdb_ffi::{DBStatisticsHistogramType as HistogramType,
                            DBStatisticsTickerType as TickerType};
use tempdir::TempDir;


#[test]
fn test_set_num_levels() {
    let path = TempDir::new("_rust_rocksdb_test_set_num_levels").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);
    opts.set_num_levels(2);
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    drop(db);
}

#[test]
fn test_log_file_opt() {
    let path = TempDir::new("_rust_rocksdb_log_file_opt").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);
    opts.set_max_log_file_size(100 * 1024 * 1024);
    opts.set_keep_log_file_num(10);
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    drop(db);
}

#[test]
fn test_compaction_readahead_size() {
    let path = TempDir::new("_rust_rocksdb_compaction_readahead_size").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);
    opts.set_compaction_readahead_size(2048);
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    drop(db);
}

#[test]
fn test_set_max_manifest_file_size() {
    let mut opts = Options::new();
    let size = 20 * 1024 * 1024;
    opts.set_max_manifest_file_size(size)
}

#[test]
fn test_enable_statistics() {
    let mut opts = Options::new();
    opts.enable_statistics();
    opts.set_stats_dump_period_sec(60);
    assert!(opts.get_statistics().is_some());
    assert!(opts.get_statistics_histogram_string(HistogramType::DbSeekMicros).is_some());
    assert_eq!(opts.get_statistics_ticker_count(TickerType::BlockCacheMiss),
               0);
    assert_eq!(opts.get_and_reset_statistics_ticker_count(TickerType::BlockCacheMiss),
               0);
    assert_eq!(opts.get_statistics_ticker_count(TickerType::BlockCacheMiss),
               0);

    let opts = Options::new();
    assert!(opts.get_statistics().is_none());
}
