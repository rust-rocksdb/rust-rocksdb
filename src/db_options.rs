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

use std::ffi::{CStr, CString};
use std::mem;
use std::path::Path;

use libc::{self, c_int, c_uchar, c_uint, c_void, size_t, uint64_t};

use compaction_filter::{self, filter_callback, CompactionFilterCallback, CompactionFilterFn};
use comparator::{self, ComparatorCallback, CompareFn};
use ffi;
use merge_operator::{
    self, full_merge_callback, partial_merge_callback, MergeFn, MergeOperatorCallback,
};
use slice_transform::SliceTransform;
use {
    BlockBasedIndexType, BlockBasedOptions, DBCompactionStyle, DBCompressionType, DBRecoveryMode,
    MemtableFactory, Options, PlainTableFactoryOptions, WriteOptions,
};

pub fn new_cache(capacity: size_t) -> *mut ffi::rocksdb_cache_t {
    unsafe { ffi::rocksdb_cache_create_lru(capacity) }
}

unsafe impl Send for Options {}

impl Drop for Options {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_options_destroy(self.inner);
        }
    }
}

impl Drop for BlockBasedOptions {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_block_based_options_destroy(self.inner);
        }
    }
}

impl Drop for WriteOptions {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_writeoptions_destroy(self.inner);
        }
    }
}

impl BlockBasedOptions {
    pub fn set_block_size(&mut self, size: usize) {
        unsafe {
            ffi::rocksdb_block_based_options_set_block_size(self.inner, size);
        }
    }

    pub fn set_lru_cache(&mut self, size: size_t) {
        let cache = new_cache(size);
        unsafe {
            // Since cache is wrapped in shared_ptr, we don't need to
            // call rocksdb_cache_destroy explicitly.
            ffi::rocksdb_block_based_options_set_block_cache(self.inner, cache);
        }
    }

    pub fn disable_cache(&mut self) {
        unsafe {
            ffi::rocksdb_block_based_options_set_no_block_cache(self.inner, true as c_uchar);
        }
    }

    pub fn set_bloom_filter(&mut self, bits_per_key: c_int, block_based: bool) {
        unsafe {
            let bloom = if block_based {
                ffi::rocksdb_filterpolicy_create_bloom(bits_per_key)
            } else {
                ffi::rocksdb_filterpolicy_create_bloom_full(bits_per_key)
            };

            ffi::rocksdb_block_based_options_set_filter_policy(self.inner, bloom);
        }
    }

    pub fn set_cache_index_and_filter_blocks(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_block_based_options_set_cache_index_and_filter_blocks(self.inner, v as u8);
        }
    }

    /// Defines the index type to be used for SS-table lookups.
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::{BlockBasedOptions, BlockBasedIndexType, Options};
    ///
    /// let mut opts = Options::default();
    /// let mut block_opts = BlockBasedOptions::default();
    /// block_opts.set_index_type(BlockBasedIndexType::HashSearch);
    /// ```
    pub fn set_index_type(&mut self, index_type: BlockBasedIndexType) {
        let index = index_type as i32;
        unsafe {
            ffi::rocksdb_block_based_options_set_index_type(self.inner, index);
        }
    }
}

