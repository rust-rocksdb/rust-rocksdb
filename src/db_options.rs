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

use std::ffi::CStr;
use std::path::Path;
use std::ptr::{null_mut, NonNull};
use std::slice;
use std::sync::Arc;

use libc::{self, c_char, c_double, c_int, c_uchar, c_uint, c_void, size_t};

use crate::{
    compaction_filter::{self, CompactionFilterCallback, CompactionFilterFn},
    compaction_filter_factory::{self, CompactionFilterFactory},
    comparator::{self, ComparatorCallback, CompareFn},
    db::DBAccess,
    env::Env,
    ffi,
    ffi_util::{from_cstr, to_cpath, CStrLike},
    merge_operator::{
        self, full_merge_callback, partial_merge_callback, MergeFn, MergeOperatorCallback,
    },
    slice_transform::SliceTransform,
    ColumnFamilyDescriptor, Error, SnapshotWithThreadMode, WriteBufferManager,
};

pub(crate) struct CacheWrapper {
    pub(crate) inner: NonNull<ffi::rocksdb_cache_t>,
}

impl Drop for CacheWrapper {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_cache_destroy(self.inner.as_ptr());
        }
    }
}

#[derive(Clone)]
pub struct Cache(pub(crate) Arc<CacheWrapper>);

impl Cache {
    /// Creates an LRU cache with capacity in bytes.
    pub fn new_lru_cache(capacity: size_t) -> Cache {
        let inner = NonNull::new(unsafe { ffi::rocksdb_cache_create_lru(capacity) }).unwrap();
        Cache(Arc::new(CacheWrapper { inner }))
    }

    /// Creates a HyperClockCache with capacity in bytes.
    ///
    /// `estimated_entry_charge` is an important tuning parameter. The optimal
    /// choice at any given time is
    /// `(cache.get_usage() - 64 * cache.get_table_address_count()) /
    /// cache.get_occupancy_count()`, or approximately `cache.get_usage() /
    /// cache.get_occupancy_count()`.
    ///
    /// However, the value cannot be changed dynamically, so as the cache
    /// composition changes at runtime, the following tradeoffs apply:
    ///
    /// * If the estimate is substantially too high (e.g., 25% higher),
    ///   the cache may have to evict entries to prevent load factors that
    ///   would dramatically affect lookup times.
    /// * If the estimate is substantially too low (e.g., less than half),
    ///   then meta data space overhead is substantially higher.
    ///
    /// The latter is generally preferable, and picking the larger of
    /// block size and meta data block size is a reasonable choice that
    /// errs towards this side.
    pub fn new_hyper_clock_cache(capacity: size_t, estimated_entry_charge: size_t) -> Cache {
        Cache(Arc::new(CacheWrapper {
            inner: NonNull::new(unsafe {
                ffi::rocksdb_cache_create_hyper_clock(capacity, estimated_entry_charge)
            })
            .unwrap(),
        }))
    }

    /// Returns the cache memory usage in bytes.
    pub fn get_usage(&self) -> usize {
        unsafe { ffi::rocksdb_cache_get_usage(self.0.inner.as_ptr()) }
    }

    /// Returns the pinned memory usage in bytes.
    pub fn get_pinned_usage(&self) -> usize {
        unsafe { ffi::rocksdb_cache_get_pinned_usage(self.0.inner.as_ptr()) }
    }

    /// Sets cache capacity in bytes.
    pub fn set_capacity(&mut self, capacity: size_t) {
        unsafe {
            ffi::rocksdb_cache_set_capacity(self.0.inner.as_ptr(), capacity);
        }
    }
}

#[derive(Default)]
pub(crate) struct OptionsMustOutliveDB {
    env: Option<Env>,
    row_cache: Option<Cache>,
    block_based: Option<BlockBasedOptionsMustOutliveDB>,
}

impl OptionsMustOutliveDB {
    pub(crate) fn clone(&self) -> Self {
        Self {
            env: self.env.as_ref().map(Env::clone),
            row_cache: self.row_cache.as_ref().map(Cache::clone),
            block_based: self
                .block_based
                .as_ref()
                .map(BlockBasedOptionsMustOutliveDB::clone),
        }
    }
}

#[derive(Default)]
struct BlockBasedOptionsMustOutliveDB {
    block_cache: Option<Cache>,
}

impl BlockBasedOptionsMustOutliveDB {
    fn clone(&self) -> Self {
        Self {
            block_cache: self.block_cache.as_ref().map(Cache::clone),
        }
    }
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
///    opts.set_disable_auto_compactions(true);
///
///    DB::open(&opts, path).unwrap()
/// }
/// ```
pub struct Options {
    pub(crate) inner: *mut ffi::rocksdb_options_t,
    pub(crate) outlive: OptionsMustOutliveDB,
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
/// let path = "_path_for_rocksdb_storageY1";
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
/// let path = "_path_for_rocksdb_storageY2";
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
    outlive: BlockBasedOptionsMustOutliveDB,
}

pub struct ReadOptions {
    pub(crate) inner: *mut ffi::rocksdb_readoptions_t,
    iterate_upper_bound: Option<Vec<u8>>,
    iterate_lower_bound: Option<Vec<u8>>,
}

/// Configuration of cuckoo-based storage.
pub struct CuckooTableOptions {
    pub(crate) inner: *mut ffi::rocksdb_cuckoo_table_options_t,
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
/// let path = "_path_for_rocksdb_storageY3";
/// {
///   let db = DB::open_default(&path).unwrap();
///   let mut ingest_opts = IngestExternalFileOptions::default();
///   ingest_opts.set_move_files(true);
///   db.ingest_external_file_opts(&ingest_opts, vec!["_path_for_sst_file"]).unwrap();
/// }
/// let _ = DB::destroy(&Options::default(), path);
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
unsafe impl Send for CuckooTableOptions {}
unsafe impl Send for ReadOptions {}
unsafe impl Send for IngestExternalFileOptions {}
unsafe impl Send for CacheWrapper {}

// Sync is similarly safe for many types because they do not expose interior mutability, and their
// use within the rocksdb library is generally behind a const reference
unsafe impl Sync for Options {}
unsafe impl Sync for WriteOptions {}
unsafe impl Sync for BlockBasedOptions {}
unsafe impl Sync for CuckooTableOptions {}
unsafe impl Sync for ReadOptions {}
unsafe impl Sync for IngestExternalFileOptions {}
unsafe impl Sync for CacheWrapper {}

impl Drop for Options {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_options_destroy(self.inner);
        }
    }
}

impl Clone for Options {
    fn clone(&self) -> Self {
        let inner = unsafe { ffi::rocksdb_options_create_copy(self.inner) };
        assert!(!inner.is_null(), "Could not copy RocksDB options");

        Self {
            inner,
            outlive: self.outlive.clone(),
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

impl Drop for CuckooTableOptions {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_cuckoo_options_destroy(self.inner);
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
        unsafe {
            ffi::rocksdb_readoptions_destroy(self.inner);
        }
    }
}

impl Drop for IngestExternalFileOptions {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_ingestexternalfileoptions_destroy(self.inner);
        }
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
            ffi::rocksdb_block_based_options_set_partition_filters(self.inner, c_uchar::from(size));
        }
    }

    /// Sets global cache for blocks (user data is stored in a set of blocks, and
    /// a block is the unit of reading from disk). Cache must outlive DB instance which uses it.
    ///
    /// If set, use the specified cache for blocks.
    /// By default, rocksdb will automatically create and use an 8MB internal cache.
    pub fn set_block_cache(&mut self, cache: &Cache) {
        unsafe {
            ffi::rocksdb_block_based_options_set_block_cache(self.inner, cache.0.inner.as_ptr());
        }
        self.outlive.block_cache = Some(cache.clone());
    }

    /// Disable block cache
    pub fn disable_cache(&mut self) {
        unsafe {
            ffi::rocksdb_block_based_options_set_no_block_cache(self.inner, c_uchar::from(true));
        }
    }

    /// Sets a [Bloom filter](https://github.com/facebook/rocksdb/wiki/RocksDB-Bloom-Filter)
    /// policy to reduce disk reads.
    ///
    /// # Examples
    ///
    /// ```
    /// use rocksdb::BlockBasedOptions;
    ///
    /// let mut opts = BlockBasedOptions::default();
    /// opts.set_bloom_filter(10.0, true);
    /// ```
    pub fn set_bloom_filter(&mut self, bits_per_key: c_double, block_based: bool) {
        unsafe {
            let bloom = if block_based {
                ffi::rocksdb_filterpolicy_create_bloom(bits_per_key as _)
            } else {
                ffi::rocksdb_filterpolicy_create_bloom_full(bits_per_key as _)
            };

            ffi::rocksdb_block_based_options_set_filter_policy(self.inner, bloom);
        }
    }

    /// Sets a [Ribbon filter](http://rocksdb.org/blog/2021/12/29/ribbon-filter.html)
    /// policy to reduce disk reads.
    ///
    /// Ribbon filters use less memory in exchange for slightly more CPU usage
    /// compared to an equivalent bloom filter.
    ///
    /// # Examples
    ///
    /// ```
    /// use rocksdb::BlockBasedOptions;
    ///
    /// let mut opts = BlockBasedOptions::default();
    /// opts.set_ribbon_filter(10.0);
    /// ```
    pub fn set_ribbon_filter(&mut self, bloom_equivalent_bits_per_key: c_double) {
        unsafe {
            let ribbon = ffi::rocksdb_filterpolicy_create_ribbon(bloom_equivalent_bits_per_key);
            ffi::rocksdb_block_based_options_set_filter_policy(self.inner, ribbon);
        }
    }

    /// Sets a hybrid [Ribbon filter](http://rocksdb.org/blog/2021/12/29/ribbon-filter.html)
    /// policy to reduce disk reads.
    ///
    /// Uses Bloom filters before the given level, and Ribbon filters for all
    /// other levels. This combines the memory savings from Ribbon filters
    /// with the lower CPU usage of Bloom filters.
    ///
    /// # Examples
    ///
    /// ```
    /// use rocksdb::BlockBasedOptions;
    ///
    /// let mut opts = BlockBasedOptions::default();
    /// opts.set_hybrid_ribbon_filter(10.0, 2);
    /// ```
    pub fn set_hybrid_ribbon_filter(
        &mut self,
        bloom_equivalent_bits_per_key: c_double,
        bloom_before_level: c_int,
    ) {
        unsafe {
            let ribbon = ffi::rocksdb_filterpolicy_create_ribbon_hybrid(
                bloom_equivalent_bits_per_key,
                bloom_before_level,
            );
            ffi::rocksdb_block_based_options_set_filter_policy(self.inner, ribbon);
        }
    }

    /// If cache_index_and_filter_blocks is enabled, cache index and filter blocks with high priority.
    /// If set to true, depending on implementation of block cache,
    /// index and filter blocks may be less likely to be evicted than data blocks.
    pub fn set_cache_index_and_filter_blocks(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_block_based_options_set_cache_index_and_filter_blocks(
                self.inner,
                c_uchar::from(v),
            );
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
                c_uchar::from(v),
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
                c_uchar::from(v),
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
        unsafe {
            ffi::rocksdb_block_based_options_set_data_block_index_type(self.inner, index_t);
        }
    }

    /// Set the data block hash index utilization ratio.
    ///
    /// The smaller the utilization ratio, the less hash collisions happen, and so reduce the risk for a
    /// point lookup to fall back to binary search due to the collisions. A small ratio means faster
    /// lookup at the price of more space overhead.
    ///
    /// Default: 0.75
    pub fn set_data_block_hash_ratio(&mut self, ratio: f64) {
        unsafe {
            ffi::rocksdb_block_based_options_set_data_block_hash_ratio(self.inner, ratio);
        }
    }

    /// If false, place only prefixes in the filter, not whole keys.
    ///
    /// Defaults to true.
    pub fn set_whole_key_filtering(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_block_based_options_set_whole_key_filtering(self.inner, c_uchar::from(v));
        }
    }

    /// Use the specified checksum type.
    /// Newly created table files will be protected with this checksum type.
    /// Old table files will still be readable, even though they have different checksum type.
    pub fn set_checksum_type(&mut self, checksum_type: ChecksumType) {
        unsafe {
            ffi::rocksdb_block_based_options_set_checksum(self.inner, checksum_type as c_char);
        }
    }
}

