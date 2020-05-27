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

use std::ffi::{CStr, CString};
use std::mem;
use std::path::Path;

use libc::{self, c_char, c_int, c_uchar, c_uint, c_void, size_t};

use crate::{
    compaction_filter::{self, filter_callback, CompactionFilterCallback, CompactionFilterFn},
    comparator::{self, ComparatorCallback, CompareFn},
    ffi,
    merge_operator::{
        self, full_merge_callback, partial_merge_callback, MergeFn, MergeOperatorCallback,
    },
    slice_transform::SliceTransform,
    Snapshot,
};

fn new_cache(capacity: size_t) -> *mut ffi::rocksdb_cache_t {
    unsafe { ffi::rocksdb_cache_create_lru(capacity) }
}

/// Database-wide options around performance and behavior.
///
/// Please read the official tuning [guide](https://github.com/facebook/rocksdb/wiki/RocksDB-Tuning-Guide)
/// and most importantly, measure performance under realistic workloads with realistic hardware.
///
/// # Examples
///
/// ```
/// use rocksdb::{Options, DB};
/// use rocksdb::DBCompactionStyle;
///
/// fn badly_tuned_for_somebody_elses_disk() -> DB {
///    let path = "path/for/rocksdb/storageX";
///    let mut opts = Options::default();
///    opts.create_if_missing(true);
///    opts.set_max_open_files(10000);
///    opts.set_use_fsync(false);
///    opts.set_bytes_per_sync(8388608);
///    opts.optimize_for_point_lookup(1024);
///    opts.set_table_cache_num_shard_bits(6);
///    opts.set_max_write_buffer_number(32);
///    opts.set_write_buffer_size(536870912);
///    opts.set_target_file_size_base(1073741824);
///    opts.set_min_write_buffer_number_to_merge(4);
///    opts.set_level_zero_stop_writes_trigger(2000);
///    opts.set_level_zero_slowdown_writes_trigger(0);
///    opts.set_compaction_style(DBCompactionStyle::Universal);
///    opts.set_max_background_compactions(4);
///    opts.set_max_background_flushes(4);
///    opts.set_disable_auto_compactions(true);
///
///    DB::open(&opts, path).unwrap()
/// }
/// ```
pub struct Options {
    pub(crate) inner: *mut ffi::rocksdb_options_t,
}

/// Optionally disable WAL or sync for this write.
///
/// # Examples
///
/// Making an unsafe write of a batch:
///
/// ```
/// use rocksdb::{DB, Options, WriteBatch, WriteOptions};
///
/// let path = "_path_for_rocksdb_storageY";
/// {
///     let db = DB::open_default(path).unwrap();
///     let mut batch = WriteBatch::default();
///     batch.put(b"my key", b"my value");
///     batch.put(b"key2", b"value2");
///     batch.put(b"key3", b"value3");
///
///     let mut write_options = WriteOptions::default();
///     write_options.set_sync(false);
///     write_options.disable_wal(true);
///
///     db.write_opt(batch, &write_options);
/// }
/// let _ = DB::destroy(&Options::default(), path);
/// ```
pub struct WriteOptions {
    pub(crate) inner: *mut ffi::rocksdb_writeoptions_t,
}

/// Optionally wait for the memtable flush to be performed.
///
/// # Examples
///
/// Manually flushing the memtable:
///
/// ```
/// use rocksdb::{DB, Options, FlushOptions};
///
/// let path = "_path_for_rocksdb_storageY";
/// {
///     let db = DB::open_default(path).unwrap();
///
///     let mut flush_options = FlushOptions::default();
///     flush_options.set_wait(true);
///
///     db.flush_opt(&flush_options);
/// }
/// let _ = DB::destroy(&Options::default(), path);
/// ```
pub struct FlushOptions {
    pub(crate) inner: *mut ffi::rocksdb_flushoptions_t,
}

/// For configuring block-based file storage.
pub struct BlockBasedOptions {
    pub(crate) inner: *mut ffi::rocksdb_block_based_table_options_t,
}

pub struct ReadOptions {
    pub(crate) inner: *mut ffi::rocksdb_readoptions_t,
    iterate_upper_bound: Option<Vec<u8>>,
}

