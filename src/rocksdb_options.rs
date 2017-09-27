// Copyright 2014 Tyler Neely
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
//

use compaction_filter::{new_compaction_filter, CompactionFilter, CompactionFilterHandle};
use comparator::{self, compare_callback, ComparatorCallback};
use crocksdb_ffi::{self, DBBlockBasedTableOptions, DBCompactOptions, DBCompressionType,
                   DBFlushOptions, DBInfoLogLevel, DBInstance, DBRateLimiter, DBReadOptions,
                   DBRecoveryMode, DBRestoreOptions, DBSnapshot, DBStatisticsHistogramType,
                   DBStatisticsTickerType, DBWriteOptions, Options};
use event_listener::{new_event_listener, EventListener};
use libc::{self, c_int, c_void, size_t};
use merge_operator::{self, full_merge_callback, partial_merge_callback, MergeOperatorCallback};
use merge_operator::MergeFn;
use slice_transform::{new_slice_transform, SliceTransform};
use std::ffi::{CStr, CString};
use std::mem;
use std::path::Path;
use table_properties_collector_factory::{new_table_properties_collector_factory,
                                         TablePropertiesCollectorFactory};

#[derive(Default, Debug)]
pub struct HistogramData {
    pub median: f64,
    pub percentile95: f64,
    pub percentile99: f64,
    pub average: f64,
    pub standard_deviation: f64,
    pub max: f64,
}

pub struct BlockBasedOptions {
    inner: *mut DBBlockBasedTableOptions,
}

impl Drop for BlockBasedOptions {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_block_based_options_destroy(self.inner);
        }
    }
}

impl Default for BlockBasedOptions {
    fn default() -> BlockBasedOptions {
        unsafe {
            let block_opts = crocksdb_ffi::crocksdb_block_based_options_create();
            assert!(
                !block_opts.is_null(),
                "Could not create rocksdb block based options"
            );
            BlockBasedOptions { inner: block_opts }
        }
    }
}

impl BlockBasedOptions {
    pub fn new() -> BlockBasedOptions {
        BlockBasedOptions::default()
    }

    pub fn set_block_size(&mut self, size: usize) {
        unsafe {
            crocksdb_ffi::crocksdb_block_based_options_set_block_size(self.inner, size);
        }
    }

    pub fn set_lru_cache(&mut self, size: size_t) {
        let cache = crocksdb_ffi::new_cache(size);
        unsafe {
            crocksdb_ffi::crocksdb_block_based_options_set_block_cache(self.inner, cache);
            crocksdb_ffi::crocksdb_cache_destroy(cache);
        }
    }

    pub fn set_bloom_filter(&mut self, bits_per_key: c_int, block_based: bool) {
        unsafe {
            let bloom = if block_based {
                crocksdb_ffi::crocksdb_filterpolicy_create_bloom(bits_per_key)
            } else {
                crocksdb_ffi::crocksdb_filterpolicy_create_bloom_full(bits_per_key)
            };

            crocksdb_ffi::crocksdb_block_based_options_set_filter_policy(self.inner, bloom);
        }
    }

    pub fn set_cache_index_and_filter_blocks(&mut self, v: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_block_based_options_set_cache_index_and_filter_blocks(
                self.inner,
                v as u8,
            );
        }
    }

    pub fn set_whole_key_filtering(&mut self, v: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_block_based_options_set_whole_key_filtering(self.inner, v);
        }
    }

    pub fn set_pin_l0_filter_and_index_blocks_in_cache(&mut self, v: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_block_based_options_set_pin_l0_filter_and_index_blocks_in_cache(
                self.inner,
                v as u8);
        }
    }

    pub fn set_read_amp_bytes_per_bit(&mut self, v: u32) {
        unsafe {
            crocksdb_ffi::crocksdb_block_based_options_set_read_amp_bytes_per_bit(
                self.inner,
                v as c_int,
            )
        }
    }
}

pub struct RateLimiter {
    inner: *mut DBRateLimiter,
}

impl RateLimiter {
    pub fn new(rate_bytes_per_sec: i64, refill_period_us: i64, fairness: i32) -> RateLimiter {
        let limiter = unsafe {
            crocksdb_ffi::crocksdb_ratelimiter_create(
                rate_bytes_per_sec,
                refill_period_us,
                fairness,
            )
        };
        RateLimiter { inner: limiter }
    }
}

impl Drop for RateLimiter {
    fn drop(&mut self) {
        unsafe { crocksdb_ffi::crocksdb_ratelimiter_destroy(self.inner) }
    }
}