impl Default for BlockBasedOptions {
    fn default() -> Self {
        let block_opts = unsafe { ffi::rocksdb_block_based_options_create() };
        assert!(
            !block_opts.is_null(),
            "Could not create RocksDB block based options"
        );

        Self {
            inner: block_opts,
            outlive: BlockBasedOptionsMustOutliveDB::default(),
        }
    }
}

impl CuckooTableOptions {
    /// Determines the utilization of hash tables. Smaller values
    /// result in larger hash tables with fewer collisions.
    /// Default: 0.9
    pub fn set_hash_ratio(&mut self, ratio: f64) {
        unsafe {
            ffi::rocksdb_cuckoo_options_set_hash_ratio(self.inner, ratio);
        }
    }

    /// A property used by builder to determine the depth to go to
    /// to search for a path to displace elements in case of
    /// collision. See Builder.MakeSpaceForKey method. Higher
    /// values result in more efficient hash tables with fewer
    /// lookups but take more time to build.
    /// Default: 100
    pub fn set_max_search_depth(&mut self, depth: u32) {
        unsafe {
            ffi::rocksdb_cuckoo_options_set_max_search_depth(self.inner, depth);
        }
    }

    /// In case of collision while inserting, the builder
    /// attempts to insert in the next cuckoo_block_size
    /// locations before skipping over to the next Cuckoo hash
    /// function. This makes lookups more cache friendly in case
    /// of collisions.
    /// Default: 5
    pub fn set_cuckoo_block_size(&mut self, size: u32) {
        unsafe {
            ffi::rocksdb_cuckoo_options_set_cuckoo_block_size(self.inner, size);
        }
    }

    /// If this option is enabled, user key is treated as uint64_t and its value
    /// is used as hash value directly. This option changes builder's behavior.
    /// Reader ignore this option and behave according to what specified in
    /// table property.
    /// Default: false
    pub fn set_identity_as_first_hash(&mut self, flag: bool) {
        unsafe {
            ffi::rocksdb_cuckoo_options_set_identity_as_first_hash(self.inner, c_uchar::from(flag));
        }
    }

    /// If this option is set to true, module is used during hash calculation.
    /// This often yields better space efficiency at the cost of performance.
    /// If this option is set to false, # of entries in table is constrained to
    /// be power of two, and bit and is used to calculate hash, which is faster in general.
    /// Default: true
    pub fn set_use_module_hash(&mut self, flag: bool) {
        unsafe {
            ffi::rocksdb_cuckoo_options_set_use_module_hash(self.inner, c_uchar::from(flag));
        }
    }
}

impl Default for CuckooTableOptions {
    fn default() -> Self {
        let opts = unsafe { ffi::rocksdb_cuckoo_options_create() };
        assert!(!opts.is_null(), "Could not create RocksDB cuckoo options");

        Self { inner: opts }
    }
}

// Verbosity of the LOG.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(i32)]
pub enum LogLevel {
    Debug = 0,
    Info,
    Warn,
    Error,
    Fatal,
    Header,
}

impl Options {
    /// Constructs the DBOptions and ColumnFamilyDescriptors by loading the
    /// latest RocksDB options file stored in the specified rocksdb database.
    pub fn load_latest<P: AsRef<Path>>(
        path: P,
        env: Env,
        ignore_unknown_options: bool,
        cache: Cache,
    ) -> Result<(Options, Vec<ColumnFamilyDescriptor>), Error> {
        let path = to_cpath(path)?;
        let mut db_options: *mut ffi::rocksdb_options_t = null_mut();
        let mut num_column_families: usize = 0;
        let mut column_family_names: *mut *mut c_char = null_mut();
        let mut column_family_options: *mut *mut ffi::rocksdb_options_t = null_mut();
        unsafe {
            ffi_try!(ffi::rocksdb_load_latest_options(
                path.as_ptr(),
                env.0.inner,
                ignore_unknown_options,
                cache.0.inner.as_ptr(),
                &mut db_options,
                &mut num_column_families,
                &mut column_family_names,
                &mut column_family_options,
            ));
        }
        let options = Options {
            inner: db_options,
            outlive: OptionsMustOutliveDB::default(),
        };
        let column_families = unsafe {
            Options::read_column_descriptors(
                num_column_families,
                column_family_names,
                column_family_options,
            )
        };
        Ok((options, column_families))
    }