impl Default for BlockBasedOptions {
    fn default() -> BlockBasedOptions {
        let block_opts = unsafe { ffi::rocksdb_block_based_options_create() };
        if block_opts.is_null() {
            panic!("Could not create RocksDB block based options");
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
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.increase_parallelism(3);
    /// ```
    pub fn increase_parallelism(&mut self, parallelism: i32) {
        unsafe {
            ffi::rocksdb_options_increase_parallelism(self.inner, parallelism);
        }
    }

    pub fn optimize_level_style_compaction(&mut self, memtable_memory_budget: usize) {
        unsafe {
            ffi::rocksdb_options_optimize_level_style_compaction(
                self.inner,
                memtable_memory_budget as uint64_t,
            );
        }
    }

    /// If true, the database will be created if it is missing.
    ///
    /// Default: `false`
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.create_if_missing(true);
    /// ```
    pub fn create_if_missing(&mut self, create_if_missing: bool) {
        unsafe {
            ffi::rocksdb_options_set_create_if_missing(self.inner, create_if_missing as c_uchar);
        }
    }

    /// If true, any column families that didn't exist when opening the database
    /// will be created.
    ///
    /// Default: `false`
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.create_missing_column_families(true);
    /// ```
    pub fn create_missing_column_families(&mut self, create_missing_cfs: bool) {
        unsafe {
            ffi::rocksdb_options_set_create_missing_column_families(
                self.inner,
                create_missing_cfs as c_uchar,
            );
        }
    }

    /// Sets the compression algorithm that will be used for the bottommost level that
    /// contain files. If level-compaction is used, this option will only affect
    /// levels after base level.
    ///
    /// Default: DBCompressionType::None
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::{Options, DBCompressionType};
    ///
    /// let mut opts = Options::default();
    /// opts.set_compression_type(DBCompressionType::Snappy);
    /// ```
    pub fn set_compression_type(&mut self, t: DBCompressionType) {
        unsafe {
            ffi::rocksdb_options_set_compression(self.inner, t as c_int);
        }
    }

    /// Different levels can have different compression policies. There
    /// are cases where most lower levels would like to use quick compression
    /// algorithms while the higher levels (which have more data) use
    /// compression algorithms that have better compression but could
    /// be slower. This array, if non-empty, should have an entry for
    /// each level of the database; these override the value specified in
    /// the previous field 'compression'.
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::{Options, DBCompressionType};
    ///
    /// let mut opts = Options::default();
    /// opts.set_compression_per_level(&[
    ///     DBCompressionType::None,
    ///     DBCompressionType::None,
    ///     DBCompressionType::Snappy,
    ///     DBCompressionType::Snappy,
    ///     DBCompressionType::Snappy
    /// ]);
    /// ```
    pub fn set_compression_per_level(&mut self, level_types: &[DBCompressionType]) {
        unsafe {
            let mut level_types: Vec<_> = level_types.iter().map(|&t| t as c_int).collect();
            ffi::rocksdb_options_set_compression_per_level(
                self.inner,
                level_types.as_mut_ptr(),
                level_types.len() as size_t,
            )
        }
    }

    /// If non-zero, we perform bigger reads when doing compaction. If you're
    /// running RocksDB on spinning disks, you should set this to at least 2MB.
    /// That way RocksDB's compaction is doing sequential instead of random reads.
    ///
    /// When non-zero, we also force new_table_reader_for_compaction_inputs to
    /// true.
    ///
    /// Default: `0`
    pub fn set_compaction_readahead_size(&mut self, compaction_readahead_size: usize) {
        unsafe {
            ffi::rocksdb_options_compaction_readahead_size(
                self.inner,
                compaction_readahead_size as usize,
            );
        }
    }

    pub fn set_merge_operator(
        &mut self,
        name: &str,
        full_merge_fn: MergeFn,
        partial_merge_fn: Option<MergeFn>,
    ) {
        let cb = Box::new(MergeOperatorCallback {
            name: CString::new(name.as_bytes()).unwrap(),
            full_merge_fn,
            partial_merge_fn: partial_merge_fn.unwrap_or(full_merge_fn),
        });

        unsafe {
            let mo = ffi::rocksdb_mergeoperator_create(
                mem::transmute(cb),
                Some(merge_operator::destructor_callback),
                Some(full_merge_callback),
                Some(partial_merge_callback),
                None,
                Some(merge_operator::name_callback),
            );
            ffi::rocksdb_options_set_merge_operator(self.inner, mo);
        }
    }

    #[deprecated(
        since = "0.5.0",
        note = "add_merge_operator has been renamed to set_merge_operator"
    )]
    pub fn add_merge_operator(&mut self, name: &str, merge_fn: MergeFn) {
        self.set_merge_operator(name, merge_fn, None);
    }