const DEFAULT_REFILL_PERIOD_US: i64 = 100 * 1000; // 100ms should work for most cases
const DEFAULT_FAIRNESS: i32 = 10; // should be good by leaving it at default 10

/// The UnsafeSnap must be destroyed by db, it maybe be leaked
/// if not using it properly, hence named as unsafe.
///
/// This object is convenient for wrapping snapshot by yourself. In most
/// cases, using `Snapshot` is enough.
pub struct UnsafeSnap {
    inner: *const DBSnapshot,
}

impl UnsafeSnap {
    pub unsafe fn new(db: *mut DBInstance) -> UnsafeSnap {
        UnsafeSnap {
            inner: crocksdb_ffi::crocksdb_create_snapshot(db),
        }
    }

    pub unsafe fn get_inner(&self) -> *const DBSnapshot {
        self.inner
    }
}

pub struct ReadOptions {
    inner: *mut DBReadOptions,
    upper_bound: Vec<u8>,
}

impl Drop for ReadOptions {
    fn drop(&mut self) {
        unsafe { crocksdb_ffi::crocksdb_readoptions_destroy(self.inner) }
    }
}

impl Default for ReadOptions {
    fn default() -> ReadOptions {
        unsafe {
            let opts = crocksdb_ffi::crocksdb_readoptions_create();
            assert!(!opts.is_null(), "Unable to create rocksdb read options");
            ReadOptions {
                inner: opts,
                upper_bound: vec![],
            }
        }
    }
}

impl ReadOptions {
    pub fn new() -> ReadOptions {
        ReadOptions::default()
    }

    // TODO add snapshot setting here
    // TODO add snapshot wrapper structs with proper destructors;
    // that struct needs an "iterator" impl too.
    #[allow(dead_code)]

    pub fn set_verify_checksums(&mut self, v: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_readoptions_set_verify_checksums(self.inner, v);
        }
    }

    pub fn fill_cache(&mut self, v: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_readoptions_set_fill_cache(self.inner, v);
        }
    }

    pub unsafe fn set_snapshot(&mut self, snapshot: &UnsafeSnap) {
        crocksdb_ffi::crocksdb_readoptions_set_snapshot(self.inner, snapshot.inner);
    }

    pub fn set_iterate_upper_bound(&mut self, key: &[u8]) {
        self.upper_bound = Vec::from(key);
        unsafe {
            crocksdb_ffi::crocksdb_readoptions_set_iterate_upper_bound(
                self.inner,
                self.upper_bound.as_ptr(),
                self.upper_bound.len(),
            );
        }
    }

    pub fn set_read_tier(&mut self, tier: c_int) {
        unsafe {
            crocksdb_ffi::crocksdb_readoptions_set_read_tier(self.inner, tier);
        }
    }

    pub fn set_tailing(&mut self, v: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_readoptions_set_tailing(self.inner, v);
        }
    }

    pub fn set_managed(&mut self, v: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_readoptions_set_managed(self.inner, v);
        }
    }

    pub fn set_readahead_size(&mut self, size: size_t) {
        unsafe {
            crocksdb_ffi::crocksdb_readoptions_set_readahead_size(self.inner, size);
        }
    }

    pub fn set_max_skippable_internal_keys(&mut self, n: u64) {
        unsafe {
            crocksdb_ffi::crocksdb_readoptions_set_max_skippable_internal_keys(self.inner, n);
        }
    }

    pub fn set_total_order_seek(&mut self, v: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_readoptions_set_total_order_seek(self.inner, v);
        }
    }

    pub fn set_prefix_same_as_start(&mut self, v: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_readoptions_set_prefix_same_as_start(self.inner, v);
        }
    }

    pub fn set_pin_data(&mut self, v: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_readoptions_set_pin_data(self.inner, v);
        }
    }

    pub fn set_background_purge_on_iterator_cleanup(&mut self, v: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_readoptions_set_background_purge_on_iterator_cleanup(
                self.inner,
                v,
            );
        }
    }

    pub fn set_ignore_range_deletions(&mut self, v: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_readoptions_set_ignore_range_deletions(self.inner, v);
        }
    }

    pub unsafe fn get_inner(&self) -> *const DBReadOptions {
        self.inner
    }
}

pub struct WriteOptions {
    pub inner: *mut DBWriteOptions,
}

impl Drop for WriteOptions {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_writeoptions_destroy(self.inner);
        }
    }
}

impl Default for WriteOptions {
    fn default() -> WriteOptions {
        let write_opts = unsafe { crocksdb_ffi::crocksdb_writeoptions_create() };
        assert!(
            !write_opts.is_null(),
            "Could not create rocksdb write options"
        );
        WriteOptions { inner: write_opts }
    }
}

