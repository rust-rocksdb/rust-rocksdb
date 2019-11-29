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

use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use rocksdb::crocksdb_ffi::{
    CompactionPriority, DBCompressionType, DBInfoLogLevel as InfoLogLevel, DBRateLimiterMode,
    DBStatisticsHistogramType as HistogramType, DBStatisticsTickerType as TickerType,
};
use rocksdb::{
    BlockBasedOptions, Cache, ColumnFamilyOptions, CompactOptions, DBOptions, Env,
    FifoCompactionOptions, IndexType, LRUCacheOptions, ReadOptions, SeekKey, SliceTransform,
    Writable, WriteOptions, DB,
};

use super::tempdir_with_prefix;

#[test]
fn test_set_num_levels() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_set_num_levels");
    let mut opts = DBOptions::new();
    let mut cf_opts = ColumnFamilyOptions::new();
    opts.create_if_missing(true);
    cf_opts.set_num_levels(2);
    assert_eq!(2, cf_opts.get_num_levels());
    let db = DB::open_cf(
        opts,
        path.path().to_str().unwrap(),
        vec![("default", cf_opts)],
    )
    .unwrap();
    drop(db);
}

#[test]
fn test_log_file_opt() {
    let path = tempdir_with_prefix("_rust_rocksdb_log_file_opt");
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.set_max_log_file_size(100 * 1024 * 1024);
    opts.set_keep_log_file_num(10);
    opts.set_recycle_log_file_num(10);
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    drop(db);
}

#[test]
fn test_compaction_readahead_size() {
    let path = tempdir_with_prefix("_rust_rocksdb_compaction_readahead_size");
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.set_compaction_readahead_size(2048);
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    drop(db);
}

#[test]
fn test_set_max_manifest_file_size() {
    let mut opts = DBOptions::new();
    let size = 20 * 1024 * 1024;
    opts.set_max_manifest_file_size(size)
}

#[test]
fn test_enable_statistics() {
    let mut opts = DBOptions::new();
    opts.enable_statistics(true);
    opts.set_stats_dump_period_sec(60);
    assert!(opts.get_statistics().is_some());
    assert!(opts
        .get_statistics_histogram(HistogramType::DbSeek)
        .is_some());
    assert!(opts
        .get_statistics_histogram_string(HistogramType::DbSeek)
        .is_some());
    assert_eq!(
        opts.get_statistics_ticker_count(TickerType::BlockCacheMiss),
        0
    );
    assert_eq!(
        opts.get_and_reset_statistics_ticker_count(TickerType::BlockCacheMiss),
        0
    );
    assert_eq!(
        opts.get_statistics_ticker_count(TickerType::BlockCacheMiss),
        0
    );

    let opts = DBOptions::new();
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
    let path = tempdir_with_prefix("_rust_rocksdb_memtable_insert_hint_prefix_extractor");
    let mut opts = DBOptions::new();
    let mut cf_opts = ColumnFamilyOptions::new();
    opts.create_if_missing(true);
    cf_opts
        .set_memtable_insert_hint_prefix_extractor(
            "FixedPrefixTransform",
            Box::new(FixedPrefixTransform { prefix_len: 2 }),
        )
        .unwrap();
    let db = DB::open_cf(
        opts,
        path.path().to_str().unwrap(),
        vec![("default", cf_opts)],
    )
    .unwrap();
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
    let path = tempdir_with_prefix("_rust_rocksdb_test_set_delayed_write_rate");
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.set_delayed_write_rate(2 * 1024 * 1024);
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    drop(db);
}

#[test]
fn test_set_ratelimiter() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_set_rate_limiter");
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    // compaction and flush rate limited below 100MB/sec
    opts.set_ratelimiter(100 * 1024 * 1024);
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    drop(db);
}

#[test]
fn test_set_ratelimiter_with_auto_tuned() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_set_rate_limiter_with_auto_tuned");
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.set_ratelimiter_with_auto_tuned(100 * 1024 * 1024, DBRateLimiterMode::AllIo, true);
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    drop(db);
}

#[test]
fn test_set_wal_opt() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_set_wal_opt");
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.set_wal_ttl_seconds(86400);
    opts.set_wal_size_limit_mb(10);
    let wal_dir = tempdir_with_prefix("_rust_rocksdb_test_set_wal_dir");
    opts.set_wal_dir(wal_dir.path().to_str().unwrap());
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    drop(db);
}