    /// read column descriptors from c pointers
    #[inline]
    unsafe fn read_column_descriptors(
        num_column_families: usize,
        column_family_names: *mut *mut c_char,
        column_family_options: *mut *mut ffi::rocksdb_options_t,
    ) -> Vec<ColumnFamilyDescriptor> {
        let column_family_names_iter =
            slice::from_raw_parts(column_family_names, num_column_families)
                .iter()
                .map(|ptr| from_cstr(*ptr));
        let column_family_options_iter =
            slice::from_raw_parts(column_family_options, num_column_families)
                .iter()
                .map(|ptr| Options {
                    inner: *ptr,
                    outlive: OptionsMustOutliveDB::default(),
                });
        let column_descriptors = column_family_names_iter
            .zip(column_family_options_iter)
            .map(|(name, options)| ColumnFamilyDescriptor { name, options })
            .collect::<Vec<_>>();
        // free pointers
        slice::from_raw_parts(column_family_names, num_column_families)
            .iter()
            .for_each(|ptr| ffi::rocksdb_free(*ptr as *mut c_void));
        ffi::rocksdb_free(column_family_names as *mut c_void);
        ffi::rocksdb_free(column_family_options as *mut c_void);
        column_descriptors
    }

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
            ffi::rocksdb_options_set_create_if_missing(
                self.inner,
                c_uchar::from(create_if_missing),
            );
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
                c_uchar::from(create_missing_cfs),
            );
        }
    }

    /// Specifies whether an error should be raised if the database already exists.
    ///
    /// Default: false
    pub fn set_error_if_exists(&mut self, enabled: bool) {
        unsafe {
            ffi::rocksdb_options_set_error_if_exists(self.inner, c_uchar::from(enabled));
        }
    }

    /// Enable/disable paranoid checks.
    ///
    /// If true, the implementation will do aggressive checking of the
    /// data it is processing and will stop early if it detects any
    /// errors. This may have unforeseen ramifications: for example, a
    /// corruption of one DB entry may cause a large number of entries to
    /// become unreadable or for the entire DB to become unopenable.
    /// If any of the  writes to the database fails (Put, Delete, Merge, Write),
    /// the database will switch to read-only mode and fail all other
    /// Write operations.
    ///
    /// Default: false
    pub fn set_paranoid_checks(&mut self, enabled: bool) {
        unsafe {
            ffi::rocksdb_options_set_paranoid_checks(self.inner, c_uchar::from(enabled));
        }
    }

    /// A list of paths where SST files can be put into, with its target size.
    /// Newer data is placed into paths specified earlier in the vector while
    /// older data gradually moves to paths specified later in the vector.
    ///
    /// For example, you have a flash device with 10GB allocated for the DB,
    /// as well as a hard drive of 2TB, you should config it to be:
    ///   [{"/flash_path", 10GB}, {"/hard_drive", 2TB}]
    ///
    /// The system will try to guarantee data under each path is close to but
    /// not larger than the target size. But current and future file sizes used
    /// by determining where to place a file are based on best-effort estimation,
    /// which means there is a chance that the actual size under the directory
    /// is slightly more than target size under some workloads. User should give
    /// some buffer room for those cases.
    ///
    /// If none of the paths has sufficient room to place a file, the file will
    /// be placed to the last path anyway, despite to the target size.
    ///
    /// Placing newer data to earlier paths is also best-efforts. User should
    /// expect user files to be placed in higher levels in some extreme cases.
    ///
    /// If left empty, only one path will be used, which is `path` passed when
    /// opening the DB.
    ///
    /// Default: empty
    pub fn set_db_paths(&mut self, paths: &[DBPath]) {
        let mut paths: Vec<_> = paths
            .iter()
            .map(|path| path.inner as *const ffi::rocksdb_dbpath_t)
            .collect();
        let num_paths = paths.len();
        unsafe {
            ffi::rocksdb_options_set_db_paths(self.inner, paths.as_mut_ptr(), num_paths);
        }
    }

    /// Use the specified object to interact with the environment,
    /// e.g. to read/write files, schedule background work, etc. In the near
    /// future, support for doing storage operations such as read/write files
    /// through env will be deprecated in favor of file_system.
    ///
    /// Default: Env::default()
    pub fn set_env(&mut self, env: &Env) {
        unsafe {
            ffi::rocksdb_options_set_env(self.inner, env.0.inner);
        }
        self.outlive.env = Some(env.clone());
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

    /// Sets the bottom-most compression algorithm that will be used for
    /// compressing blocks at the bottom-most level.
    ///
    /// Note that to actually unable bottom-most compression configuration after
    /// setting the compression type it needs to be enabled by calling
    /// [`set_bottommost_compression_options`](#method.set_bottommost_compression_options) or
    /// [`set_bottommost_zstd_max_train_bytes`](#method.set_bottommost_zstd_max_train_bytes) method with `enabled` argument
    /// set to `true`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rocksdb::{Options, DBCompressionType};
    ///
    /// let mut opts = Options::default();
    /// opts.set_bottommost_compression_type(DBCompressionType::Zstd);
    /// opts.set_bottommost_zstd_max_train_bytes(0, true);
    /// ```
    pub fn set_bottommost_compression_type(&mut self, t: DBCompressionType) {
        unsafe {
            ffi::rocksdb_options_set_bottommost_compression(self.inner, t as c_int);
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
            );
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

    /// Sets compression options for blocks at the bottom-most level.  Meaning
    /// of all settings is the same as in [`set_compression_options`](#method.set_compression_options) method but
    /// affect only the bottom-most compression which is set using
    /// [`set_bottommost_compression_type`](#method.set_bottommost_compression_type) method.
    ///
    /// # Examples
    ///
    /// ```
    /// use rocksdb::{Options, DBCompressionType};
    ///
    /// let mut opts = Options::default();
    /// opts.set_bottommost_compression_type(DBCompressionType::Zstd);
    /// opts.set_bottommost_compression_options(4, 5, 6, 7, true);
    /// ```
    pub fn set_bottommost_compression_options(
        &mut self,
        w_bits: c_int,
        level: c_int,
        strategy: c_int,
        max_dict_bytes: c_int,
        enabled: bool,
    ) {
        unsafe {
            ffi::rocksdb_options_set_bottommost_compression_options(
                self.inner,
                w_bits,
                level,
                strategy,
                max_dict_bytes,
                c_uchar::from(enabled),
            );
        }
    }

    /// Sets maximum size of training data passed to zstd's dictionary trainer. Using zstd's
    /// dictionary trainer can achieve even better compression ratio improvements than using
    /// `max_dict_bytes` alone.
    ///
    /// The training data will be used to generate a dictionary of max_dict_bytes.
    ///
    /// Default: 0.
    pub fn set_zstd_max_train_bytes(&mut self, value: c_int) {
        unsafe {
            ffi::rocksdb_options_set_compression_options_zstd_max_train_bytes(self.inner, value);
        }
    }

    /// Sets maximum size of training data passed to zstd's dictionary trainer
    /// when compressing the bottom-most level. Using zstd's dictionary trainer
    /// can achieve even better compression ratio improvements than using
    /// `max_dict_bytes` alone.
    ///
    /// The training data will be used to generate a dictionary of
    /// `max_dict_bytes`.
    ///
    /// Default: 0.
    pub fn set_bottommost_zstd_max_train_bytes(&mut self, value: c_int, enabled: bool) {
        unsafe {
            ffi::rocksdb_options_set_bottommost_compression_options_zstd_max_train_bytes(
                self.inner,
                value,
                c_uchar::from(enabled),
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
            ffi::rocksdb_options_compaction_readahead_size(self.inner, compaction_readahead_size);
        }
    }

    /// Allow RocksDB to pick dynamic base of bytes for levels.
    /// With this feature turned on, RocksDB will automatically adjust max bytes for each level.
    /// The goal of this feature is to have lower bound on size amplification.
    ///
    /// Default: false.
    pub fn set_level_compaction_dynamic_level_bytes(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_options_set_level_compaction_dynamic_level_bytes(
                self.inner,
                c_uchar::from(v),
            );
        }
    }

    pub fn set_merge_operator_associative<F: MergeFn + Clone>(
        &mut self,
        name: impl CStrLike,
        full_merge_fn: F,
    ) {
        let cb = Box::new(MergeOperatorCallback {
            name: name.into_c_string().unwrap(),
            full_merge_fn: full_merge_fn.clone(),
            partial_merge_fn: full_merge_fn,
        });

        unsafe {
            let mo = ffi::rocksdb_mergeoperator_create(
                Box::into_raw(cb).cast::<c_void>(),
                Some(merge_operator::destructor_callback::<F, F>),
                Some(full_merge_callback::<F, F>),
                Some(partial_merge_callback::<F, F>),
                Some(merge_operator::delete_callback),
                Some(merge_operator::name_callback::<F, F>),
            );
            ffi::rocksdb_options_set_merge_operator(self.inner, mo);
        }
    }

    pub fn set_merge_operator<F: MergeFn, PF: MergeFn>(
        &mut self,
        name: impl CStrLike,
        full_merge_fn: F,
        partial_merge_fn: PF,
    ) {
        let cb = Box::new(MergeOperatorCallback {
            name: name.into_c_string().unwrap(),
            full_merge_fn,
            partial_merge_fn,
        });

        unsafe {
            let mo = ffi::rocksdb_mergeoperator_create(
                Box::into_raw(cb).cast::<c_void>(),
                Some(merge_operator::destructor_callback::<F, PF>),
                Some(full_merge_callback::<F, PF>),
                Some(partial_merge_callback::<F, PF>),
                Some(merge_operator::delete_callback),
                Some(merge_operator::name_callback::<F, PF>),
            );
            ffi::rocksdb_options_set_merge_operator(self.inner, mo);
        }
    }

    #[deprecated(
        since = "0.5.0",
        note = "add_merge_operator has been renamed to set_merge_operator"
    )]
    pub fn add_merge_operator<F: MergeFn + Clone>(&mut self, name: &str, merge_fn: F) {
        self.set_merge_operator_associative(name, merge_fn);
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
    pub fn set_compaction_filter<F>(&mut self, name: impl CStrLike, filter_fn: F)
    where
        F: CompactionFilterFn + Send + 'static,
    {
        let cb = Box::new(CompactionFilterCallback {
            name: name.into_c_string().unwrap(),
            filter_fn,
        });

        unsafe {
            let cf = ffi::rocksdb_compactionfilter_create(
                Box::into_raw(cb).cast::<c_void>(),
                Some(compaction_filter::destructor_callback::<CompactionFilterCallback<F>>),
                Some(compaction_filter::filter_callback::<CompactionFilterCallback<F>>),
                Some(compaction_filter::name_callback::<CompactionFilterCallback<F>>),
            );
            ffi::rocksdb_options_set_compaction_filter(self.inner, cf);
        }
    }

    /// This is a factory that provides compaction filter objects which allow
    /// an application to modify/delete a key-value during background compaction.
    ///
    /// A new filter will be created on each compaction run.  If multithreaded
    /// compaction is being used, each created CompactionFilter will only be used
    /// from a single thread and so does not need to be thread-safe.
    ///
    /// Default: nullptr
    pub fn set_compaction_filter_factory<F>(&mut self, factory: F)
    where
        F: CompactionFilterFactory + 'static,
    {
        let factory = Box::new(factory);

        unsafe {
            let cff = ffi::rocksdb_compactionfilterfactory_create(
                Box::into_raw(factory).cast::<c_void>(),
                Some(compaction_filter_factory::destructor_callback::<F>),
                Some(compaction_filter_factory::create_compaction_filter_callback::<F>),
                Some(compaction_filter_factory::name_callback::<F>),
            );

            ffi::rocksdb_options_set_compaction_filter_factory(self.inner, cff);
        }
    }

    /// Sets the comparator used to define the order of keys in the table.
    /// Default: a comparator that uses lexicographic byte-wise ordering
    ///
    /// The client must ensure that the comparator supplied here has the same
    /// name and orders keys *exactly* the same as the comparator provided to
    /// previous open calls on the same DB.
    pub fn set_comparator(&mut self, name: impl CStrLike, compare_fn: Box<CompareFn>) {
        let cb = Box::new(ComparatorCallback {
            name: name.into_c_string().unwrap(),
            f: compare_fn,
        });

        unsafe {
            let cmp = ffi::rocksdb_comparator_create(
                Box::into_raw(cb).cast::<c_void>(),
                Some(comparator::destructor_callback),
                Some(comparator::compare_callback),
                Some(comparator::name_callback),
            );
            ffi::rocksdb_options_set_comparator(self.inner, cmp);
        }
    }

    pub fn set_prefix_extractor(&mut self, prefix_extractor: SliceTransform) {
        unsafe {
            ffi::rocksdb_options_set_prefix_extractor(self.inner, prefix_extractor.inner);
        }
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
                c_int::from(optimize_for_hits),
            );
        }
    }

    /// Sets the periodicity when obsolete files get deleted.
    ///
    /// The files that get out of scope by compaction
    /// process will still get automatically delete on every compaction,
    /// regardless of this setting.
    ///
    /// Default: 6 hours
    pub fn set_delete_obsolete_files_period_micros(&mut self, micros: u64) {
        unsafe {
            ffi::rocksdb_options_set_delete_obsolete_files_period_micros(self.inner, micros);
        }
    }

    /// Prepare the DB for bulk loading.
    ///
    /// All data will be in level 0 without any automatic compaction.
    /// It's recommended to manually call CompactRange(NULL, NULL) before reading
    /// from the database, because otherwise the read can be very slow.
    pub fn prepare_for_bulk_load(&mut self) {
        unsafe {
            ffi::rocksdb_options_prepare_for_bulk_load(self.inner);
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

    /// If max_open_files is -1, DB will open all files on DB::Open(). You can
    /// use this option to increase the number of threads used to open the files.
    /// Default: 16
    pub fn set_max_file_opening_threads(&mut self, nthreads: c_int) {
        unsafe {
            ffi::rocksdb_options_set_max_file_opening_threads(self.inner, nthreads);
        }
    }

    /// By default, writes to stable storage use fdatasync (on platforms
    /// where this function is available). If this option is true,
    /// fsync is used instead.
    ///
    /// fsync and fdatasync are equally safe for our purposes and fdatasync is
    /// faster, so it is rarely necessary to set this option. It is provided
    /// as a workaround for kernel/filesystem bugs, such as one that affected
    /// fdatasync with ext4 in kernel versions prior to 3.7.
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
        unsafe {
            ffi::rocksdb_options_set_use_fsync(self.inner, c_int::from(useit));
        }
    }

    /// Specifies the absolute info LOG dir.
    ///
    /// If it is empty, the log files will be in the same dir as data.
    /// If it is non empty, the log files will be in the specified dir,
    /// and the db data dir's absolute path will be used as the log file
    /// name's prefix.
    ///
    /// Default: empty
    pub fn set_db_log_dir<P: AsRef<Path>>(&mut self, path: P) {
        let p = to_cpath(path).unwrap();
        unsafe {
            ffi::rocksdb_options_set_db_log_dir(self.inner, p.as_ptr());
        }
    }

    /// Specifies the log level.
    /// Consider the `LogLevel` enum for a list of possible levels.
    ///
    /// Default: Info
    ///
    /// # Examples
    ///
    /// ```
    /// use rocksdb::{Options, LogLevel};
    ///
    /// let mut opts = Options::default();
    /// opts.set_log_level(LogLevel::Warn);
    /// ```
    pub fn set_log_level(&mut self, level: LogLevel) {
        unsafe {
            ffi::rocksdb_options_set_info_log_level(self.inner, level as c_int);
        }
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

    /// Same as bytes_per_sync, but applies to WAL files.
    ///
    /// Default: 0, turned off
    ///
    /// Dynamically changeable through SetDBOptions() API.
    pub fn set_wal_bytes_per_sync(&mut self, nbytes: u64) {
        unsafe {
            ffi::rocksdb_options_set_wal_bytes_per_sync(self.inner, nbytes);
        }
    }

    /// Sets the maximum buffer size that is used by WritableFileWriter.
    ///
    /// On Windows, we need to maintain an aligned buffer for writes.
    /// We allow the buffer to grow until it's size hits the limit in buffered
    /// IO and fix the buffer size when using direct IO to ensure alignment of
    /// write requests if the logical sector size is unusual
    ///
    /// Default: 1024 * 1024 (1 MB)
    ///
    /// Dynamically changeable through SetDBOptions() API.
    pub fn set_writable_file_max_buffer_size(&mut self, nbytes: u64) {
        unsafe {
            ffi::rocksdb_options_set_writable_file_max_buffer_size(self.inner, nbytes);
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
            ffi::rocksdb_options_set_allow_concurrent_memtable_write(
                self.inner,
                c_uchar::from(allow),
            );
        }
    }

    /// If true, threads synchronizing with the write batch group leader will wait for up to
    /// write_thread_max_yield_usec before blocking on a mutex. This can substantially improve
    /// throughput for concurrent workloads, regardless of whether allow_concurrent_memtable_write
    /// is enabled.
    ///
    /// Default: true
    pub fn set_enable_write_thread_adaptive_yield(&mut self, enabled: bool) {
        unsafe {
            ffi::rocksdb_options_set_enable_write_thread_adaptive_yield(
                self.inner,
                c_uchar::from(enabled),
            );
        }
    }

    /// Specifies whether an iteration->Next() sequentially skips over keys with the same user-key or not.
    ///
    /// This number specifies the number of keys (with the same userkey)
    /// that will be sequentially skipped before a reseek is issued.
    ///
    /// Default: 8
    pub fn set_max_sequential_skip_in_iterations(&mut self, num: u64) {
        unsafe {
            ffi::rocksdb_options_set_max_sequential_skip_in_iterations(self.inner, num);
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
            ffi::rocksdb_options_set_use_direct_reads(self.inner, c_uchar::from(enabled));
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
                c_uchar::from(enabled),
            );
        }
    }

    /// Enable/dsiable child process inherit open files.
    ///
    /// Default: true
    pub fn set_is_fd_close_on_exec(&mut self, enabled: bool) {
        unsafe {
            ffi::rocksdb_options_set_is_fd_close_on_exec(self.inner, c_uchar::from(enabled));
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
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// #[allow(deprecated)]
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

    /// By default target_file_size_multiplier is 1, which means
    /// by default files in different levels will have similar size.
    ///
    /// Dynamically changeable through SetOptions() API
    pub fn set_target_file_size_multiplier(&mut self, multiplier: i32) {
        unsafe {
            ffi::rocksdb_options_set_target_file_size_multiplier(self.inner, multiplier as c_int);
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

    /// Sets the options needed to support Universal Style compactions.
    pub fn set_universal_compaction_options(&mut self, uco: &UniversalCompactOptions) {
        unsafe {
            ffi::rocksdb_options_set_universal_compaction_options(self.inner, uco.inner);
        }
    }

    /// Sets the options for FIFO compaction style.
    pub fn set_fifo_compaction_options(&mut self, fco: &FifoCompactOptions) {
        unsafe {
            ffi::rocksdb_options_set_fifo_compaction_options(self.inner, fco.inner);
        }
    }

    /// Sets unordered_write to true trades higher write throughput with
    /// relaxing the immutability guarantee of snapshots. This violates the
    /// repeatability one expects from ::Get from a snapshot, as well as
    /// ::MultiGet and Iterator's consistent-point-in-time view property.
    /// If the application cannot tolerate the relaxed guarantees, it can implement
    /// its own mechanisms to work around that and yet benefit from the higher
    /// throughput. Using TransactionDB with WRITE_PREPARED write policy and
    /// two_write_queues=true is one way to achieve immutable snapshots despite
    /// unordered_write.
    ///
    /// By default, i.e., when it is false, rocksdb does not advance the sequence
    /// number for new snapshots unless all the writes with lower sequence numbers
    /// are already finished. This provides the immutability that we except from
    /// snapshots. Moreover, since Iterator and MultiGet internally depend on
    /// snapshots, the snapshot immutability results into Iterator and MultiGet
    /// offering consistent-point-in-time view. If set to true, although
    /// Read-Your-Own-Write property is still provided, the snapshot immutability
    /// property is relaxed: the writes issued after the snapshot is obtained (with
    /// larger sequence numbers) will be still not visible to the reads from that
    /// snapshot, however, there still might be pending writes (with lower sequence
    /// number) that will change the state visible to the snapshot after they are
    /// landed to the memtable.
    ///
    /// Default: false
    pub fn set_unordered_write(&mut self, unordered: bool) {
        unsafe {
            ffi::rocksdb_options_set_unordered_write(self.inner, c_uchar::from(unordered));
        }
    }

    /// Sets maximum number of threads that will
    /// concurrently perform a compaction job by breaking it into multiple,
    /// smaller ones that are run simultaneously.
    ///
    /// Default: 1 (i.e. no subcompactions)
    pub fn set_max_subcompactions(&mut self, num: u32) {
        unsafe {
            ffi::rocksdb_options_set_max_subcompactions(self.inner, num);
        }
    }

    /// Sets maximum number of concurrent background jobs
    /// (compactions and flushes).
    ///
    /// Default: 2
    ///
    /// Dynamically changeable through SetDBOptions() API.
    pub fn set_max_background_jobs(&mut self, jobs: c_int) {
        unsafe {
            ffi::rocksdb_options_set_max_background_jobs(self.inner, jobs);
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
    /// #[allow(deprecated)]
    /// opts.set_max_background_compactions(2);
    /// ```
    #[deprecated(
        since = "0.15.0",
        note = "RocksDB automatically decides this based on the value of max_background_jobs"
    )]
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
    /// #[allow(deprecated)]
    /// opts.set_max_background_flushes(2);
    /// ```
    #[deprecated(
        since = "0.15.0",
        note = "RocksDB automatically decides this based on the value of max_background_jobs"
    )]
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
        unsafe {
            ffi::rocksdb_options_set_disable_auto_compactions(self.inner, c_int::from(disable));
        }
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
        unsafe {
            ffi::rocksdb_options_set_memtable_huge_page_size(self.inner, size);
        }
    }

    /// Sets the maximum number of successive merge operations on a key in the memtable.
    ///
    /// When a merge operation is added to the memtable and the maximum number of
    /// successive merges is reached, the value of the key will be calculated and
    /// inserted into the memtable instead of the merge operation. This will
    /// ensure that there are never more than max_successive_merges merge
    /// operations in the memtable.
    ///
    /// Default: 0 (disabled)
    pub fn set_max_successive_merges(&mut self, num: usize) {
        unsafe {
            ffi::rocksdb_options_set_max_successive_merges(self.inner, num);
        }
    }

    /// Control locality of bloom filter probes to improve cache miss rate.
    /// This option only applies to memtable prefix bloom and plaintable
    /// prefix bloom. It essentially limits the max number of cache lines each
    /// bloom filter check can touch.
    ///
    /// This optimization is turned off when set to 0. The number should never
    /// be greater than number of probes. This option can boost performance
    /// for in-memory workload but should use with care since it can cause
    /// higher false positive rate.
    ///
    /// Default: 0
    pub fn set_bloom_locality(&mut self, v: u32) {
        unsafe {
            ffi::rocksdb_options_set_bloom_locality(self.inner, v);
        }
    }

    /// Enable/disable thread-safe inplace updates.
    ///
    /// Requires updates if
    /// * key exists in current memtable
    /// * new sizeof(new_value) <= sizeof(old_value)
    /// * old_value for that key is a put i.e. kTypeValue
    ///
    /// Default: false.
    pub fn set_inplace_update_support(&mut self, enabled: bool) {
        unsafe {
            ffi::rocksdb_options_set_inplace_update_support(self.inner, c_uchar::from(enabled));
        }
    }

    /// Sets the number of locks used for inplace update.
    ///
    /// Default: 10000 when inplace_update_support = true, otherwise 0.
    pub fn set_inplace_update_locks(&mut self, num: usize) {
        unsafe {
            ffi::rocksdb_options_set_inplace_update_num_locks(self.inner, num);
        }
    }

    /// Different max-size multipliers for different levels.
    /// These are multiplied by max_bytes_for_level_multiplier to arrive
    /// at the max-size of each level.
    ///
    /// Default: 1
    ///
    /// Dynamically changeable through SetOptions() API
    pub fn set_max_bytes_for_level_multiplier_additional(&mut self, level_values: &[i32]) {
        let count = level_values.len();
        unsafe {
            ffi::rocksdb_options_set_max_bytes_for_level_multiplier_additional(
                self.inner,
                level_values.as_ptr() as *mut c_int,
                count,
            );
        }
    }

    /// If true, then DB::Open() will not fetch and check sizes of all sst files.
    /// This may significantly speed up startup if there are many sst files,
    /// especially when using non-default Env with expensive GetFileSize().
    /// We'll still check that all required sst files exist.
    /// If paranoid_checks is false, this option is ignored, and sst files are
    /// not checked at all.
    ///
    /// Default: false
    pub fn set_skip_checking_sst_file_sizes_on_db_open(&mut self, value: bool) {
        unsafe {
            ffi::rocksdb_options_set_skip_checking_sst_file_sizes_on_db_open(
                self.inner,
                c_uchar::from(value),
            );
        }
    }

    /// The total maximum size(bytes) of write buffers to maintain in memory
    /// including copies of buffers that have already been flushed. This parameter
    /// only affects trimming of flushed buffers and does not affect flushing.
    /// This controls the maximum amount of write history that will be available
    /// in memory for conflict checking when Transactions are used. The actual
    /// size of write history (flushed Memtables) might be higher than this limit
    /// if further trimming will reduce write history total size below this
    /// limit. For example, if max_write_buffer_size_to_maintain is set to 64MB,
    /// and there are three flushed Memtables, with sizes of 32MB, 20MB, 20MB.
    /// Because trimming the next Memtable of size 20MB will reduce total memory
    /// usage to 52MB which is below the limit, RocksDB will stop trimming.
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
    /// after they are flushed. If this value is set to -1,
    /// 'max_write_buffer_number * write_buffer_size' will be used.
    ///
    /// Default:
    /// If using a TransactionDB/OptimisticTransactionDB, the default value will
    /// be set to the value of 'max_write_buffer_number * write_buffer_size'
    /// if it is not explicitly set by the user.  Otherwise, the default is 0.
    pub fn set_max_write_buffer_size_to_maintain(&mut self, size: i64) {
        unsafe {
            ffi::rocksdb_options_set_max_write_buffer_size_to_maintain(self.inner, size);
        }
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
        unsafe {
            ffi::rocksdb_options_set_enable_pipelined_write(self.inner, c_uchar::from(value));
        }
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
        self.outlive.block_based = Some(factory.outlive.clone());
    }

    /// Sets the table factory to a CuckooTableFactory (the default table
    /// factory is a block-based table factory that provides a default
    /// implementation of TableBuilder and TableReader with default
    /// BlockBasedTableOptions).
    /// See official [wiki](https://github.com/facebook/rocksdb/wiki/CuckooTable-Format) for more information on this table format.
    /// # Examples
    ///
    /// ```
    /// use rocksdb::{Options, CuckooTableOptions};
    ///
    /// let mut opts = Options::default();
    /// let mut factory_opts = CuckooTableOptions::default();
    /// factory_opts.set_hash_ratio(0.8);
    /// factory_opts.set_max_search_depth(20);
    /// factory_opts.set_cuckoo_block_size(10);
    /// factory_opts.set_identity_as_first_hash(true);
    /// factory_opts.set_use_module_hash(false);
    ///
    /// opts.set_cuckoo_table_factory(&factory_opts);
    /// ```
    pub fn set_cuckoo_table_factory(&mut self, factory: &CuckooTableOptions) {
        unsafe {
            ffi::rocksdb_options_set_cuckoo_table_factory(self.inner, factory.inner);
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

    /// Sets the start level to use compression.
    pub fn set_min_level_to_compress(&mut self, lvl: c_int) {
        unsafe {
            ffi::rocksdb_options_set_min_level_to_compress(self.inner, lvl);
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
            ffi::rocksdb_options_set_report_bg_io_stats(self.inner, c_int::from(enable));
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

    /// If not zero, dump rocksdb.stats to RocksDB to LOG every `stats_persist_period_sec`.
    ///
    /// Default: `600` (10 mins)
    ///
    /// # Examples
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut opts = Options::default();
    /// opts.set_stats_persist_period_sec(5);
    /// ```
    pub fn set_stats_persist_period_sec(&mut self, period: c_uint) {
        unsafe {
            ffi::rocksdb_options_set_stats_persist_period_sec(self.inner, period);
        }
    }

    /// When set to true, reading SST files will opt out of the filesystem's
    /// readahead. Setting this to false may improve sequential iteration
    /// performance.
    ///
    /// Default: `true`
    pub fn set_advise_random_on_open(&mut self, advise: bool) {
        unsafe {
            ffi::rocksdb_options_set_advise_random_on_open(self.inner, c_uchar::from(advise));
        }
    }

    /// Specifies the file access pattern once a compaction is started.
    ///
    /// It will be applied to all input files of a compaction.
    ///
    /// Default: Normal
    pub fn set_access_hint_on_compaction_start(&mut self, pattern: AccessHint) {
        unsafe {
            ffi::rocksdb_options_set_access_hint_on_compaction_start(self.inner, pattern as c_int);
        }
    }

    /// Enable/disable adaptive mutex, which spins in the user space before resorting to kernel.
    ///
    /// This could reduce context switch when the mutex is not
    /// heavily contended. However, if the mutex is hot, we could end up
    /// wasting spin time.
    ///
    /// Default: false
    pub fn set_use_adaptive_mutex(&mut self, enabled: bool) {
        unsafe {
            ffi::rocksdb_options_set_use_adaptive_mutex(self.inner, c_uchar::from(enabled));
        }
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

    /// Sets the maximum number of bytes in all compacted files.
    /// We try to limit number of bytes in one compaction to be lower than this
    /// threshold. But it's not guaranteed.
    ///
    /// Value 0 will be sanitized.
    ///
    /// Default: target_file_size_base * 25
    pub fn set_max_compaction_bytes(&mut self, nbytes: u64) {
        unsafe {
            ffi::rocksdb_options_set_max_compaction_bytes(self.inner, nbytes);
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
        let p = to_cpath(path).unwrap();
        unsafe {
            ffi::rocksdb_options_set_wal_dir(self.inner, p.as_ptr());
        }
    }

    /// Sets the WAL ttl in seconds.
    ///
    /// The following two options affect how archived logs will be deleted.
    /// 1. If both set to 0, logs will be deleted asap and will not get into
    ///    the archive.
    /// 2. If wal_ttl_seconds is 0 and wal_size_limit_mb is not 0,
    ///    WAL files will be checked every 10 min and if total size is greater
    ///    then wal_size_limit_mb, they will be deleted starting with the
    ///    earliest until size_limit is met. All empty files will be deleted.
    /// 3. If wal_ttl_seconds is not 0 and wall_size_limit_mb is 0, then
    ///    WAL files will be checked every wal_ttl_seconds / 2 and those that
    ///    are older than wal_ttl_seconds will be deleted.
    /// 4. If both are not 0, WAL files will be checked every 10 min and both
    ///    checks will be performed with ttl being first.
    ///
    /// Default: 0
    pub fn set_wal_ttl_seconds(&mut self, secs: u64) {
        unsafe {
            ffi::rocksdb_options_set_WAL_ttl_seconds(self.inner, secs);
        }
    }

    /// Sets the WAL size limit in MB.
    ///
    /// If total size of WAL files is greater then wal_size_limit_mb,
    /// they will be deleted starting with the earliest until size_limit is met.
    ///
    /// Default: 0
    pub fn set_wal_size_limit_mb(&mut self, size: u64) {
        unsafe {
            ffi::rocksdb_options_set_WAL_size_limit_MB(self.inner, size);
        }
    }

    /// Sets the number of bytes to preallocate (via fallocate) the manifest files.
    ///
    /// Default is 4MB, which is reasonable to reduce random IO
    /// as well as prevent overallocation for mounts that preallocate
    /// large amounts of data (such as xfs's allocsize option).
    pub fn set_manifest_preallocation_size(&mut self, size: usize) {
        unsafe {
            ffi::rocksdb_options_set_manifest_preallocation_size(self.inner, size);
        }
    }

    /// If true, then DB::Open() will not update the statistics used to optimize
    /// compaction decision by loading table properties from many files.
    /// Turning off this feature will improve DBOpen time especially in disk environment.
    ///
    /// Default: false
    pub fn set_skip_stats_update_on_db_open(&mut self, skip: bool) {
        unsafe {
            ffi::rocksdb_options_set_skip_stats_update_on_db_open(self.inner, c_uchar::from(skip));
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
            ffi::rocksdb_options_set_allow_mmap_writes(self.inner, c_uchar::from(is_enabled));
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
            ffi::rocksdb_options_set_allow_mmap_reads(self.inner, c_uchar::from(is_enabled));
        }
    }

    /// If enabled, WAL is not flushed automatically after each write. Instead it
    /// relies on manual invocation of `DB::flush_wal()` to write the WAL buffer
    /// to its file.
    ///
    /// Default: false
    ///
    /// # Examples
    ///
    /// ```
    /// use rocksdb::Options;
    ///
    /// let mut options = Options::default();
    /// options.set_manual_wal_flush(true);
    /// ```
    pub fn set_manual_wal_flush(&mut self, is_enabled: bool) {
        unsafe {
            ffi::rocksdb_options_set_manual_wal_flush(self.inner, c_uchar::from(is_enabled));
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
            ffi::rocksdb_options_set_atomic_flush(self.inner, c_uchar::from(atomic_flush));
        }
    }

    /// Sets global cache for table-level rows. Cache must outlive DB instance which uses it.
    ///
    /// Default: null (disabled)
    /// Not supported in ROCKSDB_LITE mode!
    pub fn set_row_cache(&mut self, cache: &Cache) {
        unsafe {
            ffi::rocksdb_options_set_row_cache(self.inner, cache.0.inner.as_ptr());
        }
        self.outlive.row_cache = Some(cache.clone());
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

    /// Sets the time for the info log file to roll (in seconds).
    ///
    /// If specified with non-zero value, log file will be rolled
    /// if it has been active longer than `log_file_time_to_roll`.
    /// Default: 0 (disabled)
    pub fn set_log_file_time_to_roll(&mut self, secs: usize) {
        unsafe {
            ffi::rocksdb_options_set_log_file_time_to_roll(self.inner, secs);
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

    /// Sets the threshold at which all writes will be slowed down to at least delayed_write_rate if estimated
    /// bytes needed to be compaction exceed this threshold.
    ///
    /// Default: 64GB
    pub fn set_soft_pending_compaction_bytes_limit(&mut self, limit: usize) {
        unsafe {
            ffi::rocksdb_options_set_soft_pending_compaction_bytes_limit(self.inner, limit);
        }
    }

    /// Sets the bytes threshold at which all writes are stopped if estimated bytes needed to be compaction exceed
    /// this threshold.
    ///
    /// Default: 256GB
    pub fn set_hard_pending_compaction_bytes_limit(&mut self, limit: usize) {
        unsafe {
            ffi::rocksdb_options_set_hard_pending_compaction_bytes_limit(self.inner, limit);
        }
    }

    /// Sets the size of one block in arena memory allocation.
    ///
    /// If <= 0, a proper value is automatically calculated (usually 1/10 of
    /// writer_buffer_size).
    ///
    /// Default: 0
    pub fn set_arena_block_size(&mut self, size: usize) {
        unsafe {
            ffi::rocksdb_options_set_arena_block_size(self.inner, size);
        }
    }

    /// If true, then print malloc stats together with rocksdb.stats when printing to LOG.
    ///
    /// Default: false
    pub fn set_dump_malloc_stats(&mut self, enabled: bool) {
        unsafe {
            ffi::rocksdb_options_set_dump_malloc_stats(self.inner, c_uchar::from(enabled));
        }
    }

    /// Enable whole key bloom filter in memtable. Note this will only take effect
    /// if memtable_prefix_bloom_size_ratio is not 0. Enabling whole key filtering
    /// can potentially reduce CPU usage for point-look-ups.
    ///
    /// Default: false (disable)
    ///
    /// Dynamically changeable through SetOptions() API
    pub fn set_memtable_whole_key_filtering(&mut self, whole_key_filter: bool) {
        unsafe {
            ffi::rocksdb_options_set_memtable_whole_key_filtering(
                self.inner,
                c_uchar::from(whole_key_filter),
            );
        }
    }

    /// Enable the use of key-value separation.
    ///
    /// More details can be found here: [Integrated BlobDB](http://rocksdb.org/blog/2021/05/26/integrated-blob-db.html).
    ///
    /// Default: false (disable)
    ///
    /// Dynamically changeable through SetOptions() API
    pub fn set_enable_blob_files(&mut self, val: bool) {
        unsafe {
            ffi::rocksdb_options_set_enable_blob_files(self.inner, u8::from(val));
        }
    }

    /// Sets the minimum threshold value at or above which will be written
    /// to blob files during flush or compaction.
    ///
    /// Dynamically changeable through SetOptions() API
    pub fn set_min_blob_size(&mut self, val: u64) {
        unsafe {
            ffi::rocksdb_options_set_min_blob_size(self.inner, val);
        }
    }

    /// Sets the size limit for blob files.
    ///
    /// Dynamically changeable through SetOptions() API
    pub fn set_blob_file_size(&mut self, val: u64) {
        unsafe {
            ffi::rocksdb_options_set_blob_file_size(self.inner, val);
        }
    }

    /// Sets the blob compression type. All blob files use the same
    /// compression type.
    ///
    /// Dynamically changeable through SetOptions() API
    pub fn set_blob_compression_type(&mut self, val: DBCompressionType) {
        unsafe {
            ffi::rocksdb_options_set_blob_compression_type(self.inner, val as _);
        }
    }

    /// If this is set to true RocksDB will actively relocate valid blobs from the oldest blob files
    /// as they are encountered during compaction.
    ///
    /// Dynamically changeable through SetOptions() API
    pub fn set_enable_blob_gc(&mut self, val: bool) {
        unsafe {
            ffi::rocksdb_options_set_enable_blob_gc(self.inner, u8::from(val));
        }
    }

    /// Sets the threshold that the GC logic uses to determine which blob files should be considered “old.”
    ///
    /// For example, the default value of 0.25 signals to RocksDB that blobs residing in the
    /// oldest 25% of blob files should be relocated by GC. This parameter can be tuned to adjust
    /// the trade-off between write amplification and space amplification.
    ///
    /// Dynamically changeable through SetOptions() API
    pub fn set_blob_gc_age_cutoff(&mut self, val: c_double) {
        unsafe {
            ffi::rocksdb_options_set_blob_gc_age_cutoff(self.inner, val);
        }
    }

    /// Sets the blob GC force threshold.
    ///
    /// Dynamically changeable through SetOptions() API
    pub fn set_blob_gc_force_threshold(&mut self, val: c_double) {
        unsafe {
            ffi::rocksdb_options_set_blob_gc_force_threshold(self.inner, val);
        }
    }

    /// Sets the blob compaction read ahead size.
    ///
    /// Dynamically changeable through SetOptions() API
    pub fn set_blob_compaction_readahead_size(&mut self, val: u64) {
        unsafe {
            ffi::rocksdb_options_set_blob_compaction_readahead_size(self.inner, val);
        }
    }

    /// The memory usage of memtable will report to this object. The same object
    /// can be passed into multiple DBs and it will track the sum of size of all
    /// the DBs. If the total size of all live memtables of all the DBs exceeds
    /// a limit, a flush will be triggered in the next DB to which the next write
    /// is issued, as long as there is one or more column family not already
    /// flushing.
    ///
    /// If the object is only passed to one DB, the behavior is the same as
    /// db_write_buffer_size. When write_buffer_manager is set, the value set will
    /// override db_write_buffer_size.
    ///
    /// This feature is disabled by default. Specify a non-zero value
    /// to enable it.
    ///
    /// Default: null
    pub fn set_write_buffer_manager(&mut self, write_buffer_manager: &WriteBufferManager) {
        unsafe {
            ffi::rocksdb_options_set_write_buffer_manager(self.inner, write_buffer_manager.inner);
        }
    }
}

impl Default for Options {
    fn default() -> Self {
        unsafe {
            let opts = ffi::rocksdb_options_create();
            assert!(!opts.is_null(), "Could not create RocksDB options");

            Self {
                inner: opts,
                outlive: OptionsMustOutliveDB::default(),
            }
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
            ffi::rocksdb_flushoptions_set_wait(self.inner, c_uchar::from(wait));
        }
    }
}

impl Default for FlushOptions {
    fn default() -> Self {
        let flush_opts = unsafe { ffi::rocksdb_flushoptions_create() };
        assert!(
            !flush_opts.is_null(),
            "Could not create RocksDB flush options"
        );

        Self { inner: flush_opts }
    }
}

impl WriteOptions {
    pub fn new() -> WriteOptions {
        WriteOptions::default()
    }

    /// Sets the sync mode. If true, the write will be flushed
    /// from the operating system buffer cache before the write is considered complete.
    /// If this flag is true, writes will be slower.
    ///
    /// Default: false
    pub fn set_sync(&mut self, sync: bool) {
        unsafe {
            ffi::rocksdb_writeoptions_set_sync(self.inner, c_uchar::from(sync));
        }
    }

    /// Sets whether WAL should be active or not.
    /// If true, writes will not first go to the write ahead log,
    /// and the write may got lost after a crash.
    ///
    /// Default: false
    pub fn disable_wal(&mut self, disable: bool) {
        unsafe {
            ffi::rocksdb_writeoptions_disable_WAL(self.inner, c_int::from(disable));
        }
    }

    /// If true and if user is trying to write to column families that don't exist (they were dropped),
    /// ignore the write (don't return an error). If there are multiple writes in a WriteBatch,
    /// other writes will succeed.
    ///
    /// Default: false
    pub fn set_ignore_missing_column_families(&mut self, ignore: bool) {
        unsafe {
            ffi::rocksdb_writeoptions_set_ignore_missing_column_families(
                self.inner,
                c_uchar::from(ignore),
            );
        }
    }

    /// If true and we need to wait or sleep for the write request, fails
    /// immediately with Status::Incomplete().
    ///
    /// Default: false
    pub fn set_no_slowdown(&mut self, no_slowdown: bool) {
        unsafe {
            ffi::rocksdb_writeoptions_set_no_slowdown(self.inner, c_uchar::from(no_slowdown));
        }
    }

    /// If true, this write request is of lower priority if compaction is
    /// behind. In this case, no_slowdown = true, the request will be cancelled
    /// immediately with Status::Incomplete() returned. Otherwise, it will be
    /// slowed down. The slowdown value is determined by RocksDB to guarantee
    /// it introduces minimum impacts to high priority writes.
    ///
    /// Default: false
    pub fn set_low_pri(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_writeoptions_set_low_pri(self.inner, c_uchar::from(v));
        }
    }

    /// If true, writebatch will maintain the last insert positions of each
    /// memtable as hints in concurrent write. It can improve write performance
    /// in concurrent writes if keys in one writebatch are sequential. In
    /// non-concurrent writes (when concurrent_memtable_writes is false) this
    /// option will be ignored.
    ///
    /// Default: false
    pub fn set_memtable_insert_hint_per_batch(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_writeoptions_set_memtable_insert_hint_per_batch(
                self.inner,
                c_uchar::from(v),
            );
        }
    }
}

impl Default for WriteOptions {
    fn default() -> Self {
        let write_opts = unsafe { ffi::rocksdb_writeoptions_create() };
        assert!(
            !write_opts.is_null(),
            "Could not create RocksDB write options"
        );

        Self { inner: write_opts }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde1", derive(serde::Serialize, serde::Deserialize))]
#[repr(i32)]
pub enum ReadTier {
    /// Reads data in memtable, block cache, OS cache or storage.
    All = 0,
    /// Reads data in memtable or block cache.
    BlockCache,
}

impl ReadOptions {
    // TODO add snapshot setting here
    // TODO add snapshot wrapper structs with proper destructors;
    // that struct needs an "iterator" impl too.

    /// Specify whether the "data block"/"index block"/"filter block"
    /// read for this iteration should be cached in memory?
    /// Callers may wish to set this field to false for bulk scans.
    ///
    /// Default: true
    pub fn fill_cache(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_readoptions_set_fill_cache(self.inner, c_uchar::from(v));
        }
    }

    /// Sets the snapshot which should be used for the read.
    /// The snapshot must belong to the DB that is being read and must
    /// not have been released.
    pub fn set_snapshot<D: DBAccess>(&mut self, snapshot: &SnapshotWithThreadMode<D>) {
        unsafe {
            ffi::rocksdb_readoptions_set_snapshot(self.inner, snapshot.inner);
        }
    }

    /// Sets the lower bound for an iterator.
    pub fn set_iterate_lower_bound<K: Into<Vec<u8>>>(&mut self, key: K) {
        self.set_lower_bound_impl(Some(key.into()));
    }

    /// Sets the upper bound for an iterator.
    /// The upper bound itself is not included on the iteration result.
    pub fn set_iterate_upper_bound<K: Into<Vec<u8>>>(&mut self, key: K) {
        self.set_upper_bound_impl(Some(key.into()));
    }

    /// Sets lower and upper bounds based on the provided range.  This is
    /// similar to setting lower and upper bounds separately except that it also
    /// allows either bound to be reset.
    ///
    /// The argument can be a regular Rust range, e.g. `lower..upper`.  However,
    /// since RocksDB upper bound is always excluded (i.e. range can never be
    /// fully closed) inclusive ranges (`lower..=upper` and `..=upper`) are not
    /// supported.  For example:
    ///
    /// ```
    /// let mut options = rocksdb::ReadOptions::default();
    /// options.set_iterate_range("xy".as_bytes().."xz".as_bytes());
    /// ```
    ///
    /// In addition, [`crate::PrefixRange`] can be used to specify a range of
    /// keys with a given prefix.  In particular, the above example is
    /// equivalent to:
    ///
    /// ```
    /// let mut options = rocksdb::ReadOptions::default();
    /// options.set_iterate_range(rocksdb::PrefixRange("xy".as_bytes()));
    /// ```
    ///
    /// Note that setting range using this method is separate to using prefix
    /// iterators.  Prefix iterators use prefix extractor configured for
    /// a column family.  Setting bounds via [`crate::PrefixRange`] is more akin
    /// to using manual prefix.
    ///
    /// Using this method clears any previously set bounds.  In other words, the
    /// bounds can be reset by setting the range to `..` as in:
    ///
    /// ```
    /// let mut options = rocksdb::ReadOptions::default();
    /// options.set_iterate_range(..);
    /// ```
    pub fn set_iterate_range(&mut self, range: impl crate::IterateBounds) {
        let (lower, upper) = range.into_bounds();
        self.set_lower_bound_impl(lower);
        self.set_upper_bound_impl(upper);
    }

    fn set_lower_bound_impl(&mut self, bound: Option<Vec<u8>>) {
        let (ptr, len) = if let Some(ref bound) = bound {
            (bound.as_ptr() as *const c_char, bound.len())
        } else if self.iterate_lower_bound.is_some() {
            (std::ptr::null(), 0)
        } else {
            return;
        };
        self.iterate_lower_bound = bound;
        unsafe {
            ffi::rocksdb_readoptions_set_iterate_lower_bound(self.inner, ptr, len);
        }
    }

    fn set_upper_bound_impl(&mut self, bound: Option<Vec<u8>>) {
        let (ptr, len) = if let Some(ref bound) = bound {
            (bound.as_ptr() as *const c_char, bound.len())
        } else if self.iterate_upper_bound.is_some() {
            (std::ptr::null(), 0)
        } else {
            return;
        };
        self.iterate_upper_bound = bound;
        unsafe {
            ffi::rocksdb_readoptions_set_iterate_upper_bound(self.inner, ptr, len);
        }
    }

    /// Specify if this read request should process data that ALREADY
    /// resides on a particular cache. If the required data is not
    /// found at the specified cache, then Status::Incomplete is returned.
    ///
    /// Default: ::All
    pub fn set_read_tier(&mut self, tier: ReadTier) {
        unsafe {
            ffi::rocksdb_readoptions_set_read_tier(self.inner, tier as c_int);
        }
    }

    /// Enforce that the iterator only iterates over the same
    /// prefix as the seek.
    /// This option is effective only for prefix seeks, i.e. prefix_extractor is
    /// non-null for the column family and total_order_seek is false.  Unlike
    /// iterate_upper_bound, prefix_same_as_start only works within a prefix
    /// but in both directions.
    ///
    /// Default: false
    pub fn set_prefix_same_as_start(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_readoptions_set_prefix_same_as_start(self.inner, c_uchar::from(v));
        }
    }

    /// Enable a total order seek regardless of index format (e.g. hash index)
    /// used in the table. Some table format (e.g. plain table) may not support
    /// this option.
    ///
    /// If true when calling Get(), we also skip prefix bloom when reading from
    /// block based table. It provides a way to read existing data after
    /// changing implementation of prefix extractor.
    pub fn set_total_order_seek(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_readoptions_set_total_order_seek(self.inner, c_uchar::from(v));
        }
    }

    /// Sets a threshold for the number of keys that can be skipped
    /// before failing an iterator seek as incomplete. The default value of 0 should be used to
    /// never fail a request as incomplete, even on skipping too many keys.
    ///
    /// Default: 0
    pub fn set_max_skippable_internal_keys(&mut self, num: u64) {
        unsafe {
            ffi::rocksdb_readoptions_set_max_skippable_internal_keys(self.inner, num);
        }
    }

    /// If true, when PurgeObsoleteFile is called in CleanupIteratorState, we schedule a background job
    /// in the flush job queue and delete obsolete files in background.
    ///
    /// Default: false
    pub fn set_background_purge_on_iterator_cleanup(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_readoptions_set_background_purge_on_iterator_cleanup(
                self.inner,
                c_uchar::from(v),
            );
        }
    }

    /// If true, keys deleted using the DeleteRange() API will be visible to
    /// readers until they are naturally deleted during compaction. This improves
    /// read performance in DBs with many range deletions.
    ///
    /// Default: false
    pub fn set_ignore_range_deletions(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_readoptions_set_ignore_range_deletions(self.inner, c_uchar::from(v));
        }
    }

    /// If true, all data read from underlying storage will be
    /// verified against corresponding checksums.
    ///
    /// Default: true
    pub fn set_verify_checksums(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_readoptions_set_verify_checksums(self.inner, c_uchar::from(v));
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
            ffi::rocksdb_readoptions_set_tailing(self.inner, c_uchar::from(v));
        }
    }

    /// Specifies the value of "pin_data". If true, it keeps the blocks
    /// loaded by the iterator pinned in memory as long as the iterator is not deleted,
    /// If used when reading from tables created with
    /// BlockBasedTableOptions::use_delta_encoding = false,
    /// Iterator's property "rocksdb.iterator.is-key-pinned" is guaranteed to
    /// return 1.
    ///
    /// Default: false
    pub fn set_pin_data(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_readoptions_set_pin_data(self.inner, c_uchar::from(v));
        }
    }

    /// Asynchronously prefetch some data.
    ///
    /// Used for sequential reads and internal automatic prefetching.
    ///
    /// Default: `false`
    pub fn set_async_io(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_readoptions_set_async_io(self.inner, c_uchar::from(v));
        }
    }
}

impl Default for ReadOptions {
    fn default() -> Self {
        unsafe {
            Self {
                inner: ffi::rocksdb_readoptions_create(),
                iterate_upper_bound: None,
                iterate_lower_bound: None,
            }
        }
    }
}

impl IngestExternalFileOptions {
    /// Can be set to true to move the files instead of copying them.
    pub fn set_move_files(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_ingestexternalfileoptions_set_move_files(self.inner, c_uchar::from(v));
        }
    }

    /// If set to false, an ingested file keys could appear in existing snapshots
    /// that where created before the file was ingested.
    pub fn set_snapshot_consistency(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_ingestexternalfileoptions_set_snapshot_consistency(
                self.inner,
                c_uchar::from(v),
            );
        }
    }

    /// If set to false, IngestExternalFile() will fail if the file key range
    /// overlaps with existing keys or tombstones in the DB.
    pub fn set_allow_global_seqno(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_ingestexternalfileoptions_set_allow_global_seqno(
                self.inner,
                c_uchar::from(v),
            );
        }
    }

    /// If set to false and the file key range overlaps with the memtable key range
    /// (memtable flush required), IngestExternalFile will fail.
    pub fn set_allow_blocking_flush(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_ingestexternalfileoptions_set_allow_blocking_flush(
                self.inner,
                c_uchar::from(v),
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
            ffi::rocksdb_ingestexternalfileoptions_set_ingest_behind(self.inner, c_uchar::from(v));
        }
    }
}