impl WriteOptions {
    pub fn new() -> WriteOptions {
        WriteOptions::default()
    }

    pub fn set_sync(&mut self, sync: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_writeoptions_set_sync(self.inner, sync);
        }
    }

    pub fn disable_wal(&mut self, disable: bool) {
        unsafe {
            if disable {
                crocksdb_ffi::crocksdb_writeoptions_disable_wal(self.inner, 1);
            } else {
                crocksdb_ffi::crocksdb_writeoptions_disable_wal(self.inner, 0);
            }
        }
    }

    pub fn set_ignore_missing_column_families(&mut self, v: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_writeoptions_set_ignore_missing_column_families(self.inner, v);
        }
    }

    pub fn set_no_slowdown(&mut self, v: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_writeoptions_set_no_slowdown(self.inner, v);
        }
    }

    pub fn set_low_pri(&mut self, v: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_writeoptions_set_low_pri(self.inner, v);
        }
    }
}

pub struct CompactOptions {
    pub inner: *mut DBCompactOptions,
}

impl CompactOptions {
    pub fn new() -> CompactOptions {
        unsafe {
            CompactOptions {
                inner: crocksdb_ffi::crocksdb_compactoptions_create(),
            }
        }
    }

    pub fn set_exclusive_manual_compaction(&mut self, v: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_compactoptions_set_exclusive_manual_compaction(self.inner, v);
        }
    }
}

impl Drop for CompactOptions {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_compactoptions_destroy(self.inner);
        }
    }
}

pub struct DBOptions {
    pub inner: *mut Options,
}

impl Drop for DBOptions {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_options_destroy(self.inner);
        }
    }
}

impl Default for DBOptions {
    fn default() -> DBOptions {
        unsafe {
            let opts = crocksdb_ffi::crocksdb_options_create();
            assert!(!opts.is_null(), "Could not create rocksdb db options");
            DBOptions { inner: opts }
        }
    }
}

impl Clone for DBOptions {
    fn clone(&self) -> Self {
        unsafe {
            let opts = crocksdb_ffi::crocksdb_options_copy(self.inner);
            assert!(!opts.is_null());
            DBOptions { inner: opts }
        }
    }
}

impl DBOptions {
    pub fn new() -> DBOptions {
        DBOptions::default()
    }

    pub fn increase_parallelism(&mut self, parallelism: i32) {
        unsafe {
            crocksdb_ffi::crocksdb_options_increase_parallelism(self.inner, parallelism);
        }
    }

    pub fn add_event_listener<L: EventListener>(&mut self, l: L) {
        let handle = new_event_listener(l);
        unsafe { crocksdb_ffi::crocksdb_options_add_eventlistener(self.inner, handle) }
    }

