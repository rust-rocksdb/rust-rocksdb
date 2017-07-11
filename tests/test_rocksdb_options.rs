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

use rocksdb::{DB, Options, BlockBasedOptions, WriteOptions, SliceTransform, Writable,
              CompactOptions};
use rocksdb::crocksdb_ffi::{DBStatisticsHistogramType as HistogramType,
                            DBStatisticsTickerType as TickerType, DBInfoLogLevel as InfoLogLevel,
                            CompactionPriority, DBCompressionType};
use std::path::Path;
use std::thread;
use std::time::Duration;
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
    assert!(opts.get_statistics_histogram_string(HistogramType::DbSeekMicros)
        .is_some());
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
fn test_set_delayed_write_rate() {
    let path = TempDir::new("_rust_rocksdb_test_set_delayed_write_rate").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);
    opts.set_delayed_write_rate(2 * 1024 * 1024);
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    drop(db);
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

#[cfg(not(windows))]
#[test]
fn test_sync_wal() {
    let path = TempDir::new("_rust_rocksdb_test_sync_wal").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    db.put(b"key", b"value").unwrap();
    db.sync_wal().unwrap();
    drop(db);
}

#[test]
fn test_create_info_log() {
    let path = TempDir::new("_rust_rocksdb_test_create_info_log_opt").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);
    opts.set_info_log_level(InfoLogLevel::DBDebug);
    opts.set_log_file_time_to_roll(1);

    let info_dir = TempDir::new("_rust_rocksdb_test_info_log_dir").expect("");
    opts.create_info_log(info_dir.path().to_str().unwrap())
        .unwrap();

    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    assert!(Path::new(info_dir.path().join("LOG").to_str().unwrap()).is_file());

    thread::sleep(Duration::from_secs(2));

    for i in 0..200 {
        db.put(format!("k_{}", i).as_bytes(), b"v").unwrap();
        db.flush(true).unwrap();
    }

    drop(db);

    // The LOG must be rolled many times.
    let count = info_dir.path().read_dir().unwrap().count();
    assert!(count > 1);
}

#[test]
fn test_auto_roll_max_size_info_log() {
    let path = TempDir::new("_rust_rocksdb_test_max_size_info_log_opt").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);
    opts.set_max_log_file_size(10);

    let info_dir = TempDir::new("_rust_rocksdb_max_size_info_log_dir").expect("");
    opts.create_info_log(info_dir.path().to_str().unwrap())
        .unwrap();

    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    assert!(Path::new(info_dir.path().join("LOG").to_str().unwrap()).is_file());

    drop(db);

    // The LOG must be rolled many times.
    let count = info_dir.path().read_dir().unwrap().count();
    assert!(count > 1);
}

#[test]
fn test_set_pin_l0_filter_and_index_blocks_in_cache() {
    let path = TempDir::new("_rust_rocksdb_set_cache_and_index").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);
    let mut block_opts = BlockBasedOptions::new();
    block_opts.set_pin_l0_filter_and_index_blocks_in_cache(true);
    opts.set_block_based_table_factory(&block_opts);
    DB::open(opts, path.path().to_str().unwrap()).unwrap();
}
#[test]
fn test_pending_compaction_bytes_limit() {
    let path = TempDir::new("_rust_rocksdb_pending_compaction_bytes_limit").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);
    opts.set_soft_pending_compaction_bytes_limit(64 * 1024 * 1024 * 1024);
    opts.set_hard_pending_compaction_bytes_limit(256 * 1024 * 1024 * 1024);
    DB::open(opts, path.path().to_str().unwrap()).unwrap();
}

#[test]
fn test_set_max_subcompactions() {
    let path = TempDir::new("_rust_rocksdb_max_subcompactions").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);
    opts.set_max_subcompactions(4);
    DB::open(opts, path.path().to_str().unwrap()).unwrap();
}

#[test]
fn test_set_bytes_per_sync() {
    let path = TempDir::new("_rust_rocksdb_bytes_per_sync").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);
    opts.set_bytes_per_sync(1024 * 1024);
    opts.set_wal_bytes_per_sync(1024 * 1024);
    DB::open(opts, path.path().to_str().unwrap()).unwrap();
}

#[test]
fn test_set_optimize_filters_for_hits() {
    let path = TempDir::new("_rust_rocksdb_optimize_filters_for_hits").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);
    opts.set_optimize_filters_for_hits(true);
    DB::open(opts, path.path().to_str().unwrap()).unwrap();
}

