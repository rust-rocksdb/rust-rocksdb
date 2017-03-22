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

use rocksdb::{DB, Options, WriteOptions, SliceTransform};
use rocksdb::crocksdb_ffi::{DBStatisticsHistogramType as HistogramType,
                            DBStatisticsTickerType as TickerType};
use std::path::Path;
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

struct FixedPrefixTransform {
    pub prefix_len: usize,
}

impl SliceTransform for FixedPrefixTransform {
    fn transform<'a>(&mut self, key: &'a [u8]) -> &'a [u8] {
        &key[..self.prefix_len]
    }

    fn in_domain(&mut self, key: &[u8]) -> bool {
        key.len() >= self.prefix_len
    }
}

#[test]
fn test_memtable_insert_hint_prefix_extractor() {
    let path = TempDir::new("_rust_rocksdb_memtable_insert_hint_prefix_extractor").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);
    opts.set_memtable_insert_hint_prefix_extractor("FixedPrefixTransform",
                                                   Box::new(FixedPrefixTransform {
                                                       prefix_len: 2,
                                                   }))
        .unwrap();
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    let wopts = WriteOptions::new();

    db.put_opt(b"k0-1", b"a", &wopts).unwrap();
    db.put_opt(b"k0-2", b"b", &wopts).unwrap();
    db.put_opt(b"k0-3", b"c", &wopts).unwrap();
    assert_eq!(db.get(b"k0-1").unwrap().unwrap(), b"a");
    assert_eq!(db.get(b"k0-2").unwrap().unwrap(), b"b");
    assert_eq!(db.get(b"k0-3").unwrap().unwrap(), b"c");
}

#[test]
fn test_set_ratelimiter() {
    let path = TempDir::new("_rust_rocksdb_test_set_rate_limiter").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);
    // compaction and flush rate limited below 100MB/sec
    opts.set_ratelimiter(100 * 1024 * 1024);
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    drop(db);
}

#[test]
fn test_set_wal_opt() {
    let path = TempDir::new("_rust_rocksdb_test_set_wal_opt").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);
    opts.set_wal_ttl_seconds(86400);
    opts.set_wal_size_limit_mb(10);
    let wal_dir = TempDir::new("_rust_rocksdb_test_set_wal_dir").expect("");
    opts.set_wal_dir(wal_dir.path().to_str().unwrap());
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    drop(db);
}

#[test]
fn test_create_info_log() {
    let path = TempDir::new("_rust_rocksdb_test_create_info_log_opt").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);

    let info_dir = TempDir::new("_rust_rocksdb_test_info_log_dir").expect("");
    opts.create_info_log(info_dir.path().to_str().unwrap()).unwrap();
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();

    assert!(Path::new(info_dir.path().join("LOG").to_str().unwrap()).is_file());

    drop(db);
}