    pub fn create_if_missing(&mut self, create_if_missing: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_create_if_missing(self.inner, create_if_missing);
        }
    }

    pub fn set_max_open_files(&mut self, nfiles: c_int) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_max_open_files(self.inner, nfiles);
        }
    }

    pub fn set_max_total_wal_size(&mut self, size: u64) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_max_total_wal_size(self.inner, size);
        }
    }

    pub fn set_use_fsync(&mut self, useit: bool) {
        unsafe {
            if useit {
                crocksdb_ffi::crocksdb_options_set_use_fsync(self.inner, 1)
            } else {
                crocksdb_ffi::crocksdb_options_set_use_fsync(self.inner, 0)
            }
        }
    }

    pub fn set_bytes_per_sync(&mut self, nbytes: u64) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_bytes_per_sync(self.inner, nbytes);
        }
    }

    pub fn set_table_cache_num_shard_bits(&mut self, nbits: c_int) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_table_cache_numshardbits(self.inner, nbits);
        }
    }

    pub fn set_writable_file_max_buffer_size(&mut self, nbytes: c_int) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_writable_file_max_buffer_size(self.inner, nbytes);
        }
    }

    pub fn set_use_direct_reads(&mut self, v: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_use_direct_reads(self.inner, v);
        }
    }

    pub fn set_use_direct_io_for_flush_and_compaction(&mut self, v: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_use_direct_io_for_flush_and_compaction(
                self.inner,
                v,
            );
        }
    }

    pub fn set_max_manifest_file_size(&mut self, size: u64) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_max_manifest_file_size(self.inner, size);
        }
    }

    pub fn set_max_background_jobs(&mut self, n: c_int) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_max_background_jobs(self.inner, n);
        }
    }

    pub fn set_max_subcompactions(&mut self, n: u32) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_max_subcompactions(self.inner, n);
        }
    }

    pub fn set_wal_bytes_per_sync(&mut self, n: u64) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_wal_bytes_per_sync(self.inner, n);
        }
    }

    pub fn set_wal_recovery_mode(&mut self, mode: DBRecoveryMode) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_wal_recovery_mode(self.inner, mode);
        }
    }

    pub fn set_delayed_write_rate(&mut self, rate: u64) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_delayed_write_rate(self.inner, rate);
        }
    }

    pub fn enable_statistics(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_options_enable_statistics(self.inner);
        }
    }

    pub fn get_statistics_ticker_count(&self, ticker_type: DBStatisticsTickerType) -> u64 {
        unsafe {
            crocksdb_ffi::crocksdb_options_statistics_get_ticker_count(self.inner, ticker_type)
        }
    }

    pub fn get_and_reset_statistics_ticker_count(
        &self,
        ticker_type: DBStatisticsTickerType,
    ) -> u64 {
        unsafe {
            crocksdb_ffi::crocksdb_options_statistics_get_and_reset_ticker_count(
                self.inner,
                ticker_type,
            )
        }
    }

    pub fn get_statistics_histogram(
        &self,
        hist_type: DBStatisticsHistogramType,
    ) -> Option<HistogramData> {
        unsafe {
            let mut data = HistogramData::default();
            let ret = crocksdb_ffi::crocksdb_options_statistics_get_histogram(
                self.inner,
                hist_type,
                &mut data.median,
                &mut data.percentile95,
                &mut data.percentile99,
                &mut data.average,
                &mut data.standard_deviation,
                &mut data.max,
            );
            if !ret {
                return None;
            }
            Some(data)
        }
    }

    pub fn get_statistics_histogram_string(
        &self,
        hist_type: DBStatisticsHistogramType,
    ) -> Option<String> {
        unsafe {
            let value = crocksdb_ffi::crocksdb_options_statistics_get_histogram_string(
                self.inner,
                hist_type,
            );

            if value.is_null() {
                return None;
            }

            let s = CStr::from_ptr(value).to_str().unwrap().to_owned();
            libc::free(value as *mut c_void);
            Some(s)
        }
    }

    pub fn get_statistics(&self) -> Option<String> {
        unsafe {
            let value = crocksdb_ffi::crocksdb_options_statistics_get_string(self.inner);


            if value.is_null() {
                return None;
            }

            // Must valid UTF-8 format.
            let s = CStr::from_ptr(value).to_str().unwrap().to_owned();
            libc::free(value as *mut c_void);
            Some(s)
        }
    }

    pub fn set_stats_dump_period_sec(&mut self, period: usize) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_stats_dump_period_sec(self.inner, period);
        }
    }

    pub fn set_db_log_dir(&mut self, path: &str) {
        let path = CString::new(path.as_bytes()).unwrap();
        unsafe {
            crocksdb_ffi::crocksdb_options_set_db_log_dir(self.inner, path.as_ptr());
        }
    }

    pub fn set_wal_dir(&mut self, path: &str) {
        let path = CString::new(path.as_bytes()).unwrap();
        unsafe {
            crocksdb_ffi::crocksdb_options_set_wal_dir(self.inner, path.as_ptr());
        }
    }

    pub fn set_wal_ttl_seconds(&mut self, ttl: u64) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_wal_ttl_seconds(self.inner, ttl as u64);
        }
    }

    pub fn set_wal_size_limit_mb(&mut self, limit: u64) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_wal_size_limit_mb(self.inner, limit as u64);
        }
    }

    pub fn set_max_log_file_size(&mut self, size: u64) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_max_log_file_size(self.inner, size as size_t);
        }
    }

    pub fn set_log_file_time_to_roll(&mut self, ttl: u64) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_log_file_time_to_roll(self.inner, ttl as size_t);
        }
    }

    pub fn set_info_log_level(&mut self, level: DBInfoLogLevel) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_info_log_level(self.inner, level);
        }
    }

    pub fn set_keep_log_file_num(&mut self, num: u64) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_keep_log_file_num(self.inner, num as size_t);
        }
    }

    pub fn set_compaction_readahead_size(&mut self, size: u64) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_compaction_readahead_size(
                self.inner,
                size as size_t,
            );
        }
    }

    pub fn set_ratelimiter(&mut self, rate_bytes_per_sec: i64) {
        let rate_limiter = RateLimiter::new(
            rate_bytes_per_sec,
            DEFAULT_REFILL_PERIOD_US,
            DEFAULT_FAIRNESS,
        );
        unsafe {
            crocksdb_ffi::crocksdb_options_set_ratelimiter(self.inner, rate_limiter.inner);
        }
    }

    // Create a info log with `path` and save to options logger field directly.
    // TODO: export more logger options like level, roll size, time, etc...
    pub fn create_info_log(&self, path: &str) -> Result<(), String> {
        let cpath = match CString::new(path.as_bytes()) {
            Ok(c) => c,
            Err(_) => {
                return Err(
                    "Failed to convert path to CString when creating rocksdb info log".to_owned(),
                )
            }
        };

        unsafe {
            let logger = ffi_try!(crocksdb_create_log_from_options(cpath.as_ptr(), self.inner));
            crocksdb_ffi::crocksdb_options_set_info_log(self.inner, logger);
            // logger uses shared_ptr, it is OK to destroy here.
            crocksdb_ffi::crocksdb_log_destroy(logger);
        }

        Ok(())
    }

    pub fn enable_pipelined_write(&self, v: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_enable_pipelined_write(self.inner, v);
        }
    }

    pub fn allow_concurrent_memtable_write(&self, v: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_allow_concurrent_memtable_write(self.inner, v);
        }
    }

    pub fn manual_wal_flush(&self, v: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_manual_wal_flush(self.inner, v);
        }
    }

    /// the second parameter is a slice which contains tuples (path, target_size).
    pub fn set_db_paths<T: AsRef<Path>>(&self, val: &[(T, u64)]) {
        let num_paths = val.len();
        let mut cpaths = Vec::with_capacity(num_paths);
        let mut cpath_lens = Vec::with_capacity(num_paths);
        let mut sizes = Vec::with_capacity(num_paths);
        for dbpath in val {
            let dbpath_str = dbpath.0.as_ref().to_str();
            cpaths.push(dbpath_str.unwrap().as_ptr() as _);
            cpath_lens.push(dbpath_str.unwrap().len());
            sizes.push(dbpath.1);
        }

        unsafe {
            crocksdb_ffi::crocksdb_options_set_db_paths(
                self.inner,
                cpaths.as_ptr(),
                cpath_lens.as_ptr(),
                sizes.as_ptr(),
                num_paths as c_int,
            );
        }
    }
}