#[test]
fn test_get_block_cache_usage() {
    let path = TempDir::new("_rust_rocksdb_set_cache_and_index").expect("");

    let mut opts = Options::new();
    assert_eq!(opts.get_block_cache_usage(), 0);

    opts.create_if_missing(true);
    let mut block_opts = BlockBasedOptions::new();
    block_opts.set_lru_cache(16 * 1024 * 1024);
    opts.set_block_based_table_factory(&block_opts);
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();

    for i in 0..200 {
        db.put(format!("k_{}", i).as_bytes(), b"v").unwrap();
    }
    db.flush(true).unwrap();
    for i in 0..200 {
        db.get(format!("k_{}", i).as_bytes()).unwrap();
    }

    assert!(db.get_options().get_block_cache_usage() > 0);
}

#[test]
fn test_set_level_compaction_dynamic_level_bytes() {
    let path = TempDir::new("_rust_rocksdb_level_compaction_dynamic_level_bytes").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);
    opts.set_level_compaction_dynamic_level_bytes(true);
    DB::open(opts, path.path().to_str().unwrap()).unwrap();
}

#[test]
fn test_compact_options() {
    let path = TempDir::new("_rust_rocksdb_compact_options").expect("");
    let db = DB::open_default(path.path().to_str().unwrap()).unwrap();
    db.put(b"k1", b"v1").unwrap();
    db.put(b"k2", b"v2").unwrap();
    db.flush(true).unwrap();

    let mut compact_opts = CompactOptions::new();
    compact_opts.set_exclusive_manual_compaction(false);
    let cf = db.cf_handle("default").unwrap();
    db.compact_range_cf_opt(cf, &compact_opts, None, None);
}

#[test]
fn test_direct_read_write() {
    let path = TempDir::new("_rust_rocksdb_direct_read_write").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);
    opts.set_use_direct_reads(true);
    opts.set_use_direct_io_for_flush_and_compaction(true);
    DB::open(opts, path.path().to_str().unwrap()).unwrap();
}

#[test]
fn test_writable_file_max_buffer_size() {
    let path = TempDir::new("_rust_rocksdb_writable_file_max_buffer_size").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);
    opts.set_writable_file_max_buffer_size(1024 * 1024);
    DB::open(opts, path.path().to_str().unwrap()).unwrap();
}

#[test]
fn test_set_base_background_compactions() {
    let path = TempDir::new("_rust_rocksdb_base_background_compactions").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);
    opts.set_base_background_compactions(4);
    DB::open(opts, path.path().to_str().unwrap()).unwrap();
}

#[test]
fn test_set_compaction_pri() {
    let path = TempDir::new("_rust_rocksdb_compaction_pri").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);
    opts.compaction_priority(CompactionPriority::MinOverlappingRatio);
    DB::open(opts, path.path().to_str().unwrap()).unwrap();
}

#[test]
fn test_allow_concurrent_memtable_write() {
    let path = TempDir::new("_rust_rocksdb_allow_concurrent_memtable_write").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);
    opts.allow_concurrent_memtable_write(false);
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    for i in 0..200 {
        db.put(format!("k_{}", i).as_bytes(), b"v").unwrap();
    }
}

#[test]
fn test_enable_pipelined_write() {
    let path = TempDir::new("_rust_rocksdb_enable_pipelined_write").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);
    opts.enable_pipelined_write(true);
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    for i in 0..200 {
        db.put(format!("k_{}", i).as_bytes(), b"v").unwrap();
    }
}

#[test]
fn test_get_compression() {
    let mut opts = Options::new();
    opts.create_if_missing(true);
    opts.compression(DBCompressionType::DBSnappy);
    assert_eq!(opts.get_compression(), DBCompressionType::DBSnappy);
}

#[test]
fn test_get_compression_per_level() {
    let mut opts = Options::new();
    let compressions = &[DBCompressionType::DBNo, DBCompressionType::DBSnappy];
    opts.compression_per_level(compressions);
    let v = opts.get_compression_per_level();
    assert_eq!(v.len(), 2);
    assert_eq!(v[0], DBCompressionType::DBNo);
    assert_eq!(v[1], DBCompressionType::DBSnappy);
    let mut opts2 = Options::new();
    let empty: &[DBCompressionType] = &[];
    opts2.compression_per_level(empty);
    let v2 = opts2.get_compression_per_level();
    assert_eq!(v2.len(), 0);
}

#[test]
fn test_bottommost_compression() {
    let path = TempDir::new("_rust_rocksdb_bottommost_compression").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);
    opts.bottommost_compression(DBCompressionType::DBNo);
    DB::open(opts, path.path().to_str().unwrap()).unwrap();
}

#[test]
fn test_clone_options() {
    let mut opts = Options::new();
    opts.compression(DBCompressionType::DBSnappy);
    let opts2 = opts.clone();
    assert_eq!(opts.get_compression(), opts2.get_compression());
}
