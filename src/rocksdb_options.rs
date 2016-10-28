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
extern crate libc;
use self::libc::{c_int, size_t, c_void};
use std::ffi::{CStr, CString};
use std::mem;

use rocksdb_ffi::{self, DBCompressionType, DBRecoveryMode};
use merge_operator::{self, MergeFn, MergeOperatorCallback,
                     full_merge_callback, partial_merge_callback};
use comparator::{self, ComparatorCallback, compare_callback};

pub struct BlockBasedOptions {
    inner: rocksdb_ffi::DBBlockBasedTableOptions,
}

pub struct Options {
    pub inner: rocksdb_ffi::DBOptions,
}

pub struct WriteOptions {
    pub inner: rocksdb_ffi::DBWriteOptions,
}

impl Drop for Options {
    fn drop(&mut self) {
        unsafe {
            rocksdb_ffi::rocksdb_options_destroy(self.inner);
        }
    }
}

impl Drop for BlockBasedOptions {
    fn drop(&mut self) {
        unsafe {
            rocksdb_ffi::rocksdb_block_based_options_destroy(self.inner);
        }
    }
}

impl Drop for WriteOptions {
    fn drop(&mut self) {
        unsafe {
            rocksdb_ffi::rocksdb_writeoptions_destroy(self.inner);
        }
    }
}

impl BlockBasedOptions {
    pub fn set_block_size(&mut self, size: usize) {
        unsafe {
            rocksdb_ffi::rocksdb_block_based_options_set_block_size(self.inner,
                                                                    size);
        }
    }

    pub fn set_lru_cache(&mut self, size: size_t) {
        let cache = rocksdb_ffi::new_cache(size);
        unsafe {
            // because cache is wrapped in shared_ptr, so we don't need to call
            // rocksdb_cache_destroy explicitly.
            rocksdb_ffi::rocksdb_block_based_options_set_block_cache(self.inner, cache);
        }
    }

    pub fn set_bloom_filter(&mut self, bits_per_key: c_int, block_based: bool) {
        unsafe {
            let bloom = if block_based {
                rocksdb_ffi::rocksdb_filterpolicy_create_bloom(bits_per_key)
            } else {
                rocksdb_ffi::rocksdb_filterpolicy_create_bloom_full(bits_per_key)
            };

            rocksdb_ffi::rocksdb_block_based_options_set_filter_policy(self.inner,
                                                                       bloom);
        }
    }

    pub fn set_cache_index_and_filter_blocks(&mut self, v: bool) {
        unsafe {
            rocksdb_ffi::rocksdb_block_based_options_set_cache_index_and_filter_blocks(self.inner,
                                                                                       v as u8);
        }
    }
}

impl Default for BlockBasedOptions {
    fn default() -> BlockBasedOptions {
        let block_opts = unsafe {
            rocksdb_ffi::rocksdb_block_based_options_create()
        };
        if block_opts.is_null() {
            panic!("Could not create rocksdb block based options".to_string());
        }
        BlockBasedOptions { inner: block_opts }
    }
}

impl Options {
    /// By default, RocksDB uses only one background thread for flush and
    /// compaction. Calling this function will set it up such that total of
    /// `total_threads` is used. Good value for `total_threads` is the number of
    /// cores. You almost definitely want to call this function if your system is
    /// bottlenecked by RocksDB.
    pub fn increase_parallelism(&mut self, parallelism: i32) {
        unsafe {
            rocksdb_ffi::rocksdb_options_increase_parallelism(self.inner,
                                                              parallelism);
        }
    }

    pub fn optimize_level_style_compaction(&mut self,
                                           memtable_memory_budget: i32) {
        unsafe {
            rocksdb_ffi::rocksdb_options_optimize_level_style_compaction(
                self.inner, memtable_memory_budget);
        }
    }