pub struct ColumnFamilyOptions {
    pub inner: *mut Options,
    filter: Option<CompactionFilterHandle>,
}

impl Drop for ColumnFamilyOptions {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_options_destroy(self.inner);
        }
    }
}

impl Default for ColumnFamilyOptions {
    fn default() -> ColumnFamilyOptions {
        unsafe {
            let opts = crocksdb_ffi::crocksdb_options_create();
            assert!(
                !opts.is_null(),
                "Could not create rocksdb column family options"
            );
            ColumnFamilyOptions {
                inner: opts,
                filter: None,
            }
        }
    }
}

impl Clone for ColumnFamilyOptions {
    fn clone(&self) -> Self {
        assert!(self.filter.is_none());
        unsafe {
            let opts = crocksdb_ffi::crocksdb_options_copy(self.inner);
            assert!(!opts.is_null());
            ColumnFamilyOptions {
                inner: opts,
                filter: None,
            }
        }
    }
}

impl ColumnFamilyOptions {
    pub fn new() -> ColumnFamilyOptions {
        ColumnFamilyOptions::default()
    }

    pub unsafe fn from_raw(inner: *mut Options) -> ColumnFamilyOptions {
        assert!(
            !inner.is_null(),
            "could not new rocksdb options with null inner"
        );
        ColumnFamilyOptions {
            inner: inner,
            filter: None,
        }
    }

    pub fn optimize_level_style_compaction(&mut self, memtable_memory_budget: i32) {
        unsafe {
            crocksdb_ffi::crocksdb_options_optimize_level_style_compaction(
                self.inner,
                memtable_memory_budget,
            );
        }
    }

    /// Set compaction filter.
    ///
    /// filter will be dropped when this option is dropped or a new filter is
    /// set.
    ///
    /// By default, compaction will only pass keys written after the most
    /// recent call to GetSnapshot() to filter. However, if `ignore_snapshots`
    /// is set to true, even if the keys were written before the last snapshot
    /// will be passed to filter too. For more details please checkout
    /// rocksdb's documentation.
    ///
    /// See also `CompactionFilter`.
    pub fn set_compaction_filter<S>(
        &mut self,
        name: S,
        ignore_snapshots: bool,
        filter: Box<CompactionFilter>,
    ) -> Result<(), String>
    where
        S: Into<Vec<u8>>,
    {
        unsafe {
            let c_name = match CString::new(name) {
                Ok(s) => s,
                Err(e) => return Err(format!("failed to convert to cstring: {:?}", e)),
            };
            self.filter = Some(try!(
                new_compaction_filter(c_name, ignore_snapshots, filter)
            ));
            crocksdb_ffi::crocksdb_options_set_compaction_filter(
                self.inner,
                self.filter.as_ref().unwrap().inner,
            );
            Ok(())
        }
    }