/// For configuring external files ingestion.
///
/// # Examples
///
/// Move files instead of copying them:
///
/// ```
/// use rocksdb::{DB, IngestExternalFileOptions, SstFileWriter, Options};
///
/// let writer_opts = Options::default();
/// let mut writer = SstFileWriter::create(&writer_opts);
/// writer.open("_path_for_sst_file").unwrap();
/// writer.put(b"k1", b"v1").unwrap();
/// writer.finish().unwrap();
///
/// let path = "_path_for_rocksdb_storageY";
/// let db = DB::open_default(&path).unwrap();
/// let mut ingest_opts = IngestExternalFileOptions::default();
/// ingest_opts.set_move_files(true);
/// db.ingest_external_file_opts(&ingest_opts, vec!["_path_for_sst_file"]).unwrap();
/// ```
pub struct IngestExternalFileOptions {
    pub(crate) inner: *mut ffi::rocksdb_ingestexternalfileoptions_t,
}

// Safety note: auto-implementing Send on most db-related types is prevented by the inner FFI
// pointer. In most cases, however, this pointer is Send-safe because it is never aliased and
// rocksdb internally does not rely on thread-local information for its user-exposed types.
unsafe impl Send for Options {}
unsafe impl Send for WriteOptions {}
unsafe impl Send for BlockBasedOptions {}
unsafe impl Send for ReadOptions {}
unsafe impl Send for IngestExternalFileOptions {}

// Sync is similarly safe for many types because they do not expose interior mutability, and their
// use within the rocksdb library is generally behind a const reference
unsafe impl Sync for Options {}
unsafe impl Sync for WriteOptions {}
unsafe impl Sync for BlockBasedOptions {}
unsafe impl Sync for ReadOptions {}
unsafe impl Sync for IngestExternalFileOptions {}

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

impl Drop for FlushOptions {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_flushoptions_destroy(self.inner);
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

impl Drop for ReadOptions {
    fn drop(&mut self) {
        unsafe { ffi::rocksdb_readoptions_destroy(self.inner) }
    }
}

impl Drop for IngestExternalFileOptions {
    fn drop(&mut self) {
        unsafe { ffi::rocksdb_ingestexternalfileoptions_destroy(self.inner) }
    }
}

impl BlockBasedOptions {
    /// Approximate size of user data packed per block. Note that the
    /// block size specified here corresponds to uncompressed data. The
    /// actual size of the unit read from disk may be smaller if
    /// compression is enabled. This parameter can be changed dynamically.
    pub fn set_block_size(&mut self, size: usize) {
        unsafe {
            ffi::rocksdb_block_based_options_set_block_size(self.inner, size);
        }
    }

    /// Block size for partitioned metadata. Currently applied to indexes when
    /// kTwoLevelIndexSearch is used and to filters when partition_filters is used.
    /// Note: Since in the current implementation the filters and index partitions
    /// are aligned, an index/filter block is created when either index or filter
    /// block size reaches the specified limit.
    ///
    /// Note: this limit is currently applied to only index blocks; a filter
    /// partition is cut right after an index block is cut.
    pub fn set_metadata_block_size(&mut self, size: usize) {
        unsafe {
            ffi::rocksdb_block_based_options_set_metadata_block_size(self.inner, size as u64);
        }
    }

    /// Note: currently this option requires kTwoLevelIndexSearch to be set as
    /// well.
    ///
    /// Use partitioned full filters for each SST file. This option is
    /// incompatible with block-based filters.
    pub fn set_partition_filters(&mut self, size: bool) {
        unsafe {
            ffi::rocksdb_block_based_options_set_partition_filters(self.inner, size as c_uchar);
        }
    }

    /// When provided: use the specified cache for blocks.
    /// Otherwise rocksdb will automatically create and use an 8MB internal cache.
    pub fn set_lru_cache(&mut self, size: size_t) {
        let cache = new_cache(size);
        unsafe {
            // Since cache is wrapped in shared_ptr, we don't need to
            // call rocksdb_cache_destroy explicitly.
            ffi::rocksdb_block_based_options_set_block_cache(self.inner, cache);
        }
    }