    /// If true, the database will be created if it is missing.
    ///
    /// Default: false
    pub fn create_if_missing(&mut self, create_if_missing: bool) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_create_if_missing(
                self.inner, create_if_missing);
        }
    }

    /// Sets the compression algorithm that will be used for the bottommost level that
    /// contain files. If level-compaction is used, this option will only affect
    /// levels after base level.
    ///
    /// Default: DBCompressionType::None
    pub fn compression(&mut self, t: DBCompressionType) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_compression(self.inner, t);
        }
    }

    /// Different levels can have different compression policies. There
    /// are cases where most lower levels would like to use quick compression
    /// algorithms while the higher levels (which have more data) use
    /// compression algorithms that have better compression but could
    /// be slower. This array, if non-empty, should have an entry for
    /// each level of the database; these override the value specified in
    /// the previous field 'compression'.
    pub fn compression_per_level(&mut self, level_types: &[DBCompressionType]) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_compression_per_level(self.inner,
                                                                level_types.as_ptr(),
                                                                level_types.len() as size_t)
        }
    }

    pub fn add_merge_operator(&mut self,
                              name: &str,
                              merge_fn: MergeFn) {
        let cb = Box::new(MergeOperatorCallback {
            name: CString::new(name.as_bytes()).unwrap(),
            merge_fn: merge_fn,
        });

        unsafe {
            let mo = rocksdb_ffi::rocksdb_mergeoperator_create(
                mem::transmute(cb),
                merge_operator::destructor_callback,
                full_merge_callback,
                partial_merge_callback,
                None,
                merge_operator::name_callback);
            rocksdb_ffi::rocksdb_options_set_merge_operator(self.inner, mo);
        }
    }

    /// Sets the comparator used to define the order of keys in the table.
    /// Default: a comparator that uses lexicographic byte-wise ordering
    ///
    /// The client must ensure that the comparator supplied here has the
    //// same name and orders keys *exactly* the same as the comparator
    /// provided to previous open calls on the same DB.
    pub fn add_comparator(&mut self,
                          name: &str,
                          compare_fn: fn(&[u8], &[u8]) -> i32) {
        let cb = Box::new(ComparatorCallback {
            name: CString::new(name.as_bytes()).unwrap(),
            f: compare_fn,
        });

        unsafe {
            let cmp = rocksdb_ffi::rocksdb_comparator_create(
                mem::transmute(cb),
                comparator::destructor_callback,
                compare_callback,
                comparator::name_callback);
            rocksdb_ffi::rocksdb_options_set_comparator(self.inner, cmp);
        }
    }

    pub fn set_block_cache_size_mb(&mut self, cache_size: u64) {
        unsafe {
            rocksdb_ffi::rocksdb_options_optimize_for_point_lookup(self.inner,
                                                                   cache_size);
        }
    }

    /// Sets the number of open files that can be used by the DB. You may need to
    /// increase this if your database has a large working set. Value -1 means
    /// files opened are always kept open. You can estimate number of files based
    /// on target_file_size_base and target_file_size_multiplier for level-based
    /// compaction. For universal-style compaction, you can usually set it to -1.
    ///
    /// Default: -1
    pub fn set_max_open_files(&mut self, nfiles: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_max_open_files(self.inner, nfiles);
        }
    }

    /// If true, then every store to stable storage will issue a fsync.
    /// If false, then every store to stable storage will issue a fdatasync.
    /// This parameter should be set to true while storing data to
    /// filesystem like ext3 that can lose files after a reboot.
    /// Default: false
    pub fn set_use_fsync(&mut self, useit: bool) {
        unsafe {
            if useit {
                rocksdb_ffi::rocksdb_options_set_use_fsync(self.inner, 1)
            } else {
                rocksdb_ffi::rocksdb_options_set_use_fsync(self.inner, 0)
            }
        }
    }

    /// Allows OS to incrementally sync files to disk while they are being
    /// written, asynchronously, in the background. This operation can be used
    /// to smooth out write I/Os over time. Users shouldn't rely on it for
    /// persistency guarantee.
    /// Issue one request for every bytes_per_sync written. 0 turns it off.
    ///
    /// Default: 0
    ///
    /// You may consider using rate_limiter to regulate write rate to device.
    /// When rate limiter is enabled, it automatically enables bytes_per_sync
    /// to 1MB.
    ///
    /// This option applies to table files
    pub fn set_bytes_per_sync(&mut self, nbytes: u64) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_bytes_per_sync(self.inner, nbytes);
        }
    }

    pub fn set_disable_data_sync(&mut self, disable: bool) {
        unsafe {
            if disable {
                rocksdb_ffi::rocksdb_options_set_disable_data_sync(self.inner, 1)
            } else {
                rocksdb_ffi::rocksdb_options_set_disable_data_sync(self.inner, 0)
            }
        }
    }

    /// Hints to the OS that it should not buffer disk I/O. Enabling this
    /// parameter may improve performance but increases pressure on the
    /// system cache.
    ///
    /// The exact behavior of this parameter is platform dependent.
    ///
    /// On POSIX systems, after RocksDB reads data from disk it will
    /// mark the pages as "unneeded". The operating system may - or may not
    /// - evict these pages from memory, reducing pressure on the system
    /// cache. If the disk block is requested again this can result in
    /// additional disk I/O.
    ///
    /// On WINDOWS system, files will be opened in "unbuffered I/O" mode
    /// which means that data read from the disk will not be cached or
    /// bufferized. The hardware buffer of the devices may however still
    /// be used. Memory mapped files are not impacted by this parameter.
    ///
    /// Default: true
    pub fn allow_os_buffer(&mut self, is_allow: bool) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_allow_os_buffer(self.inner,
                                                             is_allow);
        }
    }

    /// Sets the number of shards used for table cache.
    pub fn set_table_cache_num_shard_bits(&mut self, nbits: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_table_cache_numshardbits(self.inner,
                                                                      nbits);
        }
    }

    /// Sets the minimum number of write buffers that will be merged together
    /// before writing to storage.  If set to 1, then
    /// all write buffers are flushed to L0 as individual files and this increases
    /// read amplification because a get request has to check in all of these
    /// files. Also, an in-memory merge may result in writing lesser
    /// data to storage if there are duplicate records in each of these
    /// individual write buffers.
    ///
    /// Default: 1
    pub fn set_min_write_buffer_number(&mut self, nbuf: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_min_write_buffer_number_to_merge(
                self.inner, nbuf);
        }
    }

    /// Sets the total maximum number of write buffers to maintain in memory including
    /// copies of buffers that have already been flushed.  Unlike
    /// max_write_buffer_number, this parameter does not affect flushing.
    /// This controls the minimum amount of write history that will be available
    /// in memory for conflict checking when Transactions are used.
    ///
    /// When using an OptimisticTransactionDB:
    /// If this value is too low, some transactions may fail at commit time due
    /// to not being able to determine whether there were any write conflicts.
    ///
    /// When using a TransactionDB:
    /// If Transaction::SetSnapshot is used, TransactionDB will read either
    /// in-memory write buffers or SST files to do write-conflict checking.
    /// Increasing this value can reduce the number of reads to SST files
    /// done for conflict detection.
    ///
    /// Setting this value to 0 will cause write buffers to be freed immediately
    /// after they are flushed.
    /// If this value is set to -1, 'max_write_buffer_number' will be used.
    ///
    /// Default:
    /// If using a TransactionDB/OptimisticTransactionDB, the default value will
    /// be set to the value of 'max_write_buffer_number' if it is not explicitly
    /// set by the user.  Otherwise, the default is 0.
    pub fn set_max_write_buffer_number(&mut self, nbuf: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_max_write_buffer_number(self.inner,
                                                                     nbuf);
        }
    }

    /// Sets the amount of data to build up in memory (backed by an unsorted log
    /// on disk) before converting to a sorted on-disk file.
    ///
    /// Larger values increase performance, especially during bulk loads.
    /// Up to max_write_buffer_number write buffers may be held in memory
    /// at the same time,
    /// so you may wish to adjust this parameter to control memory usage.
    /// Also, a larger write buffer will result in a longer recovery time
    /// the next time the database is opened.
    ///
    /// Note that write_buffer_size is enforced per column family.
    /// See db_write_buffer_size for sharing memory across column families.
    ///
    /// Default: 64MB
    ///
    /// Dynamically changeable through SetOptions() API
    pub fn set_write_buffer_size(&mut self, size: usize) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_write_buffer_size(self.inner,
                                                               size);
        }
    }

    /// Control maximum total data size for a level.
    /// max_bytes_for_level_base is the max total for level-1.
    /// Maximum number of bytes for level L can be calculated as
    /// (max_bytes_for_level_base) * (max_bytes_for_level_multiplier ^ (L-1))
    /// For example, if max_bytes_for_level_base is 200MB, and if
    /// max_bytes_for_level_multiplier is 10, total data size for level-1
    /// will be 200MB, total file size for level-2 will be 2GB,
    /// and total file size for level-3 will be 20GB.
    ///
    /// Default: 256MB.
    ///
    /// Dynamically changeable through SetOptions() API
    pub fn set_max_bytes_for_level_base(&mut self, size: u64) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_max_bytes_for_level_base(self.inner, size);
        }
    }

    /// Default: 10
    pub fn set_max_bytes_for_level_multiplier(&mut self, mul: i32) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_max_bytes_for_level_multiplier(self.inner, mul);
        }
    }

    /// The manifest file is rolled over on reaching this limit.
    /// The older manifest file be deleted.
    /// The default value is MAX_INT so that roll-over does not take place.
    pub fn set_max_manifest_file_size(&mut self, size: usize) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_max_manifest_file_size(self.inner, size);
        }
    }

    /// Sets the target file size for compaction.
    /// target_file_size_base is per-file size for level-1.
    /// Target file size for level L can be calculated by
    /// target_file_size_base * (target_file_size_multiplier ^ (L-1))
    /// For example, if target_file_size_base is 2MB and
    /// target_file_size_multiplier is 10, then each file on level-1 will
    /// be 2MB, and each file on level 2 will be 20MB,
    /// and each file on level-3 will be 200MB.
    ///
    /// Default: 64MB.
    ///
    /// Dynamically changeable through SetOptions() API
    pub fn set_target_file_size_base(&mut self, size: u64) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_target_file_size_base(self.inner,
                                                                   size);
        }
    }

    /// Sets the minimum number of write buffers that will be merged together
    /// before writing to storage.  If set to 1, then
    /// all write buffers are flushed to L0 as individual files and this increases
    /// read amplification because a get request has to check in all of these
    /// files. Also, an in-memory merge may result in writing lesser
    /// data to storage if there are duplicate records in each of these
    /// individual write buffers.
    ///
    /// Default: 1
    pub fn set_min_write_buffer_number_to_merge(&mut self, to_merge: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_min_write_buffer_number_to_merge(
                self.inner, to_merge);
        }
    }

    /// Sets the number of files to trigger level-0 compaction. A value <0 means that
    /// level-0 compaction will not be triggered by number of files at all.
    ///
    /// Default: 4
    ///
    /// Dynamically changeable through SetOptions() API
    pub fn set_level_zero_file_num_compaction_trigger(&mut self, n: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_level0_file_num_compaction_trigger(
                self.inner, n);
        }
    }

    /// Sets the soft limit on number of level-0 files. We start slowing down writes at this
    /// point. A value <0 means that no writing slow down will be triggered by
    /// number of files in level-0.
    ///
    /// Dynamically changeable through SetOptions() API
    pub fn set_level_zero_slowdown_writes_trigger(&mut self, n: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_level0_slowdown_writes_trigger(
                self.inner, n);
        }
    }

    /// Sets the maximum number of level-0 files.  We stop writes at this point.
    ///
    /// Dynamically changeable through SetOptions() API
    pub fn set_level_zero_stop_writes_trigger(&mut self, n: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_level0_stop_writes_trigger(
                self.inner, n);
        }
    }

    /// Sets the compaction style.
    ///
    /// Default: DBCompactionStyle::Level
    pub fn set_compaction_style(&mut self,
                                style: rocksdb_ffi::DBCompactionStyle) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_compaction_style(self.inner,
                                                              style);
        }
    }


    /// Sets the maximum number of concurrent background compaction jobs, submitted to
    /// the default LOW priority thread pool.
    /// We first try to schedule compactions based on
    /// `base_background_compactions`. If the compaction cannot catch up , we
    /// will increase number of compaction threads up to
    /// `max_background_compactions`.
    ///
    /// If you're increasing this, also consider increasing number of threads in
    /// LOW priority thread pool. For more information, see
    /// Env::SetBackgroundThreads
    ///
    /// Default: 1
    pub fn set_max_background_compactions(&mut self, n: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_max_background_compactions(
                self.inner, n);
        }
    }

    /// Sets the maximum number of concurrent background memtable flush jobs, submitted to
    /// the HIGH priority thread pool.
    ///
    /// By default, all background jobs (major compaction and memtable flush) go
    /// to the LOW priority pool. If this option is set to a positive number,
    /// memtable flush jobs will be submitted to the HIGH priority pool.
    /// It is important when the same Env is shared by multiple db instances.
    /// Without a separate pool, long running major compaction jobs could
    /// potentially block memtable flush jobs of other db instances, leading to
    /// unnecessary Put stalls.
    ///
    /// If you're increasing this, also consider increasing number of threads in
    /// HIGH priority thread pool. For more information, see
    /// Env::SetBackgroundThreads
    ///
    /// Default: 1
    pub fn set_max_background_flushes(&mut self, n: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_max_background_flushes(self.inner,
                                                                    n);
        }
    }

    /// Disables automatic compactions. Manual compactions can still
    /// be issued on this column family
    ///
    /// Dynamically changeable through SetOptions() API
    pub fn set_disable_auto_compactions(&mut self, disable: bool) {
        let c_bool = if disable {
            1
        } else {
            0
        };
        unsafe {
            rocksdb_ffi::rocksdb_options_set_disable_auto_compactions(self.inner, c_bool)
        }
    }

    pub fn set_block_based_table_factory(&mut self,
                                         factory: &BlockBasedOptions) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_block_based_table_factory(self.inner, factory.inner);
        }
    }

    /// Measure IO stats in compactions and flushes, if true.
    ///
    /// Default: false
    pub fn set_report_bg_io_stats(&mut self, enable: bool) {
        unsafe {
            if enable {
                rocksdb_ffi::rocksdb_options_set_report_bg_io_stats(self.inner, 1);
            } else {
                rocksdb_ffi::rocksdb_options_set_report_bg_io_stats(self.inner, 0);
            }
        }
    }

    /// Recovery mode to control the consistency while replaying WAL
    ///
    /// Default: DBRecoveryMode::PointInTime
    pub fn set_wal_recovery_mode(&mut self, mode: DBRecoveryMode) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_wal_recovery_mode(self.inner, mode);
        }
    }

    pub fn enable_statistics(&mut self) {
        unsafe {
            rocksdb_ffi::rocksdb_options_enable_statistics(self.inner);
        }
    }

    pub fn get_statistics(&self) -> Option<String> {
        unsafe {
            let value = rocksdb_ffi::rocksdb_options_statistics_get_string(self.inner);


            if value.is_null() {
                return None;
            }

            // Must valid UTF-8 format.
            let s = CStr::from_ptr(value).to_str().unwrap().to_owned();
            libc::free(value as *mut c_void);
            Some(s)
        }
    }

    /// If not zero, dump rocksdb.stats to LOG every stats_dump_period_sec
    ///
    /// Default: 600 (10 min)
    pub fn set_stats_dump_period_sec(&mut self, period: usize) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_stats_dump_period_sec(self.inner,
                                                      period);
        }
    }

    /// Sets the number of levels for this database
    pub fn set_num_levels(&mut self, n: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_num_levels(self.inner, n);
        }
    }
}