    pub fn add_table_properties_collector_factory(
        &mut self,
        fname: &str,
        factory: Box<TablePropertiesCollectorFactory>,
    ) {
        unsafe {
            let f = new_table_properties_collector_factory(fname, factory);
            crocksdb_ffi::crocksdb_options_add_table_properties_collector_factory(self.inner, f);
        }
    }


    pub fn compression(&mut self, t: DBCompressionType) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_compression(self.inner, t);
        }
    }

    pub fn get_compression(&self) -> DBCompressionType {
        unsafe { crocksdb_ffi::crocksdb_options_get_compression(self.inner) }
    }

    pub fn compression_per_level(&mut self, level_types: &[DBCompressionType]) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_compression_per_level(
                self.inner,
                level_types.as_ptr(),
                level_types.len() as size_t,
            )
        }
    }

    pub fn get_compression_per_level(&self) -> Vec<DBCompressionType> {
        unsafe {
            let size =
                crocksdb_ffi::crocksdb_options_get_compression_level_number(self.inner) as usize;
            let mut ret = Vec::with_capacity(size);
            let pret = ret.as_mut_ptr();
            crocksdb_ffi::crocksdb_options_get_compression_per_level(self.inner, pret);
            ret.set_len(size);
            ret
        }
    }

    pub fn bottommost_compression(&self, c: DBCompressionType) {
        unsafe { crocksdb_ffi::crocksdb_set_bottommost_compression(self.inner, c) }
    }

    pub fn add_merge_operator(&mut self, name: &str, merge_fn: MergeFn) {
        let cb = Box::new(MergeOperatorCallback {
            name: CString::new(name.as_bytes()).unwrap(),
            merge_fn: merge_fn,
        });

        unsafe {
            let mo = crocksdb_ffi::crocksdb_mergeoperator_create(
                mem::transmute(cb),
                merge_operator::destructor_callback,
                full_merge_callback,
                partial_merge_callback,
                None,
                merge_operator::name_callback,
            );
            crocksdb_ffi::crocksdb_options_set_merge_operator(self.inner, mo);
        }
    }

    pub fn add_comparator(&mut self, name: &str, compare_fn: fn(&[u8], &[u8]) -> i32) {
        let cb = Box::new(ComparatorCallback {
            name: CString::new(name.as_bytes()).unwrap(),
            f: compare_fn,
        });

        unsafe {
            let cmp = crocksdb_ffi::crocksdb_comparator_create(
                mem::transmute(cb),
                comparator::destructor_callback,
                compare_callback,
                comparator::name_callback,
            );
            crocksdb_ffi::crocksdb_options_set_comparator(self.inner, cmp);
        }
    }


    pub fn set_block_cache_size_mb(&mut self, cache_size: u64) {
        unsafe {
            crocksdb_ffi::crocksdb_options_optimize_for_point_lookup(self.inner, cache_size);
        }
    }

    pub fn set_min_write_buffer_number(&mut self, nbuf: c_int) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_min_write_buffer_number_to_merge(self.inner, nbuf);
        }
    }

    pub fn set_max_write_buffer_number(&mut self, nbuf: c_int) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_max_write_buffer_number(self.inner, nbuf);
        }
    }

    pub fn set_write_buffer_size(&mut self, size: u64) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_write_buffer_size(self.inner, size);
        }
    }

    pub fn set_max_bytes_for_level_base(&mut self, size: u64) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_max_bytes_for_level_base(self.inner, size);
        }
    }

    pub fn set_max_bytes_for_level_multiplier(&mut self, mul: i32) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_max_bytes_for_level_multiplier(self.inner, mul);
        }
    }

    pub fn set_max_compaction_bytes(&mut self, bytes: u64) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_max_compaction_bytes(self.inner, bytes);
        }
    }

    pub fn set_level_compaction_dynamic_level_bytes(&mut self, v: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_level_compaction_dynamic_level_bytes(self.inner, v);
        }
    }

    pub fn set_soft_pending_compaction_bytes_limit(&mut self, size: u64) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_soft_pending_compaction_bytes_limit(
                self.inner,
                size,
            );
        }
    }

    pub fn set_hard_pending_compaction_bytes_limit(&mut self, size: u64) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_hard_pending_compaction_bytes_limit(
                self.inner,
                size,
            );
        }
    }

    pub fn set_target_file_size_base(&mut self, size: u64) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_target_file_size_base(self.inner, size);
        }
    }

    pub fn set_min_write_buffer_number_to_merge(&mut self, to_merge: c_int) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_min_write_buffer_number_to_merge(
                self.inner,
                to_merge,
            );
        }
    }

    pub fn set_level_zero_file_num_compaction_trigger(&mut self, n: c_int) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_level0_file_num_compaction_trigger(self.inner, n);
        }
    }

    pub fn set_level_zero_slowdown_writes_trigger(&mut self, n: c_int) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_level0_slowdown_writes_trigger(self.inner, n);
        }
    }

    pub fn set_level_zero_stop_writes_trigger(&mut self, n: c_int) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_level0_stop_writes_trigger(self.inner, n);
        }
    }

    pub fn set_compaction_style(&mut self, style: crocksdb_ffi::DBCompactionStyle) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_compaction_style(self.inner, style);
        }
    }

    pub fn compaction_priority(&mut self, priority: crocksdb_ffi::CompactionPriority) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_compaction_priority(self.inner, priority);
        }
    }

    pub fn set_disable_auto_compactions(&mut self, disable: bool) {
        unsafe {
            if disable {
                crocksdb_ffi::crocksdb_options_set_disable_auto_compactions(self.inner, 1)
            } else {
                crocksdb_ffi::crocksdb_options_set_disable_auto_compactions(self.inner, 0)
            }
        }
    }

    pub fn set_block_based_table_factory(&mut self, factory: &BlockBasedOptions) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_block_based_table_factory(self.inner, factory.inner);
        }
    }

    pub fn set_report_bg_io_stats(&mut self, enable: bool) {
        unsafe {
            if enable {
                crocksdb_ffi::crocksdb_options_set_report_bg_io_stats(self.inner, 1);
            } else {
                crocksdb_ffi::crocksdb_options_set_report_bg_io_stats(self.inner, 0);
            }
        }
    }

    pub fn set_num_levels(&mut self, n: c_int) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_num_levels(self.inner, n);
        }
    }

    pub fn get_num_levels(&self) -> usize {
        unsafe { crocksdb_ffi::crocksdb_options_get_num_levels(self.inner) as usize }
    }

    pub fn set_prefix_extractor<S>(
        &mut self,
        name: S,
        transform: Box<SliceTransform>,
    ) -> Result<(), String>
    where
        S: Into<Vec<u8>>,
    {
        unsafe {
            let c_name = match CString::new(name) {
                Ok(s) => s,
                Err(e) => return Err(format!("failed to convert to cstring: {:?}", e)),
            };
            let transform = try!(new_slice_transform(c_name, transform));
            crocksdb_ffi::crocksdb_options_set_prefix_extractor(self.inner, transform);
            Ok(())
        }
    }

    pub fn set_optimize_filters_for_hits(&mut self, v: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_optimize_filters_for_hits(self.inner, v);
        }
    }

    pub fn set_memtable_insert_hint_prefix_extractor<S>(
        &mut self,
        name: S,
        transform: Box<SliceTransform>,
    ) -> Result<(), String>
    where
        S: Into<Vec<u8>>,
    {
        unsafe {
            let c_name = match CString::new(name) {
                Ok(s) => s,
                Err(e) => return Err(format!("failed to convert to cstring: {:?}", e)),
            };
            let transform = try!(new_slice_transform(c_name, transform));
            crocksdb_ffi::crocksdb_options_set_memtable_insert_with_hint_prefix_extractor(
                self.inner,
                transform,
            );
            Ok(())
        }
    }

    pub fn set_memtable_prefix_bloom_size_ratio(&mut self, ratio: f64) {
        unsafe {
            crocksdb_ffi::crocksdb_options_set_memtable_prefix_bloom_size_ratio(self.inner, ratio);
        }
    }

    pub fn get_block_cache_usage(&self) -> u64 {
        unsafe { crocksdb_ffi::crocksdb_options_get_block_cache_usage(self.inner) as u64 }
    }
}