    /// Sets a compaction filter used to determine if entries should be kept, changed,
    /// or removed during compaction.
    ///
    /// An example use case is to remove entries with an expired TTL.
    ///
    /// If you take a snapshot of the database, only values written since the last
    /// snapshot will be passed through the compaction filter.
    ///
    /// If multi-threaded compaction is used, `filter_fn` may be called multiple times
    /// simultaneously.
    pub fn set_compaction_filter<F>(&mut self, name: &str, filter_fn: F)
    where
        F: CompactionFilterFn + Send + 'static,
    {
        let cb = Box::new(CompactionFilterCallback {
            name: CString::new(name.as_bytes()).unwrap(),
            filter_fn,
        });

        unsafe {
            let cf = ffi::rocksdb_compactionfilter_create(
                mem::transmute(cb),
                Some(compaction_filter::destructor_callback::<F>),
                Some(filter_callback::<F>),
                Some(compaction_filter::name_callback::<F>),
            );
            ffi::rocksdb_options_set_compaction_filter(self.inner, cf);
        }
    }

    /// Sets the comparator used to define the order of keys in the table.
    /// Default: a comparator that uses lexicographic byte-wise ordering
    ///
    /// The client must ensure that the comparator supplied here has the same
    /// name and orders keys *exactly* the same as the comparator provided to
    /// previous open calls on the same DB.
    pub fn set_comparator(&mut self, name: &str, compare_fn: CompareFn) {
        let cb = Box::new(ComparatorCallback {
            name: CString::new(name.as_bytes()).unwrap(),
            f: compare_fn,
        });

        unsafe {
            let cmp = ffi::rocksdb_comparator_create(
                mem::transmute(cb),
                Some(comparator::destructor_callback),
                Some(comparator::compare_callback),
                Some(comparator::name_callback),
            );
            ffi::rocksdb_options_set_comparator(self.inner, cmp);
        }
    }

    pub fn set_prefix_extractor(&mut self, prefix_extractor: SliceTransform) {
        unsafe { ffi::rocksdb_options_set_prefix_extractor(self.inner, prefix_extractor.inner) }
    }