    /// When configured: use the specified cache for compressed blocks.
    /// Otherwise rocksdb will not use a compressed block cache.
    ///
    /// Note: though it looks similar to `block_cache`, RocksDB doesn't put the
    /// same type of object there.
    pub fn set_lru_cache_compressed(&mut self, size: size_t) {
        let cache = new_cache(size);
        unsafe {
            // Since cache is wrapped in shared_ptr, we don't need to
            // call rocksdb_cache_destroy explicitly.
            ffi::rocksdb_block_based_options_set_block_cache_compressed(self.inner, cache);
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
    /// # Examples
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

    /// If cache_index_and_filter_blocks is true and the below is true, then
    /// filter and index blocks are stored in the cache, but a reference is
    /// held in the "table reader" object so the blocks are pinned and only
    /// evicted from cache when the table reader is freed.
    ///
    /// Default: false.
    pub fn set_pin_l0_filter_and_index_blocks_in_cache(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_block_based_options_set_pin_l0_filter_and_index_blocks_in_cache(
                self.inner,
                v as c_uchar,
            );
        }
    }

    /// If cache_index_and_filter_blocks is true and the below is true, then
    /// the top-level index of partitioned filter and index blocks are stored in
    /// the cache, but a reference is held in the "table reader" object so the
    /// blocks are pinned and only evicted from cache when the table reader is
    /// freed. This is not limited to l0 in LSM tree.
    ///
    /// Default: false.
    pub fn set_pin_top_level_index_and_filter(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_block_based_options_set_pin_top_level_index_and_filter(
                self.inner,
                v as c_uchar,
            );
        }
    }

    /// Format version, reserved for backward compatibility.
    ///
    /// See full [list](https://github.com/facebook/rocksdb/blob/f059c7d9b96300091e07429a60f4ad55dac84859/include/rocksdb/table.h#L249-L274)
    /// of the supported versions.
    ///
    /// Default: 2.
    pub fn set_format_version(&mut self, version: i32) {
        unsafe {
            ffi::rocksdb_block_based_options_set_format_version(self.inner, version);
        }
    }

    /// Number of keys between restart points for delta encoding of keys.
    /// This parameter can be changed dynamically. Most clients should
    /// leave this parameter alone. The minimum value allowed is 1. Any smaller
    /// value will be silently overwritten with 1.
    ///
    /// Default: 16.
    pub fn set_block_restart_interval(&mut self, interval: i32) {
        unsafe {
            ffi::rocksdb_block_based_options_set_block_restart_interval(self.inner, interval);
        }
    }

    /// Same as block_restart_interval but used for the index block.
    /// If you don't plan to run RocksDB before version 5.16 and you are
    /// using `index_block_restart_interval` > 1, you should
    /// probably set the `format_version` to >= 4 as it would reduce the index size.
    ///
    /// Default: 1.
    pub fn set_index_block_restart_interval(&mut self, interval: i32) {
        unsafe {
            ffi::rocksdb_block_based_options_set_index_block_restart_interval(self.inner, interval);
        }
    }

    /// Set the data block index type for point lookups:
    ///  `DataBlockIndexType::BinarySearch` to use binary search within the data block.
    ///  `DataBlockIndexType::BinaryAndHash` to use the data block hash index in combination with
    ///  the normal binary search.
    ///
    /// The hash table utilization ratio is adjustable using [`set_data_block_hash_ratio`](#method.set_data_block_hash_ratio), which is
    /// valid only when using `DataBlockIndexType::BinaryAndHash`.
    ///
    /// Default: `BinarySearch`
    /// # Examples
    ///
    /// ```
    /// use rocksdb::{BlockBasedOptions, DataBlockIndexType, Options};
    ///
    /// let mut opts = Options::default();
    /// let mut block_opts = BlockBasedOptions::default();
    /// block_opts.set_data_block_index_type(DataBlockIndexType::BinaryAndHash);
    /// block_opts.set_data_block_hash_ratio(0.85);
    /// ```
    pub fn set_data_block_index_type(&mut self, index_type: DataBlockIndexType) {
        let index_t = index_type as i32;
        unsafe { ffi::rocksdb_block_based_options_set_data_block_index_type(self.inner, index_t) }
    }