// ColumnFamilyDescriptor is a pair of column family's name and options.
pub struct ColumnFamilyDescriptor<'a> {
    pub name: &'a str,
    pub options: ColumnFamilyOptions,
}

impl<'a> ColumnFamilyDescriptor<'a> {
    const DEFAULT_COLUMN_FAMILY: &'static str = "default";

    pub fn new(name: &'a str, options: ColumnFamilyOptions) -> Self {
        ColumnFamilyDescriptor { name, options }
    }

    pub fn is_default(&self) -> bool {
        self.name == Self::DEFAULT_COLUMN_FAMILY
    }
}

impl Default for ColumnFamilyDescriptor<'static> {
    fn default() -> Self {
        let name = Self::DEFAULT_COLUMN_FAMILY;
        let options = ColumnFamilyOptions::new();
        ColumnFamilyDescriptor::new(name, options)
    }
}

impl<'a> From<&'a str> for ColumnFamilyDescriptor<'a> {
    fn from(name: &'a str) -> Self {
        let options = ColumnFamilyOptions::new();
        ColumnFamilyDescriptor::new(name, options)
    }
}

impl<'a> From<(&'a str, ColumnFamilyOptions)> for ColumnFamilyDescriptor<'a> {
    fn from(tuple: (&'a str, ColumnFamilyOptions)) -> Self {
        let (name, options) = tuple;
        ColumnFamilyDescriptor::new(name, options)
    }
}