impl Default for IngestExternalFileOptions {
    fn default() -> Self {
        unsafe {
            Self {
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

/// Used by BlockBasedOptions::set_checksum_type.
pub enum ChecksumType {
    NoChecksum = 0,
    CRC32c = 1,
    XXHash = 2,
    XXHash64 = 3,
    XXH3 = 4, // Supported since RocksDB 6.27
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde1", derive(serde::Serialize, serde::Deserialize))]
pub enum DBCompressionType {
    None = ffi::rocksdb_no_compression as isize,
    Snappy = ffi::rocksdb_snappy_compression as isize,
    Zlib = ffi::rocksdb_zlib_compression as isize,
    Bz2 = ffi::rocksdb_bz2_compression as isize,
    Lz4 = ffi::rocksdb_lz4_compression as isize,
    Lz4hc = ffi::rocksdb_lz4hc_compression as isize,
    Zstd = ffi::rocksdb_zstd_compression as isize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde1", derive(serde::Serialize, serde::Deserialize))]
pub enum DBCompactionStyle {
    Level = ffi::rocksdb_level_compaction as isize,
    Universal = ffi::rocksdb_universal_compaction as isize,
    Fifo = ffi::rocksdb_fifo_compaction as isize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde1", derive(serde::Serialize, serde::Deserialize))]
pub enum DBRecoveryMode {
    TolerateCorruptedTailRecords = ffi::rocksdb_tolerate_corrupted_tail_records_recovery as isize,
    AbsoluteConsistency = ffi::rocksdb_absolute_consistency_recovery as isize,
    PointInTime = ffi::rocksdb_point_in_time_recovery as isize,
    SkipAnyCorruptedRecord = ffi::rocksdb_skip_any_corrupted_records_recovery as isize,
}

/// File access pattern once a compaction has started
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde1", derive(serde::Serialize, serde::Deserialize))]
#[repr(i32)]
pub enum AccessHint {
    None = 0,
    Normal,
    Sequential,
    WillNeed,
}

pub struct FifoCompactOptions {
    pub(crate) inner: *mut ffi::rocksdb_fifo_compaction_options_t,
}

impl Default for FifoCompactOptions {
    fn default() -> Self {
        let opts = unsafe { ffi::rocksdb_fifo_compaction_options_create() };
        assert!(
            !opts.is_null(),
            "Could not create RocksDB Fifo Compaction Options"
        );

        Self { inner: opts }
    }
}

impl Drop for FifoCompactOptions {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_fifo_compaction_options_destroy(self.inner);
        }
    }
}

impl FifoCompactOptions {
    /// Sets the max table file size.
    ///
    /// Once the total sum of table files reaches this, we will delete the oldest
    /// table file
    ///
    /// Default: 1GB
    pub fn set_max_table_files_size(&mut self, nbytes: u64) {
        unsafe {
            ffi::rocksdb_fifo_compaction_options_set_max_table_files_size(self.inner, nbytes);
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde1", derive(serde::Serialize, serde::Deserialize))]
pub enum UniversalCompactionStopStyle {
    Similar = ffi::rocksdb_similar_size_compaction_stop_style as isize,
    Total = ffi::rocksdb_total_size_compaction_stop_style as isize,
}

pub struct UniversalCompactOptions {
    pub(crate) inner: *mut ffi::rocksdb_universal_compaction_options_t,
}

impl Default for UniversalCompactOptions {
    fn default() -> Self {
        let opts = unsafe { ffi::rocksdb_universal_compaction_options_create() };
        assert!(
            !opts.is_null(),
            "Could not create RocksDB Universal Compaction Options"
        );

        Self { inner: opts }
    }
}

impl Drop for UniversalCompactOptions {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_universal_compaction_options_destroy(self.inner);
        }
    }
}

impl UniversalCompactOptions {
    /// Sets the percentage flexibility while comparing file size.
    /// If the candidate file(s) size is 1% smaller than the next file's size,
    /// then include next file into this candidate set.
    ///
    /// Default: 1
    pub fn set_size_ratio(&mut self, ratio: c_int) {
        unsafe {
            ffi::rocksdb_universal_compaction_options_set_size_ratio(self.inner, ratio);
        }
    }

    /// Sets the minimum number of files in a single compaction run.
    ///
    /// Default: 2
    pub fn set_min_merge_width(&mut self, num: c_int) {
        unsafe {
            ffi::rocksdb_universal_compaction_options_set_min_merge_width(self.inner, num);
        }
    }

    /// Sets the maximum number of files in a single compaction run.
    ///
    /// Default: UINT_MAX
    pub fn set_max_merge_width(&mut self, num: c_int) {
        unsafe {
            ffi::rocksdb_universal_compaction_options_set_max_merge_width(self.inner, num);
        }
    }

    /// sets the size amplification.
    ///
    /// It is defined as the amount (in percentage) of
    /// additional storage needed to store a single byte of data in the database.
    /// For example, a size amplification of 2% means that a database that
    /// contains 100 bytes of user-data may occupy upto 102 bytes of
    /// physical storage. By this definition, a fully compacted database has
    /// a size amplification of 0%. Rocksdb uses the following heuristic
    /// to calculate size amplification: it assumes that all files excluding
    /// the earliest file contribute to the size amplification.
    ///
    /// Default: 200, which means that a 100 byte database could require upto 300 bytes of storage.
    pub fn set_max_size_amplification_percent(&mut self, v: c_int) {
        unsafe {
            ffi::rocksdb_universal_compaction_options_set_max_size_amplification_percent(
                self.inner, v,
            );
        }
    }

    /// Sets the percentage of compression size.
    ///
    /// If this option is set to be -1, all the output files
    /// will follow compression type specified.
    ///
    /// If this option is not negative, we will try to make sure compressed
    /// size is just above this value. In normal cases, at least this percentage
    /// of data will be compressed.
    /// When we are compacting to a new file, here is the criteria whether
    /// it needs to be compressed: assuming here are the list of files sorted
    /// by generation time:
    ///    A1...An B1...Bm C1...Ct
    /// where A1 is the newest and Ct is the oldest, and we are going to compact
    /// B1...Bm, we calculate the total size of all the files as total_size, as
    /// well as  the total size of C1...Ct as total_C, the compaction output file
    /// will be compressed iff
    ///   total_C / total_size < this percentage
    ///
    /// Default: -1
    pub fn set_compression_size_percent(&mut self, v: c_int) {
        unsafe {
            ffi::rocksdb_universal_compaction_options_set_compression_size_percent(self.inner, v);
        }
    }

    /// Sets the algorithm used to stop picking files into a single compaction run.
    ///
    /// Default: ::Total
    pub fn set_stop_style(&mut self, style: UniversalCompactionStopStyle) {
        unsafe {
            ffi::rocksdb_universal_compaction_options_set_stop_style(self.inner, style as c_int);
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde1", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum BottommostLevelCompaction {
    /// Skip bottommost level compaction
    Skip = 0,
    /// Only compact bottommost level if there is a compaction filter
    /// This is the default option
    IfHaveCompactionFilter,
    /// Always compact bottommost level
    Force,
    /// Always compact bottommost level but in bottommost level avoid
    /// double-compacting files created in the same compaction
    ForceOptimized,
}

pub struct CompactOptions {
    pub(crate) inner: *mut ffi::rocksdb_compactoptions_t,
}

impl Default for CompactOptions {
    fn default() -> Self {
        let opts = unsafe { ffi::rocksdb_compactoptions_create() };
        assert!(!opts.is_null(), "Could not create RocksDB Compact Options");

        Self { inner: opts }
    }
}

impl Drop for CompactOptions {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_compactoptions_destroy(self.inner);
        }
    }
}

impl CompactOptions {
    /// If more than one thread calls manual compaction,
    /// only one will actually schedule it while the other threads will simply wait
    /// for the scheduled manual compaction to complete. If exclusive_manual_compaction
    /// is set to true, the call will disable scheduling of automatic compaction jobs
    /// and wait for existing automatic compaction jobs to finish.
    pub fn set_exclusive_manual_compaction(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_compactoptions_set_exclusive_manual_compaction(
                self.inner,
                c_uchar::from(v),
            );
        }
    }

    /// Sets bottommost level compaction.
    pub fn set_bottommost_level_compaction(&mut self, lvl: BottommostLevelCompaction) {
        unsafe {
            ffi::rocksdb_compactoptions_set_bottommost_level_compaction(self.inner, lvl as c_uchar);
        }
    }

    /// If true, compacted files will be moved to the minimum level capable
    /// of holding the data or given level (specified non-negative target_level).
    pub fn set_change_level(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_compactoptions_set_change_level(self.inner, c_uchar::from(v));
        }
    }

    /// If change_level is true and target_level have non-negative value, compacted
    /// files will be moved to target_level.
    pub fn set_target_level(&mut self, lvl: c_int) {
        unsafe {
            ffi::rocksdb_compactoptions_set_target_level(self.inner, lvl);
        }
    }
}

/// Represents a path where sst files can be put into
pub struct DBPath {
    pub(crate) inner: *mut ffi::rocksdb_dbpath_t,
}

impl DBPath {
    /// Create a new path
    pub fn new<P: AsRef<Path>>(path: P, target_size: u64) -> Result<Self, Error> {
        let p = to_cpath(path.as_ref()).unwrap();
        let dbpath = unsafe { ffi::rocksdb_dbpath_create(p.as_ptr(), target_size) };
        if dbpath.is_null() {
            Err(Error::new(format!(
                "Could not create path for storing sst files at location: {}",
                path.as_ref().display()
            )))
        } else {
            Ok(DBPath { inner: dbpath })
        }
    }
}

impl Drop for DBPath {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_dbpath_destroy(self.inner);
        }
    }
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

    #[test]
    fn test_set_stats_persist_period_sec() {
        let mut opts = Options::default();
        opts.enable_statistics();
        opts.set_stats_persist_period_sec(5);
        assert!(opts.get_statistics().is_some());

        let opts = Options::default();
        assert!(opts.get_statistics().is_none());
    }
}