    /// Set the data block hash index utilization ratio.
    ///
    /// The smaller the utilization ratio, the less hash collisions happen, and so reduce the risk for a
    /// point lookup to fall back to binary search due to the collisions. A small ratio means faster
    /// lookup at the price of more space overhead.
    ///
    /// Default: 0.75
    pub fn set_data_block_hash_ratio(&mut self, ratio: f64) {
        unsafe { ffi::rocksdb_block_based_options_set_data_block_hash_ratio(self.inner, ratio) }
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
    /// # Examples
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

    /// Optimize level style compaction.
    ///
    /// Default values for some parameters in `Options` are not optimized for heavy
    /// workloads and big datasets, which means you might observe write stalls under
    /// some conditions.
    ///
    /// This can be used as one of the starting points for tuning RocksDB options in
    /// such cases.
    ///
    /// Internally, it sets `write_buffer_size`, `min_write_buffer_number_to_merge`,
    /// `max_write_buffer_number`, `level0_file_num_compaction_trigger`,
    /// `target_file_size_base`, `max_bytes_for_level_base`, so it can override if those
    /// parameters were set before.
    ///
    /// It sets buffer sizes so that memory consumption would be constrained by
    /// `memtable_memory_budget`.
    pub fn optimize_level_style_compaction(&mut self, memtable_memory_budget: usize) {
        unsafe {
            ffi::rocksdb_options_optimize_level_style_compaction(
                self.inner,
                memtable_memory_budget as u64,
            );
        }
    }

    /// Optimize universal style compaction.
    ///
    /// Default values for some parameters in `Options` are not optimized for heavy
    /// workloads and big datasets, which means you might observe write stalls under
    /// some conditions.
    ///
    /// This can be used as one of the starting points for tuning RocksDB options in
    /// such cases.
    ///
    /// Internally, it sets `write_buffer_size`, `min_write_buffer_number_to_merge`,
    /// `max_write_buffer_number`, `level0_file_num_compaction_trigger`,
    /// `target_file_size_base`, `max_bytes_for_level_base`, so it can override if those
    /// parameters were set before.
    ///
    /// It sets buffer sizes so that memory consumption would be constrained by
    /// `memtable_memory_budget`.
    pub fn optimize_universal_style_compaction(&mut self, memtable_memory_budget: usize) {
        unsafe {
            ffi::rocksdb_options_optimize_universal_style_compaction(
                self.inner,
                memtable_memory_budget as u64,
            );
        }
    }

    /// If true, the database will be created if it is missing.
    ///
    /// Default: `false`
    ///
    /// # Examples
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
    /// # Examples
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

    /// Sets the compression algorithm that will be used for compressing blocks.
    ///
    /// Default: `DBCompressionType::Snappy` (`DBCompressionType::None` if
    /// snappy feature is not enabled).
    ///
    /// # Examples
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
    /// # Examples
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

    /// Maximum size of dictionaries used to prime the compression library.
    /// Enabling dictionary can improve compression ratios when there are
    /// repetitions across data blocks.
    ///
    /// The dictionary is created by sampling the SST file data. If
    /// `zstd_max_train_bytes` is nonzero, the samples are passed through zstd's
    /// dictionary generator. Otherwise, the random samples are used directly as
    /// the dictionary.
    ///
    /// When compression dictionary is disabled, we compress and write each block
    /// before buffering data for the next one. When compression dictionary is
    /// enabled, we buffer all SST file data in-memory so we can sample it, as data
    /// can only be compressed and written after the dictionary has been finalized.
    /// So users of this feature may see increased memory usage.
    ///
    /// Default: `0`
    ///
    /// # Examples
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_compression_options(4, 5, 6, 7);
    /// ```
    pub fn set_compression_options(
        &mut self,
        w_bits: c_int,
        level: c_int,
        strategy: c_int,
        max_dict_bytes: c_int,
    ) {
        unsafe {
            ffi::rocksdb_options_set_compression_options(
                self.inner,
                w_bits,
                level,
                strategy,
                max_dict_bytes,
            );
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

    /// Allow RocksDB to pick dynamic base of bytes for levels.
    /// With this feature turned on, RocksDB will automatically adjust max bytes for each level.
    /// The goal of this feature is to have lower bound on size amplification.
    ///
    /// Default: false.
    pub fn set_level_compaction_dynamic_level_bytes(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_options_set_level_compaction_dynamic_level_bytes(self.inner, v as c_uchar);
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

    /// Sets the optimize_filters_for_hits flag
    ///
    /// Default: `false`
    ///
    /// # Examples
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_optimize_filters_for_hits(true);
    /// ```
    pub fn set_optimize_filters_for_hits(&mut self, optimize_for_hits: bool) {
        unsafe {
            ffi::rocksdb_options_set_optimize_filters_for_hits(
                self.inner,
                optimize_for_hits as c_int,
            );
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
    /// # Examples
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
    /// # Examples
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
    /// # Examples
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
    /// # Examples
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
    /// # Examples
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
    /// # Examples
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
    /// # Examples
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
    /// # Examples
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
    /// # Examples
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

    /// Sets the maximum number of write buffers that are built up in memory.
    /// The default and the minimum number is 2, so that when 1 write buffer
    /// is being flushed to storage, new writes can continue to the other
    /// write buffer.
    /// If max_write_buffer_number > 3, writing will be slowed down to
    /// options.delayed_write_rate if we are writing to the last write buffer
    /// allowed.
    ///
    /// Default: `2`
    ///
    /// # Examples
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_max_write_buffer_number(4);
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
    /// # Examples
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
    /// # Examples
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
    /// # Examples
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
    /// # Examples
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
    /// # Examples
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
    /// # Examples
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
    /// # Examples
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
    /// # Examples
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
    /// # Examples
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
    /// # Examples
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
    /// # Examples
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
    /// # Examples
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
    /// # Examples
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
    /// # Examples
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

    /// SetMemtableHugePageSize sets the page size for huge page for
    /// arena used by the memtable.
    /// If <=0, it won't allocate from huge page but from malloc.
    /// Users are responsible to reserve huge pages for it to be allocated. For
    /// example:
    ///      sysctl -w vm.nr_hugepages=20
    /// See linux doc Documentation/vm/hugetlbpage.txt
    /// If there isn't enough free huge page available, it will fall back to
    /// malloc.
    ///
    /// Dynamically changeable through SetOptions() API
    pub fn set_memtable_huge_page_size(&mut self, size: size_t) {
        unsafe { ffi::rocksdb_options_set_memtable_huge_page_size(self.inner, size) }
    }

    /// By default, a single write thread queue is maintained. The thread gets
    /// to the head of the queue becomes write batch group leader and responsible
    /// for writing to WAL and memtable for the batch group.
    ///
    /// If enable_pipelined_write is true, separate write thread queue is
    /// maintained for WAL write and memtable write. A write thread first enter WAL
    /// writer queue and then memtable writer queue. Pending thread on the WAL
    /// writer queue thus only have to wait for previous writers to finish their
    /// WAL writing but not the memtable writing. Enabling the feature may improve
    /// write throughput and reduce latency of the prepare phase of two-phase
    /// commit.
    ///
    /// Default: false
    pub fn set_enable_pipelined_write(&mut self, value: bool) {
        unsafe { ffi::rocksdb_options_set_enable_pipelined_write(self.inner, value as c_uchar) }
    }

    /// Defines the underlying memtable implementation.
    /// See official [wiki](https://github.com/facebook/rocksdb/wiki/MemTable) for more information.
    /// Defaults to using a skiplist.
    ///
    /// # Examples
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

    // This is a factory that provides TableFactory objects.
    // Default: a block-based table factory that provides a default
    // implementation of TableBuilder and TableReader with default
    // BlockBasedTableOptions.
    /// Sets the factory as plain table.
    /// See official [wiki](https://github.com/facebook/rocksdb/wiki/PlainTable-Format) for more
    /// information.
    ///
    /// # Examples
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
    /// # Examples
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

    /// Once write-ahead logs exceed this size, we will start forcing the flush of
    /// column families whose memtables are backed by the oldest live WAL file
    /// (i.e. the ones that are causing all the space amplification).
    ///
    /// Default: `0`
    ///
    /// # Examples
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// // Set max total wal size to 1G.
    /// opts.set_max_total_wal_size(1 << 30);
    /// ```
    pub fn set_max_total_wal_size(&mut self, size: u64) {
        unsafe {
            ffi::rocksdb_options_set_max_total_wal_size(self.inner, size);
        }
    }

    /// Recovery mode to control the consistency while replaying WAL.
    ///
    /// Default: DBRecoveryMode::PointInTime
    ///
    /// # Examples
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
    /// # Examples
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
    /// # Examples
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
    /// # Examples
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
    ///
    /// Default: 1000
    ///
    /// # Examples
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut options = Options::default();
    /// options.set_keep_log_file_num(100);
    /// ```
    pub fn set_keep_log_file_num(&mut self, nfiles: usize) {
        unsafe {
            ffi::rocksdb_options_set_keep_log_file_num(self.inner, nfiles);
        }
    }

    /// Allow the OS to mmap file for writing.
    ///
    /// Default: false
    ///
    /// # Examples
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
    /// # Examples
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

    /// Guarantee that all column families are flushed together atomically.
    /// This option applies to both manual flushes (`db.flush()`) and automatic
    /// background flushes caused when memtables are filled.
    ///
    /// Note that this is only useful when the WAL is disabled. When using the
    /// WAL, writes are always consistent across column families.
    ///
    /// Default: false
    ///
    /// # Examples
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut options = Options::default();
    /// options.set_atomic_flush(true);
    /// ```
    pub fn set_atomic_flush(&mut self, atomic_flush: bool) {
        unsafe {
            ffi::rocksdb_options_set_atomic_flush(self.inner, atomic_flush as c_uchar);
        }
    }

    /// Use to control write rate of flush and compaction. Flush has higher
    /// priority than compaction.
    /// If rate limiter is enabled, bytes_per_sync is set to 1MB by default.
    ///
    /// Default: disable
    ///
    /// # Examples
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut options = Options::default();
    /// options.set_ratelimiter(1024 * 1024, 100 * 1000, 10);
    /// ```
    pub fn set_ratelimiter(
        &mut self,
        rate_bytes_per_sec: i64,
        refill_period_us: i64,
        fairness: i32,
    ) {
        unsafe {
            let ratelimiter =
                ffi::rocksdb_ratelimiter_create(rate_bytes_per_sec, refill_period_us, fairness);
            // Since limiter is wrapped in shared_ptr, we don't need to
            // call rocksdb_ratelimiter_destroy explicitly.
            ffi::rocksdb_options_set_ratelimiter(self.inner, ratelimiter);
        }
    }

    /// Sets the maximal size of the info log file.
    ///
    /// If the log file is larger than `max_log_file_size`, a new info log file
    /// will be created. If `max_log_file_size` is equal to zero, all logs will
    /// be written to one log file.
    ///
    /// Default: 0
    ///
    /// # Examples
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut options = Options::default();
    /// options.set_max_log_file_size(0);
    /// ```
    pub fn set_max_log_file_size(&mut self, size: usize) {
        unsafe {
            ffi::rocksdb_options_set_max_log_file_size(self.inner, size);
        }
    }

    /// Controls the recycling of log files.
    ///
    /// If non-zero, previously written log files will be reused for new logs,
    /// overwriting the old data. The value indicates how many such files we will
    /// keep around at any point in time for later use. This is more efficient
    /// because the blocks are already allocated and fdatasync does not need to
    /// update the inode after each write.
    ///
    /// Default: 0
    ///
    /// # Examples
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut options = Options::default();
    /// options.set_recycle_log_file_num(5);
    /// ```
    pub fn set_recycle_log_file_num(&mut self, num: usize) {
        unsafe {
            ffi::rocksdb_options_set_recycle_log_file_num(self.inner, num);
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

impl FlushOptions {
    pub fn new() -> FlushOptions {
        FlushOptions::default()
    }

    /// Waits until the flush is done.
    ///
    /// Default: true
    ///
    /// # Examples
    ///
    /// ```
    /// use rocksdb::FlushOptions;
    ///
    /// let mut options = FlushOptions::default();
    /// options.set_wait(false);
    /// ```
    pub fn set_wait(&mut self, wait: bool) {
        unsafe {
            ffi::rocksdb_flushoptions_set_wait(self.inner, wait as c_uchar);
        }
    }
}

impl Default for FlushOptions {
    fn default() -> FlushOptions {
        let flush_opts = unsafe { ffi::rocksdb_flushoptions_create() };
        if flush_opts.is_null() {
            panic!("Could not create RocksDB flush options");
        }
        FlushOptions { inner: flush_opts }
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

impl ReadOptions {
    // TODO add snapshot setting here
    // TODO add snapshot wrapper structs with proper destructors;
    // that struct needs an "iterator" impl too.
    #[allow(dead_code)]
    fn fill_cache(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_readoptions_set_fill_cache(self.inner, v as c_uchar);
        }
    }

    pub(crate) fn set_snapshot(&mut self, snapshot: &Snapshot) {
        unsafe {
            ffi::rocksdb_readoptions_set_snapshot(self.inner, snapshot.inner);
        }
    }

    /// Sets the upper bound for an iterator.
    /// The upper bound itself is not included on the iteration result.
    pub fn set_iterate_upper_bound<K: Into<Vec<u8>>>(&mut self, key: K) {
        self.iterate_upper_bound = Some(key.into());
        let upper_bound = self
            .iterate_upper_bound
            .as_ref()
            .expect("iterate_upper_bound must exist.");

        unsafe {
            ffi::rocksdb_readoptions_set_iterate_upper_bound(
                self.inner,
                upper_bound.as_ptr() as *const c_char,
                upper_bound.len() as size_t,
            );
        }
    }

    pub fn set_prefix_same_as_start(&mut self, v: bool) {
        unsafe { ffi::rocksdb_readoptions_set_prefix_same_as_start(self.inner, v as c_uchar) }
    }

    pub fn set_total_order_seek(&mut self, v: bool) {
        unsafe { ffi::rocksdb_readoptions_set_total_order_seek(self.inner, v as c_uchar) }
    }

    /// If true, all data read from underlying storage will be
    /// verified against corresponding checksums.
    ///
    /// Default: true
    pub fn set_verify_checksums(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_readoptions_set_verify_checksums(self.inner, v as c_uchar);
        }
    }

    /// If non-zero, an iterator will create a new table reader which
    /// performs reads of the given size. Using a large size (> 2MB) can
    /// improve the performance of forward iteration on spinning disks.
    /// Default: 0
    ///
    /// ```
    /// use rocksdb::{ReadOptions};
    ///
    /// let mut opts = ReadOptions::default();
    /// opts.set_readahead_size(4_194_304); // 4mb
    /// ```
    pub fn set_readahead_size(&mut self, v: usize) {
        unsafe {
            ffi::rocksdb_readoptions_set_readahead_size(self.inner, v as size_t);
        }
    }

    /// If true, create a tailing iterator. Note that tailing iterators
    /// only support moving in the forward direction. Iterating in reverse
    /// or seek_to_last are not supported.
    pub fn set_tailing(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_readoptions_set_tailing(self.inner, v as c_uchar);
        }
    }
}

impl Default for ReadOptions {
    fn default() -> ReadOptions {
        unsafe {
            ReadOptions {
                inner: ffi::rocksdb_readoptions_create(),
                iterate_upper_bound: None,
            }
        }
    }
}

impl IngestExternalFileOptions {
    /// Can be set to true to move the files instead of copying them.
    pub fn set_move_files(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_ingestexternalfileoptions_set_move_files(self.inner, v as c_uchar);
        }
    }

    /// If set to false, an ingested file keys could appear in existing snapshots
    /// that where created before the file was ingested.
    pub fn set_snapshot_consistency(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_ingestexternalfileoptions_set_snapshot_consistency(
                self.inner,
                v as c_uchar,
            );
        }
    }

    /// If set to false, IngestExternalFile() will fail if the file key range
    /// overlaps with existing keys or tombstones in the DB.
    pub fn set_allow_global_seqno(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_ingestexternalfileoptions_set_allow_global_seqno(self.inner, v as c_uchar);
        }
    }

    /// If set to false and the file key range overlaps with the memtable key range
    /// (memtable flush required), IngestExternalFile will fail.
    pub fn set_allow_blocking_flush(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_ingestexternalfileoptions_set_allow_blocking_flush(
                self.inner,
                v as c_uchar,
            );
        }
    }

    /// Set to true if you would like duplicate keys in the file being ingested
    /// to be skipped rather than overwriting existing data under that key.
    /// Usecase: back-fill of some historical data in the database without
    /// over-writing existing newer version of data.
    /// This option could only be used if the DB has been running
    /// with allow_ingest_behind=true since the dawn of time.
    /// All files will be ingested at the bottommost level with seqno=0.
    pub fn set_ingest_behind(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_ingestexternalfileoptions_set_ingest_behind(self.inner, v as c_uchar);
        }
    }
}

impl Default for IngestExternalFileOptions {
    fn default() -> IngestExternalFileOptions {
        unsafe {
            IngestExternalFileOptions {
                inner: ffi::rocksdb_ingestexternalfileoptions_create(),
            }
        }
    }
}

/// Used by BlockBasedOptions::set_index_type.
pub enum BlockBasedIndexType {
    /// A space efficient index block that is optimized for
    /// binary-search-based index.
    BinarySearch,

    /// The hash index, if enabled, will perform a hash lookup if
    /// a prefix extractor has been provided through Options::set_prefix_extractor.
    HashSearch,

    /// A two-level index implementation. Both levels are binary search indexes.
    TwoLevelIndexSearch,
}

/// Used by BlockBasedOptions::set_data_block_index_type.
#[repr(C)]
pub enum DataBlockIndexType {
    /// Use binary search when performing point lookup for keys in data blocks.
    /// This is the default.
    BinarySearch = 0,

    /// Appends a compact hash table to the end of the data block for efficient indexing. Backwards
    /// compatible with databases created without this feature. Once turned on, existing data will
    /// be gradually converted to the hash index format.
    BinaryAndHash = 1,
}

/// Defines the underlying memtable implementation.
/// See official [wiki](https://github.com/facebook/rocksdb/wiki/MemTable) for more information.
pub enum MemtableFactory {
    Vector,
    HashSkipList {
        bucket_count: usize,
        height: i32,
        branching_factor: i32,
    },
    HashLinkList {
        bucket_count: usize,
    },
}

/// Used with DBOptions::set_plain_table_factory.
/// See official [wiki](https://github.com/facebook/rocksdb/wiki/PlainTable-Format) for more
/// information.
///
/// Defaults:
///  user_key_length: 0 (variable length)
///  bloom_bits_per_key: 10
///  hash_table_ratio: 0.75
///  index_sparseness: 16
pub struct PlainTableFactoryOptions {
    pub user_key_length: u32,
    pub bloom_bits_per_key: i32,
    pub hash_table_ratio: f64,
    pub index_sparseness: usize,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DBCompressionType {
    None = ffi::rocksdb_no_compression as isize,
    Snappy = ffi::rocksdb_snappy_compression as isize,
    Zlib = ffi::rocksdb_zlib_compression as isize,
    Bz2 = ffi::rocksdb_bz2_compression as isize,
    Lz4 = ffi::rocksdb_lz4_compression as isize,
    Lz4hc = ffi::rocksdb_lz4hc_compression as isize,
    Zstd = ffi::rocksdb_zstd_compression as isize,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DBCompactionStyle {
    Level = ffi::rocksdb_level_compaction as isize,
    Universal = ffi::rocksdb_universal_compaction as isize,
    Fifo = ffi::rocksdb_fifo_compaction as isize,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DBRecoveryMode {
    TolerateCorruptedTailRecords = ffi::rocksdb_tolerate_corrupted_tail_records_recovery as isize,
    AbsoluteConsistency = ffi::rocksdb_absolute_consistency_recovery as isize,
    PointInTime = ffi::rocksdb_point_in_time_recovery as isize,
    SkipAnyCorruptedRecord = ffi::rocksdb_skip_any_corrupted_records_recovery as isize,
}

#[cfg(test)]
mod tests {
    use crate::{MemtableFactory, Options};

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