pub struct FlushOptions {
    pub inner: *mut DBFlushOptions,
}

impl FlushOptions {
    pub fn new() -> FlushOptions {
        unsafe {
            FlushOptions {
                inner: crocksdb_ffi::crocksdb_flushoptions_create(),
            }
        }
    }

    pub fn set_wait(&mut self, wait: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_flushoptions_set_wait(self.inner, wait);
        }
    }
}

impl Drop for FlushOptions {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_flushoptions_destroy(self.inner);
        }
    }
}

/// IngestExternalFileOptions is used by DB::ingest_external_file
pub struct IngestExternalFileOptions {
    pub inner: *mut crocksdb_ffi::IngestExternalFileOptions,
}

impl IngestExternalFileOptions {
    pub fn new() -> IngestExternalFileOptions {
        unsafe {
            IngestExternalFileOptions {
                inner: crocksdb_ffi::crocksdb_ingestexternalfileoptions_create(),
            }
        }
    }

    /// If set to false, an ingested file keys could appear in existing snapshots
    /// that where created before the file was ingested.
    pub fn snapshot_consistent(&mut self, whether_consistent: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_ingestexternalfileoptions_set_snapshot_consistency(
                self.inner,
                whether_consistent,
            );
        }
    }

    /// If set to false, DB::ingest_external_file() will fail if the file key range
    /// overlaps with existing keys or tombstones in the DB.
    pub fn allow_global_seqno(&mut self, whether_allow: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_ingestexternalfileoptions_set_allow_global_seqno(
                self.inner,
                whether_allow,
            );
        }
    }

    /// If set to false and the file key range overlaps with the memtable key range
    /// (memtable flush required), DB::ingest_external_file will fail.
    pub fn allow_blocking_flush(&mut self, whether_allow: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_ingestexternalfileoptions_set_allow_blocking_flush(
                self.inner,
                whether_allow,
            );
        }
    }

    /// Set to true to move the files instead of copying them.
    pub fn move_files(&mut self, whether_move: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_ingestexternalfileoptions_set_move_files(
                self.inner,
                whether_move,
            );
        }
    }
}

impl Drop for IngestExternalFileOptions {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_ingestexternalfileoptions_destroy(self.inner);
        }
    }
}

/// Options while opening a file to read/write
pub struct EnvOptions {
    pub inner: *mut crocksdb_ffi::EnvOptions,
}

impl EnvOptions {
    pub fn new() -> EnvOptions {
        unsafe {
            EnvOptions {
                inner: crocksdb_ffi::crocksdb_envoptions_create(),
            }
        }
    }
}

impl Drop for EnvOptions {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_envoptions_destroy(self.inner);
        }
    }
}

pub struct RestoreOptions {
    pub inner: *mut DBRestoreOptions,
}

impl RestoreOptions {
    pub fn new() -> RestoreOptions {
        unsafe {
            RestoreOptions {
                inner: crocksdb_ffi::crocksdb_restore_options_create(),
            }
        }
    }

    pub fn set_keep_log_files(&mut self, flag: bool) {
        unsafe {
            crocksdb_ffi::crocksdb_restore_options_set_keep_log_files(
                self.inner,
                if flag { 1 } else { 0 },
            )
        }
    }
}

impl Drop for RestoreOptions {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_restore_options_destroy(self.inner);
        }
    }
}