#[cfg(not(windows))]
#[test]
fn test_flush_wal() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_flush_wal");
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.manual_wal_flush(true);
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    db.put(b"key", b"value").unwrap();
    db.flush_wal(false).unwrap();
    drop(db);
}

#[cfg(not(windows))]
#[test]
fn test_sync_wal() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_sync_wal");
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    db.put(b"key", b"value").unwrap();
    db.sync_wal().unwrap();
    drop(db);
}

#[test]
fn test_create_info_log() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_create_info_log_opt");
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.set_info_log_level(InfoLogLevel::Debug);
    opts.set_log_file_time_to_roll(1);

    let info_dir = tempdir_with_prefix("_rust_rocksdb_test_info_log_dir");
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
    let path = tempdir_with_prefix("_rust_rocksdb_test_max_size_info_log_opt");
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.set_max_log_file_size(10);

    let info_dir = tempdir_with_prefix("_rust_rocksdb_max_size_info_log_dir");
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
    let path = tempdir_with_prefix("_rust_rocksdb_set_cache_and_index");
    let mut opts = DBOptions::new();
    let mut cf_opts = ColumnFamilyOptions::new();
    opts.create_if_missing(true);
    let mut block_opts = BlockBasedOptions::new();
    block_opts.set_pin_l0_filter_and_index_blocks_in_cache(true);
    cf_opts.set_block_based_table_factory(&block_opts);
    DB::open_cf(
        opts,
        path.path().to_str().unwrap(),
        vec![("default", cf_opts)],
    )
    .unwrap();
}

#[test]
fn test_partitioned_index_filters() {
    let path = tempdir_with_prefix("_rust_rocksdb_set_cache_and_index");
    let mut opts = DBOptions::new();
    let mut cf_opts = ColumnFamilyOptions::new();
    opts.create_if_missing(true);
    let mut block_opts = BlockBasedOptions::new();
    // See https://github.com/facebook/rocksdb/wiki/Partitioned-Index-Filters#how-to-use-it
    block_opts.set_index_type(IndexType::TwoLevelIndexSearch);
    block_opts.set_partition_filters(true);
    block_opts.set_bloom_filter(10, false);
    block_opts.set_metadata_block_size(4096);
    block_opts.set_cache_index_and_filter_blocks(true);
    block_opts.set_pin_top_level_index_and_filter(true);
    block_opts.set_cache_index_and_filter_blocks_with_high_priority(true);
    block_opts.set_pin_l0_filter_and_index_blocks_in_cache(true);
    cf_opts.set_block_based_table_factory(&block_opts);
    DB::open_cf(
        opts,
        path.path().to_str().unwrap(),
        vec![("default", cf_opts)],
    )
    .unwrap();
}

#[test]
fn test_set_lru_cache() {
    let path = tempdir_with_prefix("_rust_rocksdb_set_set_lru_cache");
    let mut opts = DBOptions::new();
    let mut cf_opts = ColumnFamilyOptions::new();
    opts.create_if_missing(true);
    let mut block_opts = BlockBasedOptions::new();
    let mut cache_opts = LRUCacheOptions::new();
    cache_opts.set_capacity(8388608);
    block_opts.set_block_cache(&Cache::new_lru_cache(cache_opts));
    cf_opts.set_block_based_table_factory(&block_opts);
    DB::open_cf(opts, path.path().to_str().unwrap(), vec!["default"]).unwrap();
}

#[cfg(feature = "jemalloc")]
#[test]
fn test_set_jemalloc_nodump_allocator_for_lru_cache() {
    use rocksdb::MemoryAllocator;
    let path = tempdir_with_prefix("_rust_rocksdb_set_jemalloc_nodump_allocator");
    let mut opts = DBOptions::new();
    let mut cf_opts = ColumnFamilyOptions::new();
    opts.create_if_missing(true);
    let mut block_opts = BlockBasedOptions::new();
    let mut cache_opts = LRUCacheOptions::new();
    cache_opts.set_memory_allocator(MemoryAllocator::new_jemalloc_memory_allocator().unwrap());
    cache_opts.set_capacity(8388608);
    block_opts.set_block_cache(&Cache::new_lru_cache(cache_opts));
    cf_opts.set_block_based_table_factory(&block_opts);
    DB::open_cf(opts, path.path().to_str().unwrap(), vec!["default"]).unwrap();
}