impl Default for Options {
    fn default() -> Options {
        unsafe {
            let opts = rocksdb_ffi::rocksdb_options_create();
            if opts.is_null() {
                panic!("Could not create rocksdb options".to_string());
            }
            Options { inner: opts }
        }
    }
}


impl WriteOptions {
    pub fn new() -> WriteOptions {
        WriteOptions::default()
    }

    pub fn set_sync(&mut self, sync: bool) {
        unsafe {
            rocksdb_ffi::rocksdb_writeoptions_set_sync(self.inner, sync);
        }
    }

    pub fn disable_wal(&mut self, disable: bool) {
        unsafe {
            if disable {
                rocksdb_ffi::rocksdb_writeoptions_disable_WAL(self.inner, 1);
            } else {
                rocksdb_ffi::rocksdb_writeoptions_disable_WAL(self.inner, 0);
            }
        }
    }
}

impl Default for WriteOptions {
    fn default() -> WriteOptions {
        let write_opts = unsafe { rocksdb_ffi::rocksdb_writeoptions_create() };
        if write_opts.is_null() {
            panic!("Could not create rocksdb write options".to_string());
        }
        WriteOptions { inner: write_opts }
    }
}

#[cfg(test)]
mod tests {
    use super::Options;

    #[test]
    fn test_set_max_manifest_file_size() {
        let mut opts = Options::default();
        let size = 20 * 1024 * 1024;
        opts.set_max_manifest_file_size(size)
    }

    #[test]
    fn test_enable_statistics() {
        let mut opts = Options::default();
        opts.enable_statistics();
        opts.set_stats_dump_period_sec(60);
        assert!(opts.get_statistics().is_some());

        let opts = Options::default();
        assert!(opts.get_statistics().is_none());
    }
}