    #[deprecated(
        since = "0.5.0",
        note = "add_comparator has been renamed to set_comparator"
    )]
    pub fn add_comparator(&mut self, name: &str, compare_fn: CompareFn) {
        self.set_comparator(name, compare_fn);
    }

    pub fn optimize_for_point_lookup(&mut self, cache_size: u64) {
        unsafe {
            ffi::rocksdb_options_optimize_for_point_lookup(self.inner, cache_size);
        }
    }

    /// Sets the number of open files that can be used by the DB. You may need to
    /// increase this if your database has a large working set. Value `-1` means
    /// files opened are always kept open. You can estimate number of files based
    /// on target_file_size_base and target_file_size_multiplier for level-based
    /// compaction. For universal-style compaction, you can usually set it to `-1`.
    ///
    /// Default: `-1`
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_max_open_files(10);
    /// ```
    pub fn set_max_open_files(&mut self, nfiles: c_int) {
        unsafe {
            ffi::rocksdb_options_set_max_open_files(self.inner, nfiles);
        }
    }

    /// If true, then every store to stable storage will issue a fsync.
    /// If false, then every store to stable storage will issue a fdatasync.
    /// This parameter should be set to true while storing data to
    /// filesystem like ext3 that can lose files after a reboot.
    ///
    /// Default: `false`
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_use_fsync(true);
    /// ```
    pub fn set_use_fsync(&mut self, useit: bool) {
        unsafe { ffi::rocksdb_options_set_use_fsync(self.inner, useit as c_int) }
    }

    /// Allows OS to incrementally sync files to disk while they are being
    /// written, asynchronously, in the background. This operation can be used
    /// to smooth out write I/Os over time. Users shouldn't rely on it for
    /// persistency guarantee.
    /// Issue one request for every bytes_per_sync written. `0` turns it off.
    ///
    /// Default: `0`
    ///
    /// You may consider using rate_limiter to regulate write rate to device.
    /// When rate limiter is enabled, it automatically enables bytes_per_sync
    /// to 1MB.
    ///
    /// This option applies to table files
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_bytes_per_sync(1024 * 1024);
    /// ```
    pub fn set_bytes_per_sync(&mut self, nbytes: u64) {
        unsafe {
            ffi::rocksdb_options_set_bytes_per_sync(self.inner, nbytes);
        }
    }

    /// If true, allow multi-writers to update mem tables in parallel.
    /// Only some memtable_factory-s support concurrent writes; currently it
    /// is implemented only for SkipListFactory.  Concurrent memtable writes
    /// are not compatible with inplace_update_support or filter_deletes.
    /// It is strongly recommended to set enable_write_thread_adaptive_yield
    /// if you are going to use this feature.
    ///
    /// Default: true
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_allow_concurrent_memtable_write(false);
    /// ```
    pub fn set_allow_concurrent_memtable_write(&mut self, allow: bool) {
        unsafe {
            ffi::rocksdb_options_set_allow_concurrent_memtable_write(self.inner, allow as c_uchar)
        }
    }

    /// Enable direct I/O mode for reading
    /// they may or may not improve performance depending on the use case
    ///
    /// Files will be opened in "direct I/O" mode
    /// which means that data read from the disk will not be cached or
    /// buffered. The hardware buffer of the devices may however still
    /// be used. Memory mapped files are not impacted by these parameters.
    ///
    /// Default: false
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_use_direct_reads(true);
    /// ```
    pub fn set_use_direct_reads(&mut self, enabled: bool) {
        unsafe {
            ffi::rocksdb_options_set_use_direct_reads(self.inner, enabled as c_uchar);
        }
    }

    /// Enable direct I/O mode for flush and compaction
    ///
    /// Files will be opened in "direct I/O" mode
    /// which means that data written to the disk will not be cached or
    /// buffered. The hardware buffer of the devices may however still
    /// be used. Memory mapped files are not impacted by these parameters.
    /// they may or may not improve performance depending on the use case
    ///
    /// Default: false
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_use_direct_io_for_flush_and_compaction(true);
    /// ```
    pub fn set_use_direct_io_for_flush_and_compaction(&mut self, enabled: bool) {
        unsafe {
            ffi::rocksdb_options_set_use_direct_io_for_flush_and_compaction(
                self.inner,
                enabled as c_uchar,
            );
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
    /// On WINDOWS systems, files will be opened in "unbuffered I/O" mode
    /// which means that data read from the disk will not be cached or
    /// bufferized. The hardware buffer of the devices may however still
    /// be used. Memory mapped files are not impacted by this parameter.
    ///
    /// Default: true
    ///
    /// # Example
    ///
    /// ```
    /// #[allow(deprecated)]
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_allow_os_buffer(false);
    /// ```
    #[deprecated(
        since = "0.7.0",
        note = "replaced with set_use_direct_reads/set_use_direct_io_for_flush_and_compaction methods"
    )]
    pub fn set_allow_os_buffer(&mut self, is_allow: bool) {
        self.set_use_direct_reads(!is_allow);
        self.set_use_direct_io_for_flush_and_compaction(!is_allow);
    }

    /// Sets the number of shards used for table cache.
    ///
    /// Default: `6`
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_table_cache_num_shard_bits(4);
    /// ```
    pub fn set_table_cache_num_shard_bits(&mut self, nbits: c_int) {
        unsafe {
            ffi::rocksdb_options_set_table_cache_numshardbits(self.inner, nbits);
        }
    }

    /// Sets the minimum number of write buffers that will be merged together
    /// before writing to storage.  If set to `1`, then
    /// all write buffers are flushed to L0 as individual files and this increases
    /// read amplification because a get request has to check in all of these
    /// files. Also, an in-memory merge may result in writing lesser
    /// data to storage if there are duplicate records in each of these
    /// individual write buffers.
    ///
    /// Default: `1`
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_min_write_buffer_number(2);
    /// ```
    pub fn set_min_write_buffer_number(&mut self, nbuf: c_int) {
        unsafe {
            ffi::rocksdb_options_set_min_write_buffer_number_to_merge(self.inner, nbuf);
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
    /// Setting this value to `0` will cause write buffers to be freed immediately
    /// after they are flushed.
    /// If this value is set to `-1`, 'max_write_buffer_number' will be used.
    ///
    /// Default:
    /// If using a TransactionDB/OptimisticTransactionDB, the default value will
    /// be set to the value of 'max_write_buffer_number' if it is not explicitly
    /// set by the user.  Otherwise, the default is 0.
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_min_write_buffer_number(4);
    /// ```
    pub fn set_max_write_buffer_number(&mut self, nbuf: c_int) {
        unsafe {
            ffi::rocksdb_options_set_max_write_buffer_number(self.inner, nbuf);
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
    /// Default: `0x4000000` (64MiB)
    ///
    /// Dynamically changeable through SetOptions() API
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_write_buffer_size(128 * 1024 * 1024);
    /// ```
    pub fn set_write_buffer_size(&mut self, size: usize) {
        unsafe {
            ffi::rocksdb_options_set_write_buffer_size(self.inner, size);
        }
    }

    /// Amount of data to build up in memtables across all column
    /// families before writing to disk.
    ///
    /// This is distinct from write_buffer_size, which enforces a limit
    /// for a single memtable.
    ///
    /// This feature is disabled by default. Specify a non-zero value
    /// to enable it.
    ///
    /// Default: 0 (disabled)
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_db_write_buffer_size(128 * 1024 * 1024);
    /// ```
    pub fn set_db_write_buffer_size(&mut self, size: usize) {
        unsafe {
            ffi::rocksdb_options_set_db_write_buffer_size(self.inner, size);
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
    /// Default: `0x10000000` (256MiB).
    ///
    /// Dynamically changeable through SetOptions() API
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_max_bytes_for_level_base(512 * 1024 * 1024);
    /// ```
    pub fn set_max_bytes_for_level_base(&mut self, size: u64) {
        unsafe {
            ffi::rocksdb_options_set_max_bytes_for_level_base(self.inner, size);
        }
    }

    /// Default: `10`
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_max_bytes_for_level_multiplier(4.0);
    /// ```
    pub fn set_max_bytes_for_level_multiplier(&mut self, mul: f64) {
        unsafe {
            ffi::rocksdb_options_set_max_bytes_for_level_multiplier(self.inner, mul);
        }
    }

    /// The manifest file is rolled over on reaching this limit.
    /// The older manifest file be deleted.
    /// The default value is MAX_INT so that roll-over does not take place.
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_max_manifest_file_size(20 * 1024 * 1024);
    /// ```
    pub fn set_max_manifest_file_size(&mut self, size: usize) {
        unsafe {
            ffi::rocksdb_options_set_max_manifest_file_size(self.inner, size);
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
    /// Default: `0x4000000` (64MiB)
    ///
    /// Dynamically changeable through SetOptions() API
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_target_file_size_base(128 * 1024 * 1024);
    /// ```
    pub fn set_target_file_size_base(&mut self, size: u64) {
        unsafe {
            ffi::rocksdb_options_set_target_file_size_base(self.inner, size);
        }
    }

    /// Sets the minimum number of write buffers that will be merged together
    /// before writing to storage.  If set to `1`, then
    /// all write buffers are flushed to L0 as individual files and this increases
    /// read amplification because a get request has to check in all of these
    /// files. Also, an in-memory merge may result in writing lesser
    /// data to storage if there are duplicate records in each of these
    /// individual write buffers.
    ///
    /// Default: `1`
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_min_write_buffer_number_to_merge(2);
    /// ```
    pub fn set_min_write_buffer_number_to_merge(&mut self, to_merge: c_int) {
        unsafe {
            ffi::rocksdb_options_set_min_write_buffer_number_to_merge(self.inner, to_merge);
        }
    }

    /// Sets the number of files to trigger level-0 compaction. A value < `0` means that
    /// level-0 compaction will not be triggered by number of files at all.
    ///
    /// Default: `4`
    ///
    /// Dynamically changeable through SetOptions() API
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_level_zero_file_num_compaction_trigger(8);
    /// ```
    pub fn set_level_zero_file_num_compaction_trigger(&mut self, n: c_int) {
        unsafe {
            ffi::rocksdb_options_set_level0_file_num_compaction_trigger(self.inner, n);
        }
    }

    /// Sets the soft limit on number of level-0 files. We start slowing down writes at this
    /// point. A value < `0` means that no writing slow down will be triggered by
    /// number of files in level-0.
    ///
    /// Default: `20`
    ///
    /// Dynamically changeable through SetOptions() API
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_level_zero_slowdown_writes_trigger(10);
    /// ```
    pub fn set_level_zero_slowdown_writes_trigger(&mut self, n: c_int) {
        unsafe {
            ffi::rocksdb_options_set_level0_slowdown_writes_trigger(self.inner, n);
        }
    }

    /// Sets the maximum number of level-0 files.  We stop writes at this point.
    ///
    /// Default: `24`
    ///
    /// Dynamically changeable through SetOptions() API
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_level_zero_stop_writes_trigger(48);
    /// ```
    pub fn set_level_zero_stop_writes_trigger(&mut self, n: c_int) {
        unsafe {
            ffi::rocksdb_options_set_level0_stop_writes_trigger(self.inner, n);
        }
    }

    /// Sets the compaction style.
    ///
    /// Default: DBCompactionStyle::Level
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::{Options, DBCompactionStyle};
    ///
    /// let mut opts = Options::default();
    /// opts.set_compaction_style(DBCompactionStyle::Universal);
    /// ```
    pub fn set_compaction_style(&mut self, style: DBCompactionStyle) {
        unsafe {
            ffi::rocksdb_options_set_compaction_style(self.inner, style as c_int);
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
    /// Default: `1`
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_max_background_compactions(2);
    /// ```
    pub fn set_max_background_compactions(&mut self, n: c_int) {
        unsafe {
            ffi::rocksdb_options_set_max_background_compactions(self.inner, n);
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
    /// Default: `1`
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_max_background_flushes(2);
    /// ```
    pub fn set_max_background_flushes(&mut self, n: c_int) {
        unsafe {
            ffi::rocksdb_options_set_max_background_flushes(self.inner, n);
        }
    }

    /// Disables automatic compactions. Manual compactions can still
    /// be issued on this column family
    ///
    /// Default: `false`
    ///
    /// Dynamically changeable through SetOptions() API
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_disable_auto_compactions(true);
    /// ```
    pub fn set_disable_auto_compactions(&mut self, disable: bool) {
        unsafe { ffi::rocksdb_options_set_disable_auto_compactions(self.inner, disable as c_int) }
    }

    /// Defines the underlying memtable implementation.
    /// See https://github.com/facebook/rocksdb/wiki/MemTable for more information.
    /// Defaults to using a skiplist.
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::{Options, MemtableFactory};
    /// let mut opts = Options::default();
    /// let factory = MemtableFactory::HashSkipList {
    ///     bucket_count: 1_000_000,
    ///     height: 4,
    ///     branching_factor: 4,
    /// };
    ///
    /// opts.set_allow_concurrent_memtable_write(false);
    /// opts.set_memtable_factory(factory);
    /// ```
    pub fn set_memtable_factory(&mut self, factory: MemtableFactory) {
        match factory {
            MemtableFactory::Vector => unsafe {
                ffi::rocksdb_options_set_memtable_vector_rep(self.inner);
            },
            MemtableFactory::HashSkipList {
                bucket_count,
                height,
                branching_factor,
            } => unsafe {
                ffi::rocksdb_options_set_hash_skip_list_rep(
                    self.inner,
                    bucket_count,
                    height,
                    branching_factor,
                );
            },
            MemtableFactory::HashLinkList { bucket_count } => unsafe {
                ffi::rocksdb_options_set_hash_link_list_rep(self.inner, bucket_count);
            },
        };
    }

    pub fn set_block_based_table_factory(&mut self, factory: &BlockBasedOptions) {
        unsafe {
            ffi::rocksdb_options_set_block_based_table_factory(self.inner, factory.inner);
        }
    }

    /// See https://github.com/facebook/rocksdb/wiki/PlainTable-Format.
    ///
    /// ```
    /// use rocksdb::{Options, PlainTableFactoryOptions};
    ///
    /// let mut opts = Options::default();
    /// let factory_opts = PlainTableFactoryOptions {
    ///   user_key_length: 0,
    ///   bloom_bits_per_key: 20,
    ///   hash_table_ratio: 0.75,
    ///   index_sparseness: 16,
    /// };
    ///
    /// opts.set_plain_table_factory(&factory_opts);
    /// ```
    pub fn set_plain_table_factory(&mut self, options: &PlainTableFactoryOptions) {
        unsafe {
            ffi::rocksdb_options_set_plain_table_factory(
                self.inner,
                options.user_key_length,
                options.bloom_bits_per_key,
                options.hash_table_ratio,
                options.index_sparseness,
            );
        }
    }

    /// Measure IO stats in compactions and flushes, if `true`.
    ///
    /// Default: `false`
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_report_bg_io_stats(true);
    /// ```
    pub fn set_report_bg_io_stats(&mut self, enable: bool) {
        unsafe {
            ffi::rocksdb_options_set_report_bg_io_stats(self.inner, enable as c_int);
        }
    }

    /// Recovery mode to control the consistency while replaying WAL.
    ///
    /// Default: DBRecoveryMode::PointInTime
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::{Options, DBRecoveryMode};
    ///
    /// let mut opts = Options::default();
    /// opts.set_wal_recovery_mode(DBRecoveryMode::AbsoluteConsistency);
    /// ```
    pub fn set_wal_recovery_mode(&mut self, mode: DBRecoveryMode) {
        unsafe {
            ffi::rocksdb_options_set_wal_recovery_mode(self.inner, mode as c_int);
        }
    }

    pub fn enable_statistics(&mut self) {
        unsafe {
            ffi::rocksdb_options_enable_statistics(self.inner);
        }
    }

    pub fn get_statistics(&self) -> Option<String> {
        unsafe {
            let value = ffi::rocksdb_options_statistics_get_string(self.inner);
            if value.is_null() {
                return None;
            }

            // Must have valid UTF-8 format.
            let s = CStr::from_ptr(value).to_str().unwrap().to_owned();
            libc::free(value as *mut c_void);
            Some(s)
        }
    }

    /// If not zero, dump `rocksdb.stats` to LOG every `stats_dump_period_sec`.
    ///
    /// Default: `600` (10 mins)
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_stats_dump_period_sec(300);
    /// ```
    pub fn set_stats_dump_period_sec(&mut self, period: c_uint) {
        unsafe {
            ffi::rocksdb_options_set_stats_dump_period_sec(self.inner, period);
        }
    }

    /// When set to true, reading SST files will opt out of the filesystem's
    /// readahead. Setting this to false may improve sequential iteration
    /// performance.
    ///
    /// Default: `true`
    pub fn set_advise_random_on_open(&mut self, advise: bool) {
        unsafe { ffi::rocksdb_options_set_advise_random_on_open(self.inner, advise as c_uchar) }
    }

    /// Sets the number of levels for this database.
    pub fn set_num_levels(&mut self, n: c_int) {
        unsafe {
            ffi::rocksdb_options_set_num_levels(self.inner, n);
        }
    }

    /// When a `prefix_extractor` is defined through `opts.set_prefix_extractor` this
    /// creates a prefix bloom filter for each memtable with the size of
    /// `write_buffer_size * memtable_prefix_bloom_ratio` (capped at 0.25).
    ///
    /// Default: `0`
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::{Options, SliceTransform};
    ///
    /// let mut opts = Options::default();
    /// let transform = SliceTransform::create_fixed_prefix(10);
    /// opts.set_prefix_extractor(transform);
    /// opts.set_memtable_prefix_bloom_ratio(0.2);
    /// ```
    pub fn set_memtable_prefix_bloom_ratio(&mut self, ratio: f64) {
        unsafe {
            ffi::rocksdb_options_set_memtable_prefix_bloom_size_ratio(self.inner, ratio);
        }
    }

    /// Specifies the absolute path of the directory the
    /// write-ahead log (WAL) should be written to.
    ///
    /// Default: same directory as the database
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_wal_dir("/path/to/dir");
    /// ```
    pub fn set_wal_dir<P: AsRef<Path>>(&mut self, path: P) {
        let p = CString::new(path.as_ref().to_string_lossy().as_bytes()).unwrap();
        unsafe {
            ffi::rocksdb_options_set_wal_dir(self.inner, p.as_ptr());
        }
    }

    /// If true, then DB::Open() will not update the statistics used to optimize
    /// compaction decision by loading table properties from many files.
    /// Turning off this feature will improve DBOpen time especially in disk environment.
    ///
    /// Default: false
    pub fn set_skip_stats_update_on_db_open(&mut self, skip: bool) {
        unsafe {
            ffi::rocksdb_options_set_skip_stats_update_on_db_open(self.inner, skip as c_uchar);
        }
    }

    /// Specify the maximal number of info log files to be kept.
    pub fn set_keep_log_file_num(&mut self, nfiles: usize) {
        unsafe {
            ffi::rocksdb_options_set_keep_log_file_num(self.inner, nfiles);
        }
    }

    /// Allow the OS to mmap file for writing.
    ///
    /// Default: false
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut options = Options::default();
    /// options.set_allow_mmap_writes(true);
    /// ```
    pub fn set_allow_mmap_writes(&mut self, is_enabled: bool) {
        unsafe {
            ffi::rocksdb_options_set_allow_mmap_writes(self.inner, is_enabled as c_uchar);
        }
    }

    /// Allow the OS to mmap file for reading sst tables.
    ///
    /// Default: false
    ///
    /// # Example
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut options = Options::default();
    /// options.set_allow_mmap_reads(true);
    /// ```
    pub fn set_allow_mmap_reads(&mut self, is_enabled: bool) {
        unsafe {
            ffi::rocksdb_options_set_allow_mmap_reads(self.inner, is_enabled as c_uchar);
        }
    }
}

impl Default for Options {
    fn default() -> Options {
        unsafe {
            let opts = ffi::rocksdb_options_create();
            if opts.is_null() {
                panic!("Could not create RocksDB options");
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
            ffi::rocksdb_writeoptions_set_sync(self.inner, sync as c_uchar);
        }
    }

    pub fn disable_wal(&mut self, disable: bool) {
        unsafe {
            ffi::rocksdb_writeoptions_disable_WAL(self.inner, disable as c_int);
        }
    }
}

impl Default for WriteOptions {
    fn default() -> WriteOptions {
        let write_opts = unsafe { ffi::rocksdb_writeoptions_create() };
        if write_opts.is_null() {
            panic!("Could not create RocksDB write options");
        }
        WriteOptions { inner: write_opts }
    }
}

#[cfg(test)]
mod tests {
    use MemtableFactory;
    use Options;

    #[test]
    fn test_enable_statistics() {
        let mut opts = Options::default();
        opts.enable_statistics();
        opts.set_stats_dump_period_sec(60);
        assert!(opts.get_statistics().is_some());

        let opts = Options::default();
        assert!(opts.get_statistics().is_none());
    }

    #[test]
    fn test_set_memtable_factory() {
        let mut opts = Options::default();
        opts.set_memtable_factory(MemtableFactory::Vector);
        opts.set_memtable_factory(MemtableFactory::HashLinkList { bucket_count: 100 });
        opts.set_memtable_factory(MemtableFactory::HashSkipList {
            bucket_count: 100,
            height: 4,
            branching_factor: 4,
        });
    }
}