#[test]
fn test_set_cache_index_and_filter_blocks_with_high_priority() {
    let path = tempdir_with_prefix("_rust_rocksdb_set_cache_and_index_with_high_priority");
    let mut opts = DBOptions::new();
    let mut cf_opts = ColumnFamilyOptions::new();
    opts.create_if_missing(true);
    let mut block_opts = BlockBasedOptions::new();
    block_opts.set_cache_index_and_filter_blocks_with_high_priority(true);
    cf_opts.set_block_based_table_factory(&block_opts);
    DB::open_cf(
        opts,
        path.path().to_str().unwrap(),
        vec![("default", cf_opts)],
    )
    .unwrap();
}

#[test]
fn test_pending_compaction_bytes_limit() {
    let path = tempdir_with_prefix("_rust_rocksdb_pending_compaction_bytes_limit");
    let mut opts = DBOptions::new();
    let mut cf_opts = ColumnFamilyOptions::new();
    opts.create_if_missing(true);
    cf_opts.set_soft_pending_compaction_bytes_limit(64 * 1024 * 1024 * 1024);
    cf_opts.set_hard_pending_compaction_bytes_limit(256 * 1024 * 1024 * 1024);
    DB::open_cf(
        opts,
        path.path().to_str().unwrap(),
        vec![("default", cf_opts)],
    )
    .unwrap();
}

#[test]
fn test_set_max_subcompactions() {
    let path = tempdir_with_prefix("_rust_rocksdb_max_subcompactions");
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.set_max_subcompactions(4);
    DB::open(opts, path.path().to_str().unwrap()).unwrap();
}

#[test]
fn test_set_bytes_per_sync() {
    let path = tempdir_with_prefix("_rust_rocksdb_bytes_per_sync");
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.set_bytes_per_sync(1024 * 1024);
    opts.set_wal_bytes_per_sync(1024 * 1024);
    DB::open(opts, path.path().to_str().unwrap()).unwrap();
}

#[test]
fn test_set_optimize_filters_for_hits() {
    let path = tempdir_with_prefix("_rust_rocksdb_optimize_filters_for_hits");
    let mut opts = DBOptions::new();
    let mut cf_opts = ColumnFamilyOptions::new();
    opts.create_if_missing(true);
    cf_opts.set_optimize_filters_for_hits(true);
    DB::open_cf(
        opts,
        path.path().to_str().unwrap(),
        vec![("default", cf_opts)],
    )
    .unwrap();
}

#[test]
fn test_set_force_consistency_checks() {
    let path = tempdir_with_prefix("_rust_rocksdb_force_consistency_checks");
    let mut opts = DBOptions::new();
    let mut cf_opts = ColumnFamilyOptions::new();
    opts.create_if_missing(true);
    cf_opts.set_force_consistency_checks(true);
    DB::open_cf(
        opts,
        path.path().to_str().unwrap(),
        vec![("default", cf_opts)],
    )
    .unwrap();
}

#[test]
fn test_get_block_cache_usage() {
    let path = tempdir_with_prefix("_rust_rocksdb_set_cache_and_index");

    let mut opts = DBOptions::new();
    let mut cf_opts = ColumnFamilyOptions::new();
    assert_eq!(cf_opts.get_block_cache_usage(), 0);

    opts.create_if_missing(true);
    let mut block_opts = BlockBasedOptions::new();
    let mut cache_opts = LRUCacheOptions::new();
    cache_opts.set_capacity(16 * 1024 * 1024);
    block_opts.set_block_cache(&Cache::new_lru_cache(cache_opts));
    cf_opts.set_block_based_table_factory(&block_opts);
    let db = DB::open_cf(
        opts,
        path.path().to_str().unwrap(),
        vec![("default", cf_opts)],
    )
    .unwrap();

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
fn test_block_cache_capacity() {
    let path = tempdir_with_prefix("_rust_rocksdb_set_and_get_block_cache_capacity");

    let mut opts = DBOptions::new();
    let mut cf_opts = ColumnFamilyOptions::new();
    opts.create_if_missing(true);
    let mut block_opts = BlockBasedOptions::new();
    let mut cache_opts = LRUCacheOptions::new();
    cache_opts.set_capacity(16 * 1024 * 1024);
    block_opts.set_block_cache(&Cache::new_lru_cache(cache_opts));
    cf_opts.set_block_based_table_factory(&block_opts);
    let db = DB::open_cf(
        opts,
        path.path().to_str().unwrap(),
        vec![("default", cf_opts)],
    )
    .unwrap();

    assert_eq!(
        db.get_options().get_block_cache_capacity(),
        16 * 1024 * 1024
    );

    let opt = db.get_options();
    opt.set_block_cache_capacity(32 * 1024 * 1024).unwrap();

    assert_eq!(
        db.get_options().get_block_cache_capacity(),
        32 * 1024 * 1024
    );
}

#[test]
fn test_disable_block_cache() {
    let mut cf_opts = ColumnFamilyOptions::new();
    assert_eq!(cf_opts.get_block_cache_usage(), 0);
    let mut block_opts = BlockBasedOptions::new();
    block_opts.set_no_block_cache(true);
    cf_opts.set_block_based_table_factory(&block_opts);
    assert_eq!(cf_opts.get_block_cache_usage(), 0);
}

#[test]
fn test_set_level_compaction_dynamic_level_bytes() {
    let path = tempdir_with_prefix("_rust_rocksdb_level_compaction_dynamic_level_bytes");
    let mut opts = DBOptions::new();
    let mut cf_opts = ColumnFamilyOptions::new();
    opts.create_if_missing(true);
    cf_opts.set_level_compaction_dynamic_level_bytes(true);
    DB::open_cf(
        opts,
        path.path().to_str().unwrap(),
        vec![("default", cf_opts)],
    )
    .unwrap();
}

#[test]
fn test_compact_options() {
    let path = tempdir_with_prefix("_rust_rocksdb_compact_options");
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
    let path = tempdir_with_prefix("_rust_rocksdb_direct_read_write");
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.set_use_direct_reads(true);
    opts.set_use_direct_io_for_flush_and_compaction(true);
    DB::open(opts, path.path().to_str().unwrap()).unwrap();
}

#[test]
fn test_writable_file_max_buffer_size() {
    let path = tempdir_with_prefix("_rust_rocksdb_writable_file_max_buffer_size");
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.set_writable_file_max_buffer_size(1024 * 1024);
    DB::open(opts, path.path().to_str().unwrap()).unwrap();
}

#[test]
fn test_set_max_background_jobs() {
    let path = tempdir_with_prefix("_rust_rocksdb_max_background_jobs");
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.set_max_background_jobs(4);
    DB::open(opts, path.path().to_str().unwrap()).unwrap();
}

#[test]
fn test_set_compaction_pri() {
    let path = tempdir_with_prefix("_rust_rocksdb_compaction_pri");
    let mut opts = DBOptions::new();
    let mut cf_opts = ColumnFamilyOptions::new();
    opts.create_if_missing(true);
    cf_opts.compaction_priority(CompactionPriority::MinOverlappingRatio);
    DB::open_cf(
        opts,
        path.path().to_str().unwrap(),
        vec![("default", cf_opts)],
    )
    .unwrap();
}

#[test]
fn test_allow_concurrent_memtable_write() {
    let path = tempdir_with_prefix("_rust_rocksdb_allow_concurrent_memtable_write");
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.allow_concurrent_memtable_write(false);
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    for i in 0..200 {
        db.put(format!("k_{}", i).as_bytes(), b"v").unwrap();
    }
}

#[test]
fn test_manual_wal_flush() {
    let path = tempdir_with_prefix("_rust_rocksdb_manual_wal_flush");
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.manual_wal_flush(true);
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    for i in 0..200 {
        db.put(format!("k_{}", i).as_bytes(), b"v").unwrap();
    }
}

#[test]
fn test_enable_pipelined_write() {
    let path = tempdir_with_prefix("_rust_rocksdb_enable_pipelined_write");
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.enable_pipelined_write(true);
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    for i in 0..200 {
        db.put(format!("k_{}", i).as_bytes(), b"v").unwrap();
    }
}

#[test]
fn test_get_compression() {
    let mut opts = DBOptions::new();
    let mut cf_opts = ColumnFamilyOptions::new();
    opts.create_if_missing(true);
    cf_opts.compression(DBCompressionType::Snappy);
    assert_eq!(cf_opts.get_compression(), DBCompressionType::Snappy);
}

#[test]
fn test_get_compression_per_level() {
    let mut cf_opts = ColumnFamilyOptions::new();
    let compressions = &[DBCompressionType::No, DBCompressionType::Snappy];
    cf_opts.compression_per_level(compressions);
    let v = cf_opts.get_compression_per_level();
    assert_eq!(v.len(), 2);
    assert_eq!(v[0], DBCompressionType::No);
    assert_eq!(v[1], DBCompressionType::Snappy);
    let mut cf_opts2 = ColumnFamilyOptions::new();
    let empty: &[DBCompressionType] = &[];
    cf_opts2.compression_per_level(empty);
    let v2 = cf_opts2.get_compression_per_level();
    assert_eq!(v2.len(), 0);
}

#[test]
fn test_bottommost_compression() {
    let path = tempdir_with_prefix("_rust_rocksdb_bottommost_compression");
    let mut opts = DBOptions::new();
    let cf_opts = ColumnFamilyOptions::new();
    opts.create_if_missing(true);
    cf_opts.bottommost_compression(DBCompressionType::No);
    DB::open_cf(
        opts,
        path.path().to_str().unwrap(),
        vec![("default", cf_opts)],
    )
    .unwrap();
}

#[test]
fn test_clone_options() {
    let mut cf_opts = ColumnFamilyOptions::new();
    cf_opts.compression(DBCompressionType::Snappy);
    let cf_opts2 = cf_opts.clone();
    assert_eq!(cf_opts.get_compression(), cf_opts2.get_compression());
}

#[test]
fn test_db_paths() {
    let path = tempdir_with_prefix("_rust_rocksdb_db_paths");
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let tmp_path = tempdir_with_prefix("_rust_rocksdb_test_db_path");
    opts.set_db_paths(&[(tmp_path.path(), 1073741824 as u64)]);
    DB::open(opts, path.path().to_str().unwrap()).unwrap();
}

#[test]
fn test_write_options() {
    let path = tempdir_with_prefix("_rust_rocksdb_write_options");
    let db = DB::open_default(path.path().to_str().unwrap()).unwrap();

    let mut write_opts = WriteOptions::new();
    write_opts.set_ignore_missing_column_families(true);
    write_opts.set_no_slowdown(true);
    write_opts.set_low_pri(true);
    db.put_opt(b"k1", b"a", &write_opts).unwrap();
    write_opts.set_sync(true);
    write_opts.disable_wal(false);
    db.put_opt(b"k2", b"b", &write_opts).unwrap();
    write_opts.set_sync(false);
    write_opts.disable_wal(true);
    db.put_opt(b"k3", b"c", &write_opts).unwrap();
    assert_eq!(db.get(b"k1").unwrap().unwrap(), b"a");
    assert_eq!(db.get(b"k2").unwrap().unwrap(), b"b");
    assert_eq!(db.get(b"k3").unwrap().unwrap(), b"c");
}

#[test]
fn test_read_options() {
    let path = tempdir_with_prefix("_rust_rocksdb_write_options");
    let db = DB::open_default(path.path().to_str().unwrap()).unwrap();

    let mut read_opts = ReadOptions::new();
    read_opts.set_verify_checksums(true);
    read_opts.fill_cache(true);
    read_opts.set_tailing(true);
    read_opts.set_pin_data(true);
    read_opts.set_background_purge_on_iterator_cleanup(true);
    read_opts.set_ignore_range_deletions(true);
    read_opts.set_max_skippable_internal_keys(0);
    read_opts.set_readahead_size(0);
    read_opts.set_read_tier(0);

    db.put(b"k1", b"a").unwrap();
    db.put(b"k2", b"b").unwrap();
    db.put(b"k3", b"c").unwrap();

    let keys = vec![b"k1", b"k2", b"k3"];
    let mut iter = db.iter_opt(read_opts);
    iter.seek(SeekKey::Key(b"k1"));
    let mut key_count = 0;
    while iter.valid() {
        assert_eq!(keys[key_count], iter.key());
        key_count = key_count + 1;
        iter.next();
    }
    assert!(key_count == 3);
}

#[test]
fn test_readoptions_lower_bound() {
    let path = tempdir_with_prefix("_rust_rocksdb_readoptions_lower_bound");
    let db = DB::open_default(path.path().to_str().unwrap()).unwrap();

    db.put(b"k1", b"b").unwrap();
    db.put(b"k2", b"a").unwrap();
    db.put(b"k3", b"a").unwrap();

    let mut read_opts = ReadOptions::new();
    let lower_bound = b"k2".to_vec();
    read_opts.set_iterate_lower_bound(lower_bound);
    let mut iter = db.iter_opt(read_opts);
    iter.seek(SeekKey::Key(b"k3"));
    let mut count = 0;
    while iter.valid() {
        count += 1;
        iter.prev();
    }
    assert_eq!(count, 2);
}

#[test]
fn test_block_based_options() {
    let path = tempdir_with_prefix("_rust_rocksdb_block_based_options");
    let path_str = path.path().to_str().unwrap();

    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.enable_statistics(true);
    opts.set_stats_dump_period_sec(60);
    let mut bopts = BlockBasedOptions::new();
    bopts.set_read_amp_bytes_per_bit(4);
    let mut cfopts = ColumnFamilyOptions::new();
    cfopts.set_block_based_table_factory(&bopts);

    let db = DB::open_cf(opts.clone(), path_str, vec![("default", cfopts)]).unwrap();
    // RocksDB use randomness for the read amplification statistics,
    // we should use a bigger enough value (> `bytes_per_bit`) to make
    // sure the statistics will not be 0.
    db.put(b"a", b"abcdef").unwrap();
    db.flush(true).unwrap();
    db.get(b"a").unwrap();
    assert_ne!(
        opts.get_statistics_ticker_count(TickerType::ReadAmpTotalReadBytes),
        0
    );
    assert_ne!(
        opts.get_statistics_ticker_count(TickerType::ReadAmpEstimateUsefulBytes),
        0
    );
}

#[test]
fn test_fifo_compaction_options() {
    let path = tempdir_with_prefix("_rust_rocksdb_fifo_compaction_options");
    let path_str = path.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let mut cf_opts = ColumnFamilyOptions::new();
    let mut fifo_opts = FifoCompactionOptions::new();
    fifo_opts.set_allow_compaction(true);
    fifo_opts.set_max_table_files_size(100000);
    cf_opts.set_fifo_compaction_options(fifo_opts);
    DB::open_cf(opts, path_str, vec![("default", cf_opts)]).unwrap();
}

#[test]
fn test_readoptions_max_bytes_for_level_multiplier() {
    let mut cf_opts = ColumnFamilyOptions::new();
    cf_opts.set_max_bytes_for_level_multiplier(8);
    assert_eq!(cf_opts.get_max_bytes_for_level_multiplier(), 8);
}

#[test]
fn test_vector_memtable_factory_options() {
    let path = tempdir_with_prefix("_rust_rocksdb_vector_memtable_factory_options");
    let path_str = path.path().to_str().unwrap();

    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.allow_concurrent_memtable_write(false);

    let mut cf_opts = ColumnFamilyOptions::new();
    cf_opts.set_vector_memtable_factory(4096);

    let db = DB::open_cf(opts.clone(), path_str, vec![("default", cf_opts)]).unwrap();
    db.put(b"k1", b"v1").unwrap();
    db.put(b"k2", b"v2").unwrap();
    assert_eq!(db.get(b"k1").unwrap().unwrap(), b"v1");
    assert_eq!(db.get(b"k2").unwrap().unwrap(), b"v2");
    db.flush(true).unwrap();

    let mut iter = db.iter();
    iter.seek(SeekKey::Start);
    assert!(iter.valid());
    assert_eq!(iter.key(), b"k1");
    assert_eq!(iter.value(), b"v1");
    assert!(iter.next());
    assert_eq!(iter.key(), b"k2");
    assert_eq!(iter.value(), b"v2");
    assert!(!iter.next());
    assert!(!iter.valid());
}

#[test]
fn test_dboptions_set_env() {
    let path = tempdir_with_prefix("_rust_rocksdb_dboptions_set_env");
    let path_str = path.path().to_str().unwrap();

    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.set_env(Arc::new(Env::default()));
    let _db = DB::open(opts, path_str).unwrap();
}
