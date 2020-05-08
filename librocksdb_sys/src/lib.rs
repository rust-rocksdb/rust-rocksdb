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
#![feature(c_variadic)]

extern crate bzip2_sys;
extern crate libc;
#[cfg(test)]
extern crate tempfile;

use std::ffi::CStr;
use std::fmt;

use libc::{c_char, c_double, c_int, c_uchar, c_void, size_t};

// FFI-safe opaque types.
//
// These represent opaque RocksDB types. They are used behind pointers, but are
// also wrapped in other types in the higher-level bindings.
//
// These use the strategy for opaque C types described in the nomicon [1]:
// but with the exception that they contain c_void instead of [u8; 0], thus
// making them uninstantiable sized types instead of ZSTs.
//
// The c_void documentation publicly recommends using the ZST pattern from the
// nomicon, but in private documentation [2] warns about UB from dereferencing
// pointers to uninhabited types, which these bindings do.
//
// Additionally, these bindings wrap some these types directly (not through
// pointers) and it's impossible to repr(transparent) a ZST, without which the
// unsafe casts within are dubious.
//
// [1]: https://doc.rust-lang.org/nomicon/ffi.html#representing-opaque-structs
// [2]: https://doc.rust-lang.org/nightly/src/core/ffi.rs.html#28

#[repr(C)]
pub struct Options(c_void);
#[repr(C)]
pub struct ColumnFamilyDescriptor(c_void);
#[repr(C)]
pub struct DBInstance(c_void);
#[repr(C)]
pub struct DBWriteOptions(c_void);
#[repr(C)]
pub struct DBReadOptions(c_void);
#[repr(C)]
pub struct DBMergeOperator(c_void);
#[repr(C)]
pub struct DBBlockBasedTableOptions(c_void);
#[repr(C)]
pub struct DBMemoryAllocator(c_void);
#[repr(C)]
pub struct DBLRUCacheOptions(c_void);
#[repr(C)]
pub struct DBCache(c_void);
#[repr(C)]
pub struct DBFilterPolicy(c_void);
#[repr(C)]
pub struct DBSnapshot(c_void);
#[repr(C)]
pub struct DBIterator(c_void);
#[repr(C)]
pub struct DBCFHandle(c_void);
#[repr(C)]
pub struct DBWriteBatch(c_void);
#[repr(C)]
pub struct DBComparator(c_void);
#[repr(C)]
pub struct DBFlushOptions(c_void);
#[repr(C)]
pub struct DBCompactionFilter(c_void);
#[repr(C)]
pub struct DBCompactionFilterFactory(c_void);
#[repr(C)]
pub struct DBCompactionFilterContext(c_void);
#[repr(C)]
pub struct EnvOptions(c_void);
#[repr(C)]
pub struct SstFileReader(c_void);
#[repr(C)]
pub struct SstFileWriter(c_void);
#[repr(C)]
pub struct ExternalSstFileInfo(c_void);
#[repr(C)]
pub struct IngestExternalFileOptions(c_void);
#[repr(C)]
pub struct DBBackupEngine(c_void);
#[repr(C)]
pub struct DBRestoreOptions(c_void);
#[repr(C)]
pub struct DBSliceTransform(c_void);
#[repr(C)]
pub struct DBRateLimiter(c_void);
#[repr(C)]
pub struct DBLogger(c_void);
#[repr(C)]
pub struct DBCompactOptions(c_void);
#[repr(C)]
pub struct DBFifoCompactionOptions(c_void);
#[repr(C)]
pub struct DBPinnableSlice(c_void);
#[repr(C)]
pub struct DBUserCollectedProperties(c_void);
#[repr(C)]
pub struct DBUserCollectedPropertiesIterator(c_void);
#[repr(C)]
pub struct DBTableProperties(c_void);
#[repr(C)]
pub struct DBTablePropertiesCollection(c_void);
#[repr(C)]
pub struct DBTablePropertiesCollectionIterator(c_void);
#[repr(C)]
pub struct DBTablePropertiesCollector(c_void);
#[repr(C)]
pub struct DBTablePropertiesCollectorFactory(c_void);
#[repr(C)]
pub struct DBFlushJobInfo(c_void);
#[repr(C)]
pub struct DBCompactionJobInfo(c_void);
#[repr(C)]
pub struct DBIngestionInfo(c_void);
#[repr(C)]
pub struct DBEventListener(c_void);
#[repr(C)]
pub struct DBKeyVersions(c_void);
#[repr(C)]
pub struct DBEnv(c_void);
#[repr(C)]
pub struct DBSequentialFile(c_void);
#[repr(C)]
pub struct DBColumnFamilyMetaData(c_void);
#[repr(C)]
pub struct DBLevelMetaData(c_void);
#[repr(C)]
pub struct DBSstFileMetaData(c_void);
#[repr(C)]
pub struct DBCompactionOptions(c_void);
#[repr(C)]
pub struct DBPerfContext(c_void);
#[repr(C)]
pub struct DBIOStatsContext(c_void);
#[repr(C)]
pub struct DBWriteStallInfo(c_void);
#[repr(C)]
pub struct DBStatusPtr(c_void);
#[repr(C)]
pub struct DBMapProperty(c_void);
#[cfg(feature = "encryption")]
#[repr(C)]
pub struct DBFileEncryptionInfo(c_void);
#[cfg(feature = "encryption")]
#[repr(C)]
pub struct DBEncryptionKeyManagerInstance(c_void);

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub enum WriteStallCondition {
    Normal = 0,
    Delayed = 1,
    Stopped = 2,
}

mod generated;
pub use generated::*;

#[repr(C)]
pub struct DBTitanDBOptions(c_void);
#[repr(C)]
pub struct DBTitanReadOptions(c_void);

#[derive(Clone, Debug, Default)]
#[repr(C)]
pub struct DBTitanBlobIndex {
    pub file_number: u64,
    pub blob_offset: u64,
    pub blob_size: u64,
}

pub fn new_bloom_filter(bits: c_int) -> *mut DBFilterPolicy {
    unsafe { crocksdb_filterpolicy_create_bloom(bits) }
}

/// # Safety
///
/// `DBLRUCacheOptions` should pointer to a valid cache options
pub unsafe fn new_lru_cache(opt: *mut DBLRUCacheOptions) -> *mut DBCache {
    crocksdb_cache_create_lru(opt)
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub enum DBEntryType {
    Put = 0,
    Delete = 1,
    SingleDelete = 2,
    Merge = 3,
    RangeDeletion = 4,
    BlobIndex = 5,
    Other = 6,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub enum DBCompressionType {
    No = 0,
    Snappy = 1,
    Zlib = 2,
    Bz2 = 3,
    Lz4 = 4,
    Lz4hc = 5,
    // DBXpress = 6, not support currently.
    Zstd = 7,
    ZstdNotFinal = 0x40,
    Disable = 0xff,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub enum DBCompactionStyle {
    Level = 0,
    Universal = 1,
    Fifo = 2,
    None = 3,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub enum DBUniversalCompactionStyle {
    SimilarSize = 0,
    TotalSize = 1,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub enum DBRecoveryMode {
    TolerateCorruptedTailRecords = 0,
    AbsoluteConsistency = 1,
    PointInTime = 2,
    SkipAnyCorruptedRecords = 3,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub enum CompactionPriority {
    // In Level-based compaction, it Determines which file from a level to be
    // picked to merge to the next level. We suggest people try
    // kMinOverlappingRatio first when you tune your database.
    ByCompensatedSize = 0,
    // First compact files whose data's latest update time is oldest.
    // Try this if you only update some hot keys in small ranges.
    OldestLargestSeqFirst = 1,
    // First compact files whose range hasn't been compacted to the next level
    // for the longest. If your updates are random across the key space,
    // write amplification is slightly better with this option.
    OldestSmallestSeqFirst = 2,
    // First compact files whose ratio between overlapping size in next level
    // and its size is the smallest. It in many cases can optimize write
    // amplification.
    MinOverlappingRatio = 3,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub enum CompactionReason {
    Unknown,
    // [Level] number of L0 files > level0_file_num_compaction_trigger
    LevelL0FilesNum,
    // [Level] total size of level > MaxBytesForLevel()
    LevelMaxLevelSize,
    // [Universal] Compacting for size amplification
    UniversalSizeAmplification,
    // [Universal] Compacting for size ratio
    UniversalSizeRatio,
    // [Universal] number of sorted runs > level0_file_num_compaction_trigger
    UniversalSortedRunNum,
    // [FIFO] total size > max_table_files_size
    FIFOMaxSize,
    // [FIFO] reduce number of files.
    FIFOReduceNumFiles,
    // [FIFO] files with creation time < (current_time - interval)
    FIFOTtl,
    // Manual compaction
    ManualCompaction,
    // DB::SuggestCompactRange() marked files for compaction
    FilesMarkedForCompaction,
    // [Level] Automatic compaction within bottommost level to cleanup duplicate
    // versions of same user key, usually due to a released snapshot.
    BottommostFiles,
    // Compaction based on TTL
    Ttl,
    // According to the comments in flush_job.cc, RocksDB treats flush as
    // a level 0 compaction in internal stats.
    Flush,
    // Compaction caused by external sst file ingestion
    ExternalSstIngestion,
    // total number of compaction reasons, new reasons must be added above this.
    NumOfReasons,
}

impl fmt::Display for CompactionReason {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub enum DBInfoLogLevel {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
    Fatal = 4,
    Header = 5,
    NumInfoLog = 6,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub enum DBTableProperty {
    DataSize = 1,
    IndexSize = 2,
    FilterSize = 3,
    RawKeySize = 4,
    RawValueSize = 5,
    NumDataBlocks = 6,
    NumEntries = 7,
    FormatVersion = 8,
    FixedKeyLen = 9,
    ColumnFamilyId = 10,
    ColumnFamilyName = 11,
    FilterPolicyName = 12,
    ComparatorName = 13,
    MergeOperatorName = 14,
    PrefixExtractorName = 15,
    PropertyCollectorsNames = 16,
    CompressionName = 17,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub enum DBBottommostLevelCompaction {
    // Skip bottommost level compaction
    Skip = 0,
    // Compact bottommost level if there is a compaction filter
    // This is the default option
    IfHaveCompactionFilter = 1,
    // Force bottommost level compaction
    Force = 2,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub enum DBRateLimiterMode {
    ReadOnly = 1,
    WriteOnly = 2,
    AllIo = 3,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub enum DBTitanDBBlobRunMode {
    Normal = 0,
    ReadOnly = 1,
    Fallback = 2,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub enum IndexType {
    BinarySearch = 0,
    HashSearch = 1,
    TwoLevelIndexSearch = 2,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub enum DBBackgroundErrorReason {
    Flush = 1,
    Compaction = 2,
    WriteCallback = 3,
    MemTable = 4,
}

#[cfg(feature = "encryption")]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub enum DBEncryptionMethod {
    Unknown = 0,
    Plaintext = 1,
    Aes128Ctr = 2,
    Aes192Ctr = 3,
    Aes256Ctr = 4,
}

#[cfg(feature = "encryption")]
impl fmt::Display for DBEncryptionMethod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// # Safety
///
/// ptr must point to a valid CStr value
pub unsafe fn error_message(ptr: *mut c_char) -> String {
    let c_str = CStr::from_ptr(ptr);
    let s = format!("{}", c_str.to_string_lossy());
    libc::free(ptr as *mut c_void);
    s
}

#[macro_export]
macro_rules! ffi_try {
    ($func:ident($($arg:expr),+)) => ({
        use std::ptr;
        let mut err = ptr::null_mut();
        let res = $crate::$func($($arg),+, &mut err);
        if !err.is_null() {
            return Err($crate::error_message(err));
        }
        res
    });
    ($func:ident()) => ({
        use std::ptr;
        let mut err = ptr::null_mut();
        let res = $crate::$func(&mut err);
        if !err.is_null() {
            return Err($crate::error_message(err));
        }
        res
    })
}

// TODO audit the use of boolean arguments, b/c I think they need to be u8
// instead...
extern "C" {
    pub fn crocksdb_status_ptr_get_error(status: *mut DBStatusPtr, err: *mut *mut c_char);
    pub fn crocksdb_get_db_options(db: *mut DBInstance) -> *mut Options;
    pub fn crocksdb_set_db_options(
        db: *mut DBInstance,
        names: *const *const c_char,
        values: *const *const c_char,
        num_options: size_t,
        errptr: *mut *mut c_char,
    );
    pub fn crocksdb_get_options_cf(db: *mut DBInstance, cf: *mut DBCFHandle) -> *mut Options;
    pub fn crocksdb_set_options_cf(
        db: *mut DBInstance,
        cf: *mut DBCFHandle,
        names: *const *const c_char,
        values: *const *const c_char,
        num_options: size_t,
        errptr: *mut *mut c_char,
    );
    pub fn crocksdb_options_create() -> *mut Options;
    pub fn crocksdb_options_copy(opts: *const Options) -> *mut Options;
    pub fn crocksdb_options_destroy(opts: *mut Options);
    pub fn crocksdb_options_set_paranoid_checks(opts: *mut Options, _: u8);
    pub fn crocksdb_column_family_descriptor_destroy(cf_desc: *mut ColumnFamilyDescriptor);
    pub fn crocksdb_name_from_column_family_descriptor(
        cf_descs: *const ColumnFamilyDescriptor,
    ) -> *const c_char;
    pub fn crocksdb_options_from_column_family_descriptor(
        cf_descs: *const ColumnFamilyDescriptor,
    ) -> *mut Options;

    // Memory Allocator
    pub fn crocksdb_jemalloc_nodump_allocator_create(
        err: *mut *mut c_char,
    ) -> *mut DBMemoryAllocator;
    pub fn crocksdb_memory_allocator_destroy(allocator: *mut DBMemoryAllocator);

    // Cache
    pub fn crocksdb_lru_cache_options_create() -> *mut DBLRUCacheOptions;
    pub fn crocksdb_lru_cache_options_destroy(opt: *mut DBLRUCacheOptions);
    pub fn crocksdb_lru_cache_options_set_capacity(opt: *mut DBLRUCacheOptions, capacity: size_t);
    pub fn crocksdb_lru_cache_options_set_num_shard_bits(
        opt: *mut DBLRUCacheOptions,
        num_shard_bits: c_int,
    );
    pub fn crocksdb_lru_cache_options_set_strict_capacity_limit(
        opt: *mut DBLRUCacheOptions,
        strict_capacity_limit: bool,
    );
    pub fn crocksdb_lru_cache_options_set_high_pri_pool_ratio(
        opt: *mut DBLRUCacheOptions,
        high_pri_pool_ratio: c_double,
    );
    pub fn crocksdb_lru_cache_options_set_memory_allocator(
        opt: *mut DBLRUCacheOptions,
        allocator: *mut DBMemoryAllocator,
    );
    pub fn crocksdb_cache_create_lru(opt: *mut DBLRUCacheOptions) -> *mut DBCache;
    pub fn crocksdb_cache_destroy(cache: *mut DBCache);

    pub fn crocksdb_block_based_options_create() -> *mut DBBlockBasedTableOptions;
    pub fn crocksdb_block_based_options_destroy(opts: *mut DBBlockBasedTableOptions);
    pub fn crocksdb_block_based_options_set_metadata_block_size(
        block_options: *mut DBBlockBasedTableOptions,
        block_size: size_t,
    );
    pub fn crocksdb_block_based_options_set_block_size(
        block_options: *mut DBBlockBasedTableOptions,
        block_size: size_t,
    );
    pub fn crocksdb_block_based_options_set_block_size_deviation(
        block_options: *mut DBBlockBasedTableOptions,
        block_size_deviation: c_int,
    );
    pub fn crocksdb_block_based_options_set_block_restart_interval(
        block_options: *mut DBBlockBasedTableOptions,
        block_restart_interval: c_int,
    );
    pub fn crocksdb_block_based_options_set_index_type(
        block_options: *mut DBBlockBasedTableOptions,
        v: IndexType,
    );
    pub fn crocksdb_block_based_options_set_hash_index_allow_collision(
        block_options: *mut DBBlockBasedTableOptions,
        v: c_uchar,
    );
    pub fn crocksdb_block_based_options_set_partition_filters(
        block_options: *mut DBBlockBasedTableOptions,
        v: c_uchar,
    );
    pub fn crocksdb_block_based_options_set_cache_index_and_filter_blocks(
        block_options: *mut DBBlockBasedTableOptions,
        v: c_uchar,
    );
    pub fn crocksdb_block_based_options_set_pin_top_level_index_and_filter(
        block_options: *mut DBBlockBasedTableOptions,
        v: c_uchar,
    );
    pub fn crocksdb_block_based_options_set_cache_index_and_filter_blocks_with_high_priority(
        block_options: *mut DBBlockBasedTableOptions,
        v: c_uchar,
    );
    pub fn crocksdb_block_based_options_set_filter_policy(
        block_options: *mut DBBlockBasedTableOptions,
        filter_policy: *mut DBFilterPolicy,
    );
    pub fn crocksdb_block_based_options_set_no_block_cache(
        block_options: *mut DBBlockBasedTableOptions,
        no_block_cache: bool,
    );
    pub fn crocksdb_block_based_options_set_block_cache(
        block_options: *mut DBBlockBasedTableOptions,
        block_cache: *mut DBCache,
    );
    pub fn crocksdb_block_based_options_set_block_cache_compressed(
        block_options: *mut DBBlockBasedTableOptions,
        block_cache_compressed: *mut DBCache,
    );
    pub fn crocksdb_block_based_options_set_whole_key_filtering(
        ck_options: *mut DBBlockBasedTableOptions,
        doit: bool,
    );
    pub fn crocksdb_options_set_block_based_table_factory(
        options: *mut Options,
        block_options: *mut DBBlockBasedTableOptions,
    );
    pub fn crocksdb_block_based_options_set_pin_l0_filter_and_index_blocks_in_cache(
        block_options: *mut DBBlockBasedTableOptions,
        v: c_uchar,
    );
    pub fn crocksdb_block_based_options_set_read_amp_bytes_per_bit(
        block_options: *mut DBBlockBasedTableOptions,
        v: c_int,
    );
    pub fn crocksdb_options_increase_parallelism(options: *mut Options, threads: c_int);
    pub fn crocksdb_options_optimize_level_style_compaction(
        options: *mut Options,
        memtable_memory_budget: c_int,
    );
    pub fn crocksdb_options_set_env(options: *mut Options, env: *mut DBEnv);
    pub fn crocksdb_options_set_compaction_filter(
        options: *mut Options,
        filter: *mut DBCompactionFilter,
    );
    pub fn crocksdb_options_set_compaction_filter_factory(
        options: *mut Options,
        filter: *mut DBCompactionFilterFactory,
    );
    pub fn crocksdb_options_set_create_if_missing(options: *mut Options, v: bool);
    pub fn crocksdb_options_set_max_open_files(options: *mut Options, files: c_int);
    pub fn crocksdb_options_set_max_total_wal_size(options: *mut Options, size: u64);
    pub fn crocksdb_options_set_use_fsync(options: *mut Options, v: c_int);
    pub fn crocksdb_options_set_bytes_per_sync(options: *mut Options, bytes: u64);
    pub fn crocksdb_options_set_enable_pipelined_write(options: *mut Options, v: bool);
    pub fn crocksdb_options_set_enable_multi_batch_write(options: *mut Options, v: bool);
    pub fn crocksdb_options_is_enable_multi_batch_write(options: *mut Options) -> bool;
    pub fn crocksdb_options_set_unordered_write(options: *mut Options, v: bool);
    pub fn crocksdb_options_set_allow_concurrent_memtable_write(options: *mut Options, v: bool);
    pub fn crocksdb_options_set_manual_wal_flush(options: *mut Options, v: bool);
    pub fn crocksdb_options_optimize_for_point_lookup(
        options: *mut Options,
        block_cache_size_mb: u64,
    );
    pub fn crocksdb_options_set_table_cache_numshardbits(options: *mut Options, bits: c_int);
    pub fn crocksdb_options_set_writable_file_max_buffer_size(options: *mut Options, nbytes: c_int);
    pub fn crocksdb_options_set_max_write_buffer_number(options: *mut Options, bufno: c_int);
    pub fn crocksdb_options_set_min_write_buffer_number_to_merge(
        options: *mut Options,
        bufno: c_int,
    );
    pub fn crocksdb_options_set_level0_file_num_compaction_trigger(
        options: *mut Options,
        no: c_int,
    );
    pub fn crocksdb_options_set_level0_slowdown_writes_trigger(options: *mut Options, no: c_int);
    pub fn crocksdb_options_get_level0_slowdown_writes_trigger(options: *mut Options) -> c_int;
    pub fn crocksdb_options_set_level0_stop_writes_trigger(options: *mut Options, no: c_int);
    pub fn crocksdb_options_get_level0_stop_writes_trigger(options: *mut Options) -> c_int;
    pub fn crocksdb_options_set_write_buffer_size(options: *mut Options, bytes: u64);
    pub fn crocksdb_options_set_target_file_size_base(options: *mut Options, bytes: u64);
    pub fn crocksdb_options_get_target_file_size_base(options: *const Options) -> u64;
    pub fn crocksdb_options_set_target_file_size_multiplier(options: *mut Options, mul: c_int);
    pub fn crocksdb_options_set_max_bytes_for_level_base(options: *mut Options, bytes: u64);
    pub fn crocksdb_options_set_max_bytes_for_level_multiplier(options: *mut Options, mul: f64);
    pub fn crocksdb_options_get_max_bytes_for_level_multiplier(options: *mut Options) -> f64;
    pub fn crocksdb_options_set_max_compaction_bytes(options: *mut Options, bytes: u64);
    pub fn crocksdb_options_set_max_log_file_size(options: *mut Options, bytes: size_t);
    pub fn crocksdb_options_set_log_file_time_to_roll(options: *mut Options, bytes: size_t);
    pub fn crocksdb_options_set_info_log_level(options: *mut Options, level: DBInfoLogLevel);
    pub fn crocksdb_options_set_keep_log_file_num(options: *mut Options, num: size_t);
    pub fn crocksdb_options_set_recycle_log_file_num(options: *mut Options, num: size_t);
    pub fn crocksdb_options_set_max_manifest_file_size(options: *mut Options, bytes: u64);
    pub fn crocksdb_options_get_memtable_factory_name(options: *mut Options) -> *const c_char;
    pub fn crocksdb_options_set_hash_skip_list_rep(
        options: *mut Options,
        bytes: u64,
        a1: i32,
        a2: i32,
    );
    pub fn crocksdb_options_set_doubly_skip_list_rep(options: *mut Options);
    pub fn crocksdb_options_set_compaction_style(options: *mut Options, cs: DBCompactionStyle);
    pub fn crocksdb_options_set_fifo_compaction_options(
        options: *mut Options,
        fifo_opts: *mut DBFifoCompactionOptions,
    );
    pub fn crocksdb_options_set_compression(
        options: *mut Options,
        compression_style_no: DBCompressionType,
    );
    pub fn crocksdb_options_get_compression(options: *mut Options) -> DBCompressionType;
    pub fn crocksdb_options_set_compression_per_level(
        options: *mut Options,
        level_values: *const DBCompressionType,
        num_levels: size_t,
    );
    pub fn crocksdb_options_get_compression_level_number(options: *mut Options) -> size_t;
    pub fn crocksdb_options_get_compression_per_level(
        options: *mut Options,
        level_values: *mut DBCompressionType,
    );
    pub fn crocksdb_set_bottommost_compression(options: *mut Options, c: DBCompressionType);
    pub fn crocksdb_options_set_max_background_jobs(options: *mut Options, max_bg_jobs: c_int);
    pub fn crocksdb_options_get_max_background_jobs(options: *const Options) -> c_int;
    pub fn crocksdb_options_set_disable_auto_compactions(options: *mut Options, v: c_int);
    pub fn crocksdb_options_get_disable_auto_compactions(options: *const Options) -> c_int;
    pub fn crocksdb_options_set_report_bg_io_stats(options: *mut Options, v: c_int);
    pub fn crocksdb_options_set_compaction_readahead_size(options: *mut Options, v: size_t);
    pub fn crocksdb_options_set_wal_recovery_mode(options: *mut Options, mode: DBRecoveryMode);
    pub fn crocksdb_options_set_max_subcompactions(options: *mut Options, v: u32);
    pub fn crocksdb_options_set_wal_bytes_per_sync(options: *mut Options, v: u64);
    pub fn crocksdb_options_enable_statistics(options: *mut Options, v: bool);
    pub fn crocksdb_options_reset_statistics(options: *mut Options);
    pub fn crocksdb_options_statistics_get_string(options: *mut Options) -> *const c_char;
    pub fn crocksdb_options_statistics_get_ticker_count(
        options: *mut Options,
        ticker_type: DBStatisticsTickerType,
    ) -> u64;
    pub fn crocksdb_options_statistics_get_and_reset_ticker_count(
        options: *mut Options,
        ticker_type: DBStatisticsTickerType,
    ) -> u64;
    pub fn crocksdb_options_statistics_get_histogram_string(
        options: *mut Options,
        hist_type: DBStatisticsHistogramType,
    ) -> *const c_char;
    pub fn crocksdb_options_statistics_get_histogram(
        options: *mut Options,
        hist_type: DBStatisticsHistogramType,
        median: *mut c_double,
        percentile95: *mut c_double,
        percentile99: *mut c_double,
        average: *mut c_double,
        standard_deviation: *mut c_double,
        max: *mut c_double,
    ) -> bool;
    pub fn crocksdb_options_set_stats_dump_period_sec(options: *mut Options, v: usize);
    pub fn crocksdb_options_set_num_levels(options: *mut Options, v: c_int);
    pub fn crocksdb_options_get_num_levels(options: *mut Options) -> c_int;
    pub fn crocksdb_options_set_db_log_dir(options: *mut Options, path: *const c_char);
    pub fn crocksdb_options_set_wal_dir(options: *mut Options, path: *const c_char);
    pub fn crocksdb_options_set_wal_ttl_seconds(options: *mut Options, ttl: u64);
    pub fn crocksdb_options_set_wal_size_limit_mb(options: *mut Options, limit: u64);
    pub fn crocksdb_options_set_use_direct_reads(options: *mut Options, v: bool);
    pub fn crocksdb_options_set_use_direct_io_for_flush_and_compaction(
        options: *mut Options,
        v: bool,
    );
    pub fn crocksdb_options_set_prefix_extractor(
        options: *mut Options,
        prefix_extractor: *mut DBSliceTransform,
    );
    pub fn crocksdb_options_set_optimize_filters_for_hits(options: *mut Options, v: bool);
    pub fn crocksdb_options_set_level_compaction_dynamic_level_bytes(
        options: *mut Options,
        v: bool,
    );
    pub fn crocksdb_options_get_level_compaction_dynamic_level_bytes(
        options: *const Options,
    ) -> bool;
    pub fn crocksdb_options_set_memtable_insert_with_hint_prefix_extractor(
        options: *mut Options,
        prefix_extractor: *mut DBSliceTransform,
    );
    pub fn crocksdb_options_set_memtable_prefix_bloom_size_ratio(
        options: *mut Options,
        ratio: c_double,
    );
    pub fn crocksdb_options_set_delayed_write_rate(options: *mut Options, rate: u64);
    pub fn crocksdb_options_set_force_consistency_checks(options: *mut Options, v: bool);
    pub fn crocksdb_options_set_ratelimiter(options: *mut Options, limiter: *mut DBRateLimiter);
    pub fn crocksdb_options_set_info_log(options: *mut Options, logger: *mut DBLogger);
    pub fn crocksdb_options_get_block_cache_usage(options: *const Options) -> usize;
    pub fn crocksdb_options_set_block_cache_capacity(
        options: *const Options,
        capacity: usize,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_options_get_block_cache_capacity(options: *const Options) -> usize;
    pub fn crocksdb_load_latest_options(
        dbpath: *const c_char,
        env: *mut DBEnv,
        db_options: *const Options,
        cf_descs: *const *mut *mut ColumnFamilyDescriptor,
        cf_descs_len: *mut size_t,
        ignore_unknown_options: bool,
        errptr: *mut *mut c_char,
    ) -> bool;
    pub fn crocksdb_ratelimiter_create(
        rate_bytes_per_sec: i64,
        refill_period_us: i64,
        fairness: i32,
    ) -> *mut DBRateLimiter;
    pub fn crocksdb_ratelimiter_create_with_auto_tuned(
        rate_bytes_per_sec: i64,
        refill_period_us: i64,
        fairness: i32,
        mode: DBRateLimiterMode,
        auto_tuned: bool,
    ) -> *mut DBRateLimiter;
    pub fn crocksdb_ratelimiter_destroy(limiter: *mut DBRateLimiter);
    pub fn crocksdb_ratelimiter_set_bytes_per_second(
        limiter: *mut DBRateLimiter,
        bytes_per_sec: i64,
    );
    pub fn crocksdb_ratelimiter_get_singleburst_bytes(limiter: *mut DBRateLimiter) -> i64;
    pub fn crocksdb_ratelimiter_request(limiter: *mut DBRateLimiter, bytes: i64, pri: c_uchar);
    pub fn crocksdb_ratelimiter_get_total_bytes_through(
        limiter: *mut DBRateLimiter,
        pri: c_uchar,
    ) -> i64;
    pub fn crocksdb_ratelimiter_get_bytes_per_second(limiter: *mut DBRateLimiter) -> i64;
    pub fn crocksdb_ratelimiter_get_total_requests(
        limiter: *mut DBRateLimiter,
        pri: c_uchar,
    ) -> i64;
    pub fn crocksdb_options_set_soft_pending_compaction_bytes_limit(options: *mut Options, v: u64);
    pub fn crocksdb_options_get_soft_pending_compaction_bytes_limit(options: *mut Options) -> u64;
    pub fn crocksdb_options_set_hard_pending_compaction_bytes_limit(options: *mut Options, v: u64);
    pub fn crocksdb_options_get_hard_pending_compaction_bytes_limit(options: *mut Options) -> u64;
    pub fn crocksdb_options_set_compaction_priority(options: *mut Options, v: CompactionPriority);
    pub fn crocksdb_options_set_db_paths(
        options: *mut Options,
        db_paths: *const *const c_char,
        path_lens: *const usize,
        target_size: *const u64,
        num_paths: c_int,
    );
    pub fn crocksdb_options_get_db_paths_num(options: *mut Options) -> usize;
    pub fn crocksdb_options_get_db_path(options: *mut Options, idx: size_t) -> *const c_char;
    pub fn crocksdb_options_get_path_target_size(options: *mut Options, idx: size_t) -> u64;
    pub fn crocksdb_options_set_vector_memtable_factory(options: *mut Options, reserved_bytes: u64);
    pub fn crocksdb_options_set_atomic_flush(option: *mut Options, enable: bool);
    pub fn crocksdb_filterpolicy_create_bloom_full(bits_per_key: c_int) -> *mut DBFilterPolicy;
    pub fn crocksdb_filterpolicy_create_bloom(bits_per_key: c_int) -> *mut DBFilterPolicy;
    pub fn crocksdb_open(
        options: *mut Options,
        path: *const c_char,
        err: *mut *mut c_char,
    ) -> *mut DBInstance;
    pub fn crocksdb_open_with_ttl(
        options: *mut Options,
        path: *const c_char,
        ttl: c_int,
        err: *mut *mut c_char,
    ) -> *mut DBInstance;
    pub fn crocksdb_open_for_read_only(
        options: *mut Options,
        path: *const c_char,
        error_if_log_file_exist: bool,
        err: *mut *mut c_char,
    ) -> *mut DBInstance;
    pub fn crocksdb_writeoptions_create() -> *mut DBWriteOptions;
    pub fn crocksdb_writeoptions_destroy(writeopts: *mut DBWriteOptions);
    pub fn crocksdb_writeoptions_set_sync(writeopts: *mut DBWriteOptions, v: bool);
    pub fn crocksdb_writeoptions_disable_wal(writeopts: *mut DBWriteOptions, v: c_int);
    pub fn crocksdb_writeoptions_set_ignore_missing_column_families(
        writeopts: *mut DBWriteOptions,
        v: bool,
    );
    pub fn crocksdb_writeoptions_set_no_slowdown(writeopts: *mut DBWriteOptions, v: bool);
    pub fn crocksdb_writeoptions_set_low_pri(writeopts: *mut DBWriteOptions, v: bool);
    pub fn crocksdb_put(
        db: *mut DBInstance,
        writeopts: *mut DBWriteOptions,
        k: *const u8,
        kLen: size_t,
        v: *const u8,
        vLen: size_t,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_put_cf(
        db: *mut DBInstance,
        writeopts: *mut DBWriteOptions,
        cf: *mut DBCFHandle,
        k: *const u8,
        kLen: size_t,
        v: *const u8,
        vLen: size_t,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_readoptions_create() -> *mut DBReadOptions;
    pub fn crocksdb_readoptions_destroy(readopts: *mut DBReadOptions);
    pub fn crocksdb_readoptions_set_verify_checksums(readopts: *mut DBReadOptions, v: bool);
    pub fn crocksdb_readoptions_set_fill_cache(readopts: *mut DBReadOptions, v: bool);
    pub fn crocksdb_readoptions_set_snapshot(
        readopts: *mut DBReadOptions,
        snapshot: *const DBSnapshot,
    );
    pub fn crocksdb_readoptions_set_iterate_lower_bound(
        readopts: *mut DBReadOptions,
        k: *const u8,
        kLen: size_t,
    );
    pub fn crocksdb_readoptions_set_iterate_upper_bound(
        readopts: *mut DBReadOptions,
        k: *const u8,
        kLen: size_t,
    );
    pub fn crocksdb_readoptions_set_read_tier(readopts: *mut DBReadOptions, tier: c_int);
    pub fn crocksdb_readoptions_set_tailing(readopts: *mut DBReadOptions, v: bool);
    pub fn crocksdb_readoptions_set_managed(readopts: *mut DBReadOptions, v: bool);
    pub fn crocksdb_readoptions_set_readahead_size(readopts: *mut DBReadOptions, size: size_t);
    pub fn crocksdb_readoptions_set_max_skippable_internal_keys(
        readopts: *mut DBReadOptions,
        n: u64,
    );
    pub fn crocksdb_readoptions_set_total_order_seek(readopts: *mut DBReadOptions, v: bool);
    pub fn crocksdb_readoptions_set_prefix_same_as_start(readopts: *mut DBReadOptions, v: bool);
    pub fn crocksdb_readoptions_set_pin_data(readopts: *mut DBReadOptions, v: bool);
    pub fn crocksdb_readoptions_set_background_purge_on_iterator_cleanup(
        readopts: *mut DBReadOptions,
        v: bool,
    );
    pub fn crocksdb_readoptions_set_ignore_range_deletions(readopts: *mut DBReadOptions, v: bool);
    pub fn crocksdb_readoptions_set_table_filter(
        readopts: *mut DBReadOptions,
        ctx: *mut c_void,
        filter: extern "C" fn(*mut c_void, *const DBTableProperties) -> c_int,
        destroy: extern "C" fn(*mut c_void),
    );

    pub fn crocksdb_get(
        db: *const DBInstance,
        readopts: *const DBReadOptions,
        k: *const u8,
        kLen: size_t,
        valLen: *const size_t,
        err: *mut *mut c_char,
    ) -> *mut u8;
    pub fn crocksdb_get_cf(
        db: *const DBInstance,
        readopts: *const DBReadOptions,
        cf_handle: *mut DBCFHandle,
        k: *const u8,
        kLen: size_t,
        valLen: *const size_t,
        err: *mut *mut c_char,
    ) -> *mut u8;
    pub fn crocksdb_create_iterator(
        db: *mut DBInstance,
        readopts: *const DBReadOptions,
    ) -> *mut DBIterator;
    pub fn crocksdb_create_iterator_cf(
        db: *mut DBInstance,
        readopts: *const DBReadOptions,
        cf_handle: *mut DBCFHandle,
    ) -> *mut DBIterator;
    pub fn crocksdb_create_snapshot(db: *mut DBInstance) -> *const DBSnapshot;
    pub fn crocksdb_release_snapshot(db: *mut DBInstance, snapshot: *const DBSnapshot);
    pub fn crocksdb_get_snapshot_sequence_number(snapshot: *const DBSnapshot) -> u64;

    pub fn crocksdb_delete(
        db: *mut DBInstance,
        writeopts: *const DBWriteOptions,
        k: *const u8,
        kLen: size_t,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_delete_cf(
        db: *mut DBInstance,
        writeopts: *const DBWriteOptions,
        cf: *mut DBCFHandle,
        k: *const u8,
        kLen: size_t,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_single_delete(
        db: *mut DBInstance,
        writeopts: *const DBWriteOptions,
        k: *const u8,
        kLen: size_t,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_single_delete_cf(
        db: *mut DBInstance,
        writeopts: *const DBWriteOptions,
        cf: *mut DBCFHandle,
        k: *const u8,
        kLen: size_t,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_delete_range_cf(
        db: *mut DBInstance,
        writeopts: *const DBWriteOptions,
        cf: *mut DBCFHandle,
        begin_key: *const u8,
        begin_keylen: size_t,
        end_key: *const u8,
        end_keylen: size_t,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_close(db: *mut DBInstance);
    pub fn crocksdb_pause_bg_work(db: *mut DBInstance);
    pub fn crocksdb_continue_bg_work(db: *mut DBInstance);
    pub fn crocksdb_destroy_db(options: *const Options, path: *const c_char, err: *mut *mut c_char);
    pub fn crocksdb_repair_db(options: *const Options, path: *const c_char, err: *mut *mut c_char);
    // Merge
    pub fn crocksdb_merge(
        db: *mut DBInstance,
        writeopts: *const DBWriteOptions,
        k: *const u8,
        kLen: size_t,
        v: *const u8,
        vLen: size_t,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_merge_cf(
        db: *mut DBInstance,
        writeopts: *const DBWriteOptions,
        cf: *mut DBCFHandle,
        k: *const u8,
        kLen: size_t,
        v: *const u8,
        vLen: size_t,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_mergeoperator_create(
        state: *mut c_void,
        destroy: unsafe extern "C" fn(*mut c_void) -> (),
        full_merge: unsafe extern "C" fn(
            arg: *mut c_void,
            key: *const c_char,
            key_len: size_t,
            existing_value: *const c_char,
            existing_value_len: size_t,
            operands_list: *const *const c_char,
            operands_list_len: *const size_t,
            num_operands: c_int,
            success: *mut u8,
            new_value_length: *mut size_t,
        ) -> *const c_char,
        partial_merge: unsafe extern "C" fn(
            arg: *mut c_void,
            key: *const c_char,
            key_len: size_t,
            operands_list: *const *const c_char,
            operands_list_len: *const size_t,
            num_operands: c_int,
            success: *mut u8,
            new_value_length: *mut size_t,
        ) -> *const c_char,
        delete_value: Option<
            unsafe extern "C" fn(*mut c_void, value: *const c_char, value_len: *mut size_t) -> (),
        >,
        name_fn: unsafe extern "C" fn(*mut c_void) -> *const c_char,
    ) -> *mut DBMergeOperator;
    pub fn crocksdb_mergeoperator_destroy(mo: *mut DBMergeOperator);
    pub fn crocksdb_options_set_merge_operator(options: *mut Options, mo: *mut DBMergeOperator);
    // Iterator
    pub fn crocksdb_iter_destroy(iter: *mut DBIterator);
    pub fn crocksdb_iter_valid(iter: *const DBIterator) -> bool;
    pub fn crocksdb_iter_seek_to_first(iter: *mut DBIterator);
    pub fn crocksdb_iter_seek_to_last(iter: *mut DBIterator);
    pub fn crocksdb_iter_seek(iter: *mut DBIterator, key: *const u8, klen: size_t);
    pub fn crocksdb_iter_seek_for_prev(iter: *mut DBIterator, key: *const u8, klen: size_t);
    pub fn crocksdb_iter_next(iter: *mut DBIterator);
    pub fn crocksdb_iter_prev(iter: *mut DBIterator);
    pub fn crocksdb_iter_key(iter: *const DBIterator, klen: *mut size_t) -> *mut u8;
    pub fn crocksdb_iter_value(iter: *const DBIterator, vlen: *mut size_t) -> *mut u8;
    pub fn crocksdb_iter_get_error(iter: *const DBIterator, err: *mut *mut c_char);
    // Write batch
    pub fn crocksdb_write(
        db: *mut DBInstance,
        writeopts: *const DBWriteOptions,
        batch: *mut DBWriteBatch,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_write_multi_batch(
        db: *mut DBInstance,
        writeopts: *const DBWriteOptions,
        batch: *const *mut DBWriteBatch,
        batchlen: size_t,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_writebatch_create() -> *mut DBWriteBatch;
    pub fn crocksdb_writebatch_create_with_capacity(cap: size_t) -> *mut DBWriteBatch;
    pub fn crocksdb_writebatch_create_from(rep: *const u8, size: size_t) -> *mut DBWriteBatch;
    pub fn crocksdb_writebatch_destroy(batch: *mut DBWriteBatch);
    pub fn crocksdb_writebatch_clear(batch: *mut DBWriteBatch);
    pub fn crocksdb_writebatch_count(batch: *mut DBWriteBatch) -> c_int;
    pub fn crocksdb_writebatch_put(
        batch: *mut DBWriteBatch,
        key: *const u8,
        klen: size_t,
        val: *const u8,
        vlen: size_t,
    );
    pub fn crocksdb_writebatch_put_cf(
        batch: *mut DBWriteBatch,
        cf: *mut DBCFHandle,
        key: *const u8,
        klen: size_t,
        val: *const u8,
        vlen: size_t,
    );
    pub fn crocksdb_writebatch_merge(
        batch: *mut DBWriteBatch,
        key: *const u8,
        klen: size_t,
        val: *const u8,
        vlen: size_t,
    );
    pub fn crocksdb_writebatch_merge_cf(
        batch: *mut DBWriteBatch,
        cf: *mut DBCFHandle,
        key: *const u8,
        klen: size_t,
        val: *const u8,
        vlen: size_t,
    );
    pub fn crocksdb_writebatch_delete(batch: *mut DBWriteBatch, key: *const u8, klen: size_t);
    pub fn crocksdb_writebatch_delete_cf(
        batch: *mut DBWriteBatch,
        cf: *mut DBCFHandle,
        key: *const u8,
        klen: size_t,
    );
    pub fn crocksdb_writebatch_single_delete(
        batch: *mut DBWriteBatch,
        key: *const u8,
        klen: size_t,
    );
    pub fn crocksdb_writebatch_single_delete_cf(
        batch: *mut DBWriteBatch,
        cf: *mut DBCFHandle,
        key: *const u8,
        klen: size_t,
    );
    pub fn crocksdb_writebatch_delete_range(
        batch: *mut DBWriteBatch,
        begin_key: *const u8,
        begin_keylen: size_t,
        end_key: *const u8,
        end_keylen: size_t,
    );
    pub fn crocksdb_writebatch_delete_range_cf(
        batch: *mut DBWriteBatch,
        cf: *mut DBCFHandle,
        begin_key: *const u8,
        begin_keylen: size_t,
        end_key: *const u8,
        end_keylen: size_t,
    );
    pub fn crocksdb_writebatch_iterate(
        batch: *mut DBWriteBatch,
        state: *mut c_void,
        put_fn: extern "C" fn(
            state: *mut c_void,
            k: *const u8,
            klen: size_t,
            v: *const u8,
            vlen: size_t,
        ),
        deleted_fn: extern "C" fn(state: *mut c_void, k: *const u8, klen: size_t),
    );
    pub fn crocksdb_writebatch_data(batch: *mut DBWriteBatch, size: *mut size_t) -> *const u8;
    pub fn crocksdb_writebatch_set_save_point(batch: *mut DBWriteBatch);
    pub fn crocksdb_writebatch_pop_save_point(batch: *mut DBWriteBatch, err: *mut *mut c_char);
    pub fn crocksdb_writebatch_rollback_to_save_point(
        batch: *mut DBWriteBatch,
        err: *mut *mut c_char,
    );

    // Comparator
    pub fn crocksdb_options_set_comparator(options: *mut Options, cb: *mut DBComparator);
    pub fn crocksdb_comparator_create(
        state: *mut c_void,
        destroy: unsafe extern "C" fn(*mut c_void) -> (),
        compare: unsafe extern "C" fn(
            arg: *mut c_void,
            a: *const c_char,
            alen: size_t,
            b: *const c_char,
            blen: size_t,
        ) -> c_int,
        name_fn: unsafe extern "C" fn(*mut c_void) -> *const c_char,
    ) -> *mut DBComparator;
    pub fn crocksdb_comparator_destroy(cmp: *mut DBComparator);

    // Column Family
    pub fn crocksdb_open_column_families(
        options: *const Options,
        path: *const c_char,
        num_column_families: c_int,
        column_family_names: *const *const c_char,
        column_family_options: *const *const Options,
        column_family_handles: *const *mut DBCFHandle,
        err: *mut *mut c_char,
    ) -> *mut DBInstance;
    pub fn crocksdb_open_column_families_with_ttl(
        options: *const Options,
        path: *const c_char,
        num_column_families: c_int,
        column_family_names: *const *const c_char,
        column_family_options: *const *const Options,
        ttl_array: *const c_int,
        read_only: bool,
        column_family_handles: *const *mut DBCFHandle,
        err: *mut *mut c_char,
    ) -> *mut DBInstance;
    pub fn crocksdb_open_for_read_only_column_families(
        options: *const Options,
        path: *const c_char,
        num_column_families: c_int,
        column_family_names: *const *const c_char,
        column_family_options: *const *const Options,
        column_family_handles: *const *mut DBCFHandle,
        error_if_log_file_exist: bool,
        err: *mut *mut c_char,
    ) -> *mut DBInstance;
    pub fn crocksdb_create_column_family(
        db: *mut DBInstance,
        column_family_options: *const Options,
        column_family_name: *const c_char,
        err: *mut *mut c_char,
    ) -> *mut DBCFHandle;
    pub fn crocksdb_drop_column_family(
        db: *mut DBInstance,
        column_family_handle: *mut DBCFHandle,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_column_family_handle_id(column_family_handle: *mut DBCFHandle) -> u32;
    pub fn crocksdb_column_family_handle_destroy(column_family_handle: *mut DBCFHandle);
    pub fn crocksdb_list_column_families(
        db: *const Options,
        path: *const c_char,
        lencf: *mut size_t,
        err: *mut *mut c_char,
    ) -> *mut *mut c_char;
    pub fn crocksdb_list_column_families_destroy(list: *mut *mut c_char, len: size_t);

    // Flush options
    pub fn crocksdb_flushoptions_create() -> *mut DBFlushOptions;
    pub fn crocksdb_flushoptions_destroy(opt: *mut DBFlushOptions);
    pub fn crocksdb_flushoptions_set_wait(opt: *mut DBFlushOptions, whether_wait: bool);
    pub fn crocksdb_flushoptions_set_allow_write_stall(opt: *mut DBFlushOptions, allow: bool);

    pub fn crocksdb_flush(
        db: *mut DBInstance,
        options: *const DBFlushOptions,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_flush_cf(
        db: *mut DBInstance,
        cf: *mut DBCFHandle,
        options: *const DBFlushOptions,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_flush_cfs(
        db: *mut DBInstance,
        cfs: *const *mut DBCFHandle,
        num_cfs: size_t,
        options: *const DBFlushOptions,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_flush_wal(db: *mut DBInstance, sync: bool, err: *mut *mut c_char);
    pub fn crocksdb_sync_wal(db: *mut DBInstance, err: *mut *mut c_char);

    pub fn crocksdb_get_latest_sequence_number(db: *mut DBInstance) -> u64;

    pub fn crocksdb_approximate_sizes(
        db: *mut DBInstance,
        num_ranges: c_int,
        range_start_key: *const *const u8,
        range_start_key_len: *const size_t,
        range_limit_key: *const *const u8,
        range_limit_key_len: *const size_t,
        sizes: *mut u64,
    );
    pub fn crocksdb_approximate_sizes_cf(
        db: *mut DBInstance,
        cf: *mut DBCFHandle,
        num_ranges: c_int,
        range_start_key: *const *const u8,
        range_start_key_len: *const size_t,
        range_limit_key: *const *const u8,
        range_limit_key_len: *const size_t,
        sizes: *mut u64,
    );
    pub fn crocksdb_approximate_memtable_stats(
        db: *const DBInstance,
        range_start_key: *const u8,
        range_start_key_len: size_t,
        range_limit_key: *const u8,
        range_limit_key_len: size_t,
        count: *mut u64,
        size: *mut u64,
    );
    pub fn crocksdb_approximate_memtable_stats_cf(
        db: *const DBInstance,
        cf: *const DBCFHandle,
        range_start_key: *const u8,
        range_start_key_len: size_t,
        range_limit_key: *const u8,
        range_limit_key_len: size_t,
        count: *mut u64,
        size: *mut u64,
    );
    pub fn crocksdb_compactoptions_create() -> *mut DBCompactOptions;
    pub fn crocksdb_compactoptions_destroy(opt: *mut DBCompactOptions);
    pub fn crocksdb_compactoptions_set_exclusive_manual_compaction(
        opt: *mut DBCompactOptions,
        v: bool,
    );
    pub fn crocksdb_compactoptions_set_change_level(opt: *mut DBCompactOptions, v: bool);
    pub fn crocksdb_compactoptions_set_target_level(opt: *mut DBCompactOptions, v: i32);
    pub fn crocksdb_compactoptions_set_target_path_id(opt: *mut DBCompactOptions, v: i32);
    pub fn crocksdb_compactoptions_set_max_subcompactions(opt: *mut DBCompactOptions, v: i32);
    pub fn crocksdb_compactoptions_set_bottommost_level_compaction(
        opt: *mut DBCompactOptions,
        v: DBBottommostLevelCompaction,
    );

    pub fn crocksdb_fifo_compaction_options_create() -> *mut DBFifoCompactionOptions;
    pub fn crocksdb_fifo_compaction_options_set_max_table_files_size(
        fifo_opts: *mut DBFifoCompactionOptions,
        size: u64,
    );
    pub fn crocksdb_fifo_compaction_options_set_allow_compaction(
        fifo_opts: *mut DBFifoCompactionOptions,
        allow_compaction: bool,
    );
    pub fn crocksdb_fifo_compaction_options_destroy(fifo_opts: *mut DBFifoCompactionOptions);

    pub fn crocksdb_compact_range(
        db: *mut DBInstance,
        start_key: *const u8,
        start_key_len: size_t,
        limit_key: *const u8,
        limit_key_len: size_t,
    );
    pub fn crocksdb_compact_range_cf(
        db: *mut DBInstance,
        cf: *mut DBCFHandle,
        start_key: *const u8,
        start_key_len: size_t,
        limit_key: *const u8,
        limit_key_len: size_t,
    );
    pub fn crocksdb_compact_range_cf_opt(
        db: *mut DBInstance,
        cf: *mut DBCFHandle,
        compact_options: *mut DBCompactOptions,
        start_key: *const u8,
        start_key_len: size_t,
        limit_key: *const u8,
        limit_key_len: size_t,
    );
    pub fn crocksdb_delete_files_in_range(
        db: *mut DBInstance,
        range_start_key: *const u8,
        range_start_key_len: size_t,
        range_limit_key: *const u8,
        range_limit_key_len: size_t,
        include_end: bool,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_delete_files_in_range_cf(
        db: *mut DBInstance,
        cf: *mut DBCFHandle,
        range_start_key: *const u8,
        range_start_key_len: size_t,
        range_limit_key: *const u8,
        range_limit_key_len: size_t,
        include_end: bool,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_delete_files_in_ranges_cf(
        db: *mut DBInstance,
        cf: *mut DBCFHandle,
        start_keys: *const *const u8,
        start_keys_lens: *const size_t,
        limit_keys: *const *const u8,
        limit_keys_lens: *const size_t,
        num_ranges: size_t,
        include_end: bool,
        errptr: *mut *mut c_char,
    );
    pub fn crocksdb_create_map_property() -> *mut DBMapProperty;
    pub fn crocksdb_destroy_map_property(info: *mut DBMapProperty);
    pub fn crocksdb_get_map_property_cf(
        db: *mut DBInstance,
        cf: *mut DBCFHandle,
        name: *const c_char,
        info: *mut DBMapProperty,
    ) -> bool;

    pub fn crocksdb_map_property_value(
        info: *const DBMapProperty,
        propname: *const c_char,
    ) -> *mut c_char;

    pub fn crocksdb_map_property_int_value(
        info: *const DBMapProperty,
        propname: *const c_char,
    ) -> u64;

    pub fn crocksdb_property_value(db: *mut DBInstance, propname: *const c_char) -> *mut c_char;
    pub fn crocksdb_property_value_cf(
        db: *mut DBInstance,
        cf: *mut DBCFHandle,
        propname: *const c_char,
    ) -> *mut c_char;
    // Compaction filter
    pub fn crocksdb_compactionfilter_create(
        state: *mut c_void,
        destructor: extern "C" fn(*mut c_void),
        filter: extern "C" fn(
            *mut c_void,
            c_int,
            *const u8,
            size_t,
            *const u8,
            size_t,
            *mut *mut u8,
            *mut size_t,
            *mut bool,
        ) -> bool,
        name: extern "C" fn(*mut c_void) -> *const c_char,
    ) -> *mut DBCompactionFilter;
    pub fn crocksdb_compactionfilter_set_ignore_snapshots(
        filter: *mut DBCompactionFilter,
        ignore_snapshot: bool,
    );
    pub fn crocksdb_compactionfilter_destroy(filter: *mut DBCompactionFilter);

    // Compaction filter context
    pub fn crocksdb_compactionfiltercontext_is_full_compaction(
        context: *const DBCompactionFilterContext,
    ) -> bool;
    pub fn crocksdb_compactionfiltercontext_is_manual_compaction(
        context: *const DBCompactionFilterContext,
    ) -> bool;

    // Compaction filter factory
    pub fn crocksdb_compactionfilterfactory_create(
        state: *mut c_void,
        destructor: extern "C" fn(*mut c_void),
        create_compaction_filter: extern "C" fn(
            *mut c_void,
            *const DBCompactionFilterContext,
        ) -> *mut DBCompactionFilter,
        name: extern "C" fn(*mut c_void) -> *const c_char,
    ) -> *mut DBCompactionFilterFactory;
    pub fn crocksdb_compactionfilterfactory_destroy(factory: *mut DBCompactionFilterFactory);

    // Env
    pub fn crocksdb_default_env_create() -> *mut DBEnv;
    pub fn crocksdb_mem_env_create() -> *mut DBEnv;
    pub fn crocksdb_ctr_encrypted_env_create(
        base_env: *mut DBEnv,
        ciphertext: *const c_char,
        ciphertext_len: size_t,
    ) -> *mut DBEnv;
    pub fn crocksdb_env_file_exists(env: *mut DBEnv, path: *const c_char, err: *mut *mut c_char);
    pub fn crocksdb_env_delete_file(env: *mut DBEnv, path: *const c_char, err: *mut *mut c_char);
    pub fn crocksdb_env_destroy(env: *mut DBEnv);

    // EnvOptions
    pub fn crocksdb_envoptions_create() -> *mut EnvOptions;
    pub fn crocksdb_envoptions_destroy(opt: *mut EnvOptions);

    // SequentialFile
    pub fn crocksdb_sequential_file_create(
        env: *mut DBEnv,
        path: *const c_char,
        opts: *mut EnvOptions,
        err: *mut *mut c_char,
    ) -> *mut DBSequentialFile;
    pub fn crocksdb_sequential_file_read(
        file: *mut DBSequentialFile,
        n: size_t,
        buf: *mut u8,
        err: *mut *mut c_char,
    ) -> size_t;
    pub fn crocksdb_sequential_file_skip(
        file: *mut DBSequentialFile,
        n: size_t,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_sequential_file_destroy(file: *mut DBSequentialFile);

    // IngestExternalFileOptions
    pub fn crocksdb_ingestexternalfileoptions_create() -> *mut IngestExternalFileOptions;
    pub fn crocksdb_ingestexternalfileoptions_set_move_files(
        opt: *mut IngestExternalFileOptions,
        move_files: bool,
    );
    pub fn crocksdb_ingestexternalfileoptions_set_snapshot_consistency(
        opt: *mut IngestExternalFileOptions,
        snapshot_consistency: bool,
    );
    pub fn crocksdb_ingestexternalfileoptions_set_allow_global_seqno(
        opt: *mut IngestExternalFileOptions,
        allow_global_seqno: bool,
    );
    pub fn crocksdb_ingestexternalfileoptions_set_allow_blocking_flush(
        opt: *mut IngestExternalFileOptions,
        allow_blocking_flush: bool,
    );
    pub fn crocksdb_ingestexternalfileoptions_destroy(opt: *mut IngestExternalFileOptions);

    // KeyManagedEncryptedEnv
    #[cfg(feature = "encryption")]
    pub fn crocksdb_file_encryption_info_create() -> *mut DBFileEncryptionInfo;
    #[cfg(feature = "encryption")]
    pub fn crocksdb_file_encryption_info_destroy(file_info: *mut DBFileEncryptionInfo);
    #[cfg(feature = "encryption")]
    pub fn crocksdb_file_encryption_info_method(
        file_info: *mut DBFileEncryptionInfo,
    ) -> DBEncryptionMethod;
    #[cfg(feature = "encryption")]
    pub fn crocksdb_file_encryption_info_key(
        file_info: *mut DBFileEncryptionInfo,
        key_len: *mut size_t,
    ) -> *const c_char;
    #[cfg(feature = "encryption")]
    pub fn crocksdb_file_encryption_info_iv(
        file_info: *mut DBFileEncryptionInfo,
        iv_len: *mut size_t,
    ) -> *const c_char;
    #[cfg(feature = "encryption")]
    pub fn crocksdb_file_encryption_info_set_method(
        file_info: *mut DBFileEncryptionInfo,
        method: DBEncryptionMethod,
    );
    #[cfg(feature = "encryption")]
    pub fn crocksdb_file_encryption_info_set_key(
        file_info: *mut DBFileEncryptionInfo,
        key: *const c_char,
        key_len: size_t,
    );
    #[cfg(feature = "encryption")]
    pub fn crocksdb_file_encryption_info_set_iv(
        file_info: *mut DBFileEncryptionInfo,
        iv: *const c_char,
        iv_len: size_t,
    );

    #[cfg(feature = "encryption")]
    pub fn crocksdb_encryption_key_manager_create(
        state: *mut c_void,
        destructor: extern "C" fn(*mut c_void),
        get_file: extern "C" fn(
            *mut c_void,
            *const c_char,
            *mut DBFileEncryptionInfo,
        ) -> *const c_char,
        new_file: extern "C" fn(
            *mut c_void,
            *const c_char,
            *mut DBFileEncryptionInfo,
        ) -> *const c_char,
        delete_file: extern "C" fn(*mut c_void, *const c_char) -> *const c_char,
        link_file: extern "C" fn(*mut c_void, *const c_char, *const c_char) -> *const c_char,
        rename_file: extern "C" fn(*mut c_void, *const c_char, *const c_char) -> *const c_char,
    ) -> *mut DBEncryptionKeyManagerInstance;
    #[cfg(feature = "encryption")]
    pub fn crocksdb_encryption_key_manager_destroy(
        key_manager: *mut DBEncryptionKeyManagerInstance,
    );
    #[cfg(feature = "encryption")]
    pub fn crocksdb_encryption_key_manager_get_file(
        key_manager: *mut DBEncryptionKeyManagerInstance,
        fname: *const c_char,
        file_info: *mut DBFileEncryptionInfo,
    ) -> *const c_char;
    #[cfg(feature = "encryption")]
    pub fn crocksdb_encryption_key_manager_new_file(
        key_manager: *mut DBEncryptionKeyManagerInstance,
        fname: *const c_char,
        file_info: *mut DBFileEncryptionInfo,
    ) -> *const c_char;
    #[cfg(feature = "encryption")]
    pub fn crocksdb_encryption_key_manager_delete_file(
        key_manager: *mut DBEncryptionKeyManagerInstance,
        fname: *const c_char,
    ) -> *const c_char;
    #[cfg(feature = "encryption")]
    pub fn crocksdb_encryption_key_manager_link_file(
        key_manager: *mut DBEncryptionKeyManagerInstance,
        src_fname: *const c_char,
        dst_fname: *const c_char,
    ) -> *const c_char;
    #[cfg(feature = "encryption")]
    pub fn crocksdb_encryption_key_manager_rename_file(
        key_manager: *mut DBEncryptionKeyManagerInstance,
        src_fname: *const c_char,
        dst_fname: *const c_char,
    ) -> *const c_char;

    #[cfg(feature = "encryption")]
    pub fn crocksdb_key_managed_encrypted_env_create(
        base_env: *mut DBEnv,
        key_manager: *mut DBEncryptionKeyManagerInstance,
    ) -> *mut DBEnv;

    // SstFileReader
    pub fn crocksdb_sstfilereader_create(io_options: *const Options) -> *mut SstFileReader;

    pub fn crocksdb_sstfilereader_open(
        reader: *mut SstFileReader,
        name: *const c_char,
        err: *mut *mut c_char,
    );

    pub fn crocksdb_sstfilereader_new_iterator(
        reader: *mut SstFileReader,
        options: *const DBReadOptions,
    ) -> *mut DBIterator;

    pub fn crocksdb_sstfilereader_read_table_properties(
        reader: *const SstFileReader,
        ctx: *mut c_void,
        callback: extern "C" fn(*mut c_void, *const DBTableProperties),
    );

    pub fn crocksdb_sstfilereader_verify_checksum(
        reader: *mut SstFileReader,
        errptr: *mut *mut c_char,
    );

    pub fn crocksdb_sstfilereader_destroy(reader: *mut SstFileReader);

    // SstFileWriter
    pub fn crocksdb_sstfilewriter_create(
        env: *mut EnvOptions,
        io_options: *const Options,
    ) -> *mut SstFileWriter;
    pub fn crocksdb_sstfilewriter_create_cf(
        env: *mut EnvOptions,
        io_options: *const Options,
        cf: *mut DBCFHandle,
    ) -> *mut SstFileWriter;
    pub fn crocksdb_sstfilewriter_open(
        writer: *mut SstFileWriter,
        name: *const c_char,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_sstfilewriter_put(
        writer: *mut SstFileWriter,
        key: *const u8,
        key_len: size_t,
        val: *const u8,
        val_len: size_t,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_sstfilewriter_merge(
        writer: *mut SstFileWriter,
        key: *const u8,
        key_len: size_t,
        val: *const u8,
        val_len: size_t,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_sstfilewriter_delete(
        writer: *mut SstFileWriter,
        key: *const u8,
        key_len: size_t,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_sstfilewriter_delete_range(
        writer: *mut SstFileWriter,
        begin_key: *const u8,
        begin_key_len: size_t,
        end_key: *const u8,
        end_key_len: size_t,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_sstfilewriter_finish(
        writer: *mut SstFileWriter,
        info: *mut ExternalSstFileInfo,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_sstfilewriter_file_size(writer: *mut SstFileWriter) -> u64;
    pub fn crocksdb_sstfilewriter_destroy(writer: *mut SstFileWriter);

    // ExternalSstFileInfo
    pub fn crocksdb_externalsstfileinfo_create() -> *mut ExternalSstFileInfo;
    pub fn crocksdb_externalsstfileinfo_destroy(info: *mut ExternalSstFileInfo);
    pub fn crocksdb_externalsstfileinfo_file_path(
        info: *mut ExternalSstFileInfo,
        size: *mut size_t,
    ) -> *const u8;
    pub fn crocksdb_externalsstfileinfo_smallest_key(
        info: *mut ExternalSstFileInfo,
        size: *mut size_t,
    ) -> *const u8;
    pub fn crocksdb_externalsstfileinfo_largest_key(
        info: *mut ExternalSstFileInfo,
        size: *mut size_t,
    ) -> *const u8;
    pub fn crocksdb_externalsstfileinfo_sequence_number(info: *mut ExternalSstFileInfo) -> u64;
    pub fn crocksdb_externalsstfileinfo_file_size(info: *mut ExternalSstFileInfo) -> u64;
    pub fn crocksdb_externalsstfileinfo_num_entries(info: *mut ExternalSstFileInfo) -> u64;

    pub fn crocksdb_ingest_external_file(
        db: *mut DBInstance,
        file_list: *const *const c_char,
        list_len: size_t,
        opt: *const IngestExternalFileOptions,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_ingest_external_file_cf(
        db: *mut DBInstance,
        handle: *const DBCFHandle,
        file_list: *const *const c_char,
        list_len: size_t,
        opt: *const IngestExternalFileOptions,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_ingest_external_file_optimized(
        db: *mut DBInstance,
        handle: *const DBCFHandle,
        file_list: *const *const c_char,
        list_len: size_t,
        opt: *const IngestExternalFileOptions,
        err: *mut *mut c_char,
    ) -> bool;

    // Restore Option
    pub fn crocksdb_restore_options_create() -> *mut DBRestoreOptions;
    pub fn crocksdb_restore_options_destroy(ropts: *mut DBRestoreOptions);
    pub fn crocksdb_restore_options_set_keep_log_files(ropts: *mut DBRestoreOptions, v: c_int);

    // Backup engine
    // TODO: add more ffis about backup engine.
    pub fn crocksdb_backup_engine_open(
        options: *const Options,
        path: *const c_char,
        err: *mut *mut c_char,
    ) -> *mut DBBackupEngine;
    pub fn crocksdb_backup_engine_create_new_backup(
        be: *mut DBBackupEngine,
        db: *mut DBInstance,
        err: *mut *mut c_char,
    );
    pub fn crocksdb_backup_engine_close(be: *mut DBBackupEngine);
    pub fn crocksdb_backup_engine_restore_db_from_latest_backup(
        be: *mut DBBackupEngine,
        db_path: *const c_char,
        wal_path: *const c_char,
        ropts: *const DBRestoreOptions,
        err: *mut *mut c_char,
    );
    // SliceTransform
    pub fn crocksdb_slicetransform_create(
        state: *mut c_void,
        destructor: extern "C" fn(*mut c_void),
        transform: extern "C" fn(*mut c_void, *const u8, size_t, *mut size_t) -> *const u8,
        in_domain: extern "C" fn(*mut c_void, *const u8, size_t) -> u8,
        in_range: extern "C" fn(*mut c_void, *const u8, size_t) -> u8,
        name: extern "C" fn(*mut c_void) -> *const c_char,
    ) -> *mut DBSliceTransform;
    pub fn crocksdb_slicetransform_destroy(transform: *mut DBSliceTransform);
    pub fn crocksdb_logger_create(
        state: *mut c_void,
        destructor: extern "C" fn(*mut c_void),
        logv: extern "C" fn(ctx: *mut c_void, log_level: DBInfoLogLevel, log: *const c_char),
    ) -> *mut DBLogger;
    pub fn crocksdb_create_env_logger(fname: *const libc::c_char, env: *mut DBEnv)
        -> *mut DBLogger;
    pub fn crocksdb_create_log_from_options(
        path: *const c_char,
        options: *mut Options,
        err: *mut *mut c_char,
    ) -> *mut DBLogger;
    pub fn crocksdb_log_destroy(logger: *mut DBLogger);
    pub fn crocksdb_get_pinned(
        db: *mut DBInstance,
        readopts: *const DBReadOptions,
        k: *const u8,
        kLen: size_t,
        err: *mut *mut c_char,
    ) -> *mut DBPinnableSlice;
    pub fn crocksdb_get_pinned_cf(
        db: *mut DBInstance,
        readopts: *const DBReadOptions,
        cf_handle: *mut DBCFHandle,
        k: *const u8,
        kLen: size_t,
        err: *mut *mut c_char,
    ) -> *mut DBPinnableSlice;
    pub fn crocksdb_pinnableslice_value(
        s: *const DBPinnableSlice,
        valLen: *mut size_t,
    ) -> *const u8;
    pub fn crocksdb_pinnableslice_destroy(v: *mut DBPinnableSlice);
    pub fn crocksdb_get_supported_compression_number() -> size_t;
    pub fn crocksdb_get_supported_compression(v: *mut DBCompressionType, l: size_t);

    pub fn crocksdb_user_collected_properties_add(
        props: *mut DBUserCollectedProperties,
        key: *const u8,
        key_len: size_t,
        value: *const u8,
        value_len: size_t,
    );

    pub fn crocksdb_user_collected_properties_iter_create(
        props: *const DBUserCollectedProperties,
    ) -> *mut DBUserCollectedPropertiesIterator;

    pub fn crocksdb_user_collected_properties_iter_destroy(
        it: *mut DBUserCollectedPropertiesIterator,
    );

    pub fn crocksdb_user_collected_properties_iter_valid(
        it: *const DBUserCollectedPropertiesIterator,
    ) -> bool;

    pub fn crocksdb_user_collected_properties_iter_next(it: *mut DBUserCollectedPropertiesIterator);

    pub fn crocksdb_user_collected_properties_iter_key(
        it: *const DBUserCollectedPropertiesIterator,
        klen: *mut size_t,
    ) -> *const u8;

    pub fn crocksdb_user_collected_properties_iter_value(
        it: *const DBUserCollectedPropertiesIterator,
        vlen: *mut size_t,
    ) -> *const u8;

    pub fn crocksdb_table_properties_get_u64(
        props: *const DBTableProperties,
        prop: DBTableProperty,
    ) -> u64;

    pub fn crocksdb_table_properties_get_str(
        props: *const DBTableProperties,
        prop: DBTableProperty,
        slen: *mut size_t,
    ) -> *const u8;

    pub fn crocksdb_table_properties_get_user_properties(
        props: *const DBTableProperties,
    ) -> *const DBUserCollectedProperties;

    pub fn crocksdb_user_collected_properties_get(
        props: *const DBUserCollectedProperties,
        key: *const u8,
        klen: size_t,
        vlen: *mut size_t,
    ) -> *const u8;

    pub fn crocksdb_user_collected_properties_len(
        props: *const DBUserCollectedProperties,
    ) -> size_t;

    pub fn crocksdb_table_properties_collection_len(
        props: *const DBTablePropertiesCollection,
    ) -> size_t;

    pub fn crocksdb_table_properties_collection_destroy(props: *mut DBTablePropertiesCollection);

    pub fn crocksdb_table_properties_collection_iter_create(
        props: *const DBTablePropertiesCollection,
    ) -> *mut DBTablePropertiesCollectionIterator;

    pub fn crocksdb_table_properties_collection_iter_destroy(
        it: *mut DBTablePropertiesCollectionIterator,
    );

    pub fn crocksdb_table_properties_collection_iter_valid(
        it: *const DBTablePropertiesCollectionIterator,
    ) -> bool;

    pub fn crocksdb_table_properties_collection_iter_next(
        it: *mut DBTablePropertiesCollectionIterator,
    );

    pub fn crocksdb_table_properties_collection_iter_key(
        it: *const DBTablePropertiesCollectionIterator,
        klen: *mut size_t,
    ) -> *const u8;

    pub fn crocksdb_table_properties_collection_iter_value(
        it: *const DBTablePropertiesCollectionIterator,
    ) -> *const DBTableProperties;

    pub fn crocksdb_table_properties_collector_create(
        state: *mut c_void,
        name: extern "C" fn(*mut c_void) -> *const c_char,
        destruct: extern "C" fn(*mut c_void),
        add_userkey: extern "C" fn(
            *mut c_void,
            *const u8,
            size_t,
            *const u8,
            size_t,
            c_int,
            u64,
            u64,
        ),
        finish: extern "C" fn(*mut c_void, *mut DBUserCollectedProperties),
    ) -> *mut DBTablePropertiesCollector;

    pub fn crocksdb_table_properties_collector_destroy(c: *mut DBTablePropertiesCollector);

    pub fn crocksdb_table_properties_collector_factory_create(
        state: *mut c_void,
        name: extern "C" fn(*mut c_void) -> *const c_char,
        destruct: extern "C" fn(*mut c_void),
        create_table_properties_collector: extern "C" fn(
            *mut c_void,
            u32,
        )
            -> *mut DBTablePropertiesCollector,
    ) -> *mut DBTablePropertiesCollectorFactory;

    pub fn crocksdb_table_properties_collector_factory_destroy(
        f: *mut DBTablePropertiesCollectorFactory,
    );

    pub fn crocksdb_options_add_table_properties_collector_factory(
        options: *mut Options,
        f: *mut DBTablePropertiesCollectorFactory,
    );

    pub fn crocksdb_get_properties_of_all_tables(
        db: *mut DBInstance,
        errptr: *mut *mut c_char,
    ) -> *mut DBTablePropertiesCollection;

    pub fn crocksdb_get_properties_of_all_tables_cf(
        db: *mut DBInstance,
        cf: *mut DBCFHandle,
        errptr: *mut *mut c_char,
    ) -> *mut DBTablePropertiesCollection;

    pub fn crocksdb_get_properties_of_tables_in_range(
        db: *mut DBInstance,
        cf: *mut DBCFHandle,
        num_ranges: c_int,
        start_keys: *const *const u8,
        start_keys_lens: *const size_t,
        limit_keys: *const *const u8,
        limit_keys_lens: *const size_t,
        errptr: *mut *mut c_char,
    ) -> *mut DBTablePropertiesCollection;

    pub fn crocksdb_flushjobinfo_cf_name(
        info: *const DBFlushJobInfo,
        size: *mut size_t,
    ) -> *const c_char;
    pub fn crocksdb_flushjobinfo_file_path(
        info: *const DBFlushJobInfo,
        size: *mut size_t,
    ) -> *const c_char;
    pub fn crocksdb_flushjobinfo_table_properties(
        info: *const DBFlushJobInfo,
    ) -> *const DBTableProperties;
    pub fn crocksdb_flushjobinfo_triggered_writes_slowdown(info: *const DBFlushJobInfo) -> bool;
    pub fn crocksdb_flushjobinfo_triggered_writes_stop(info: *const DBFlushJobInfo) -> bool;

    pub fn crocksdb_compactionjobinfo_status(
        info: *const DBCompactionJobInfo,
        errptr: *mut *mut c_char,
    );
    pub fn crocksdb_compactionjobinfo_cf_name(
        info: *const DBCompactionJobInfo,
        size: *mut size_t,
    ) -> *const c_char;
    pub fn crocksdb_compactionjobinfo_input_files_count(info: *const DBCompactionJobInfo)
        -> size_t;
    pub fn crocksdb_compactionjobinfo_input_file_at(
        info: *const DBCompactionJobInfo,
        pos: size_t,
        len: *mut size_t,
    ) -> *const c_char;
    pub fn crocksdb_compactionjobinfo_output_files_count(
        info: *const DBCompactionJobInfo,
    ) -> size_t;
    pub fn crocksdb_compactionjobinfo_output_file_at(
        info: *const DBCompactionJobInfo,
        pos: size_t,
        len: *mut size_t,
    ) -> *const c_char;
    pub fn crocksdb_compactionjobinfo_table_properties(
        info: *const DBCompactionJobInfo,
    ) -> *const DBTablePropertiesCollection;
    pub fn crocksdb_compactionjobinfo_elapsed_micros(info: *const DBCompactionJobInfo) -> u64;
    pub fn crocksdb_compactionjobinfo_num_corrupt_keys(info: *const DBCompactionJobInfo) -> u64;
    pub fn crocksdb_compactionjobinfo_output_level(info: *const DBCompactionJobInfo) -> c_int;
    pub fn crocksdb_compactionjobinfo_input_records(info: *const DBCompactionJobInfo) -> u64;
    pub fn crocksdb_compactionjobinfo_output_records(info: *const DBCompactionJobInfo) -> u64;
    pub fn crocksdb_compactionjobinfo_total_input_bytes(info: *const DBCompactionJobInfo) -> u64;
    pub fn crocksdb_compactionjobinfo_total_output_bytes(info: *const DBCompactionJobInfo) -> u64;
    pub fn crocksdb_compactionjobinfo_compaction_reason(
        info: *const DBCompactionJobInfo,
    ) -> CompactionReason;

    pub fn crocksdb_externalfileingestioninfo_cf_name(
        info: *const DBIngestionInfo,
        size: *mut size_t,
    ) -> *const c_char;
    pub fn crocksdb_externalfileingestioninfo_internal_file_path(
        info: *const DBIngestionInfo,
        size: *mut size_t,
    ) -> *const c_char;
    pub fn crocksdb_externalfileingestioninfo_table_properties(
        info: *const DBIngestionInfo,
    ) -> *const DBTableProperties;

    pub fn crocksdb_writestallinfo_cf_name(
        info: *const DBWriteStallInfo,
        size: *mut size_t,
    ) -> *const c_char;
    pub fn crocksdb_writestallinfo_prev(
        info: *const DBWriteStallInfo,
    ) -> *const WriteStallCondition;
    pub fn crocksdb_writestallinfo_cur(info: *const DBWriteStallInfo)
        -> *const WriteStallCondition;

    pub fn crocksdb_eventlistener_create(
        state: *mut c_void,
        destructor: extern "C" fn(*mut c_void),
        flush: extern "C" fn(*mut c_void, *mut DBInstance, *const DBFlushJobInfo),
        compact: extern "C" fn(*mut c_void, *mut DBInstance, *const DBCompactionJobInfo),
        ingest: extern "C" fn(*mut c_void, *mut DBInstance, *const DBIngestionInfo),
        bg_error: extern "C" fn(*mut c_void, DBBackgroundErrorReason, *mut DBStatusPtr),
        stall_conditions: extern "C" fn(*mut c_void, *const DBWriteStallInfo),
    ) -> *mut DBEventListener;
    pub fn crocksdb_eventlistener_destroy(et: *mut DBEventListener);
    pub fn crocksdb_options_add_eventlistener(opt: *mut Options, et: *mut DBEventListener);
    // Get All Key Versions
    pub fn crocksdb_keyversions_destroy(kvs: *mut DBKeyVersions);

    pub fn crocksdb_get_all_key_versions(
        db: *mut DBInstance,
        begin_key: *const u8,
        begin_keylen: size_t,
        end_key: *const u8,
        end_keylen: size_t,
        errptr: *mut *mut c_char,
    ) -> *mut DBKeyVersions;

    pub fn crocksdb_keyversions_count(kvs: *mut DBKeyVersions) -> size_t;

    pub fn crocksdb_keyversions_key(kvs: *mut DBKeyVersions, index: usize) -> *const c_char;

    pub fn crocksdb_keyversions_value(kvs: *mut DBKeyVersions, index: usize) -> *const c_char;

    pub fn crocksdb_keyversions_seq(kvs: *mut DBKeyVersions, index: usize) -> u64;

    pub fn crocksdb_keyversions_type(kvs: *mut DBKeyVersions, index: usize) -> c_int;

    pub fn crocksdb_set_external_sst_file_global_seq_no(
        db: *mut DBInstance,
        handle: *mut DBCFHandle,
        file: *const c_char,
        seq_no: u64,
        err: *mut *mut c_char,
    ) -> u64;

    pub fn crocksdb_get_column_family_meta_data(
        db: *mut DBInstance,
        cf: *mut DBCFHandle,
        meta: *mut DBColumnFamilyMetaData,
    );

    pub fn crocksdb_column_family_meta_data_create() -> *mut DBColumnFamilyMetaData;
    pub fn crocksdb_column_family_meta_data_destroy(meta: *mut DBColumnFamilyMetaData);
    pub fn crocksdb_column_family_meta_data_level_count(
        meta: *const DBColumnFamilyMetaData,
    ) -> size_t;
    pub fn crocksdb_column_family_meta_data_level_data(
        meta: *const DBColumnFamilyMetaData,
        n: size_t,
    ) -> *const DBLevelMetaData;

    pub fn crocksdb_level_meta_data_file_count(meta: *const DBLevelMetaData) -> size_t;
    pub fn crocksdb_level_meta_data_file_data(
        meta: *const DBLevelMetaData,
        n: size_t,
    ) -> *const DBSstFileMetaData;

    pub fn crocksdb_sst_file_meta_data_size(meta: *const DBSstFileMetaData) -> size_t;
    pub fn crocksdb_sst_file_meta_data_name(meta: *const DBSstFileMetaData) -> *const c_char;
    pub fn crocksdb_sst_file_meta_data_smallestkey(
        meta: *const DBSstFileMetaData,
        len: *mut size_t,
    ) -> *const c_char;
    pub fn crocksdb_sst_file_meta_data_largestkey(
        meta: *const DBSstFileMetaData,
        len: *mut size_t,
    ) -> *const c_char;

    pub fn crocksdb_compaction_options_create() -> *mut DBCompactionOptions;
    pub fn crocksdb_compaction_options_destroy(opts: *mut DBCompactionOptions);
    pub fn crocksdb_compaction_options_set_compression(
        opts: *mut DBCompactionOptions,
        compression: DBCompressionType,
    );
    pub fn crocksdb_compaction_options_set_output_file_size_limit(
        opts: *mut DBCompactionOptions,
        size: size_t,
    );
    pub fn crocksdb_compaction_options_set_max_subcompactions(
        opts: *mut DBCompactionOptions,
        v: i32,
    );

    pub fn crocksdb_compact_files_cf(
        db: *mut DBInstance,
        cf: *mut DBCFHandle,
        opts: *const DBCompactionOptions,
        input_file_names: *const *const c_char,
        input_file_count: size_t,
        output_level: c_int,
        errptr: *mut *mut c_char,
    );

    pub fn crocksdb_get_perf_level() -> c_int;
    pub fn crocksdb_set_perf_level(level: c_int);
    pub fn crocksdb_get_perf_context() -> *mut DBPerfContext;
    pub fn crocksdb_perf_context_reset(ctx: *mut DBPerfContext);
    pub fn crocksdb_perf_context_user_key_comparison_count(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_block_cache_hit_count(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_block_read_count(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_block_read_byte(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_block_read_time(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_block_checksum_time(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_block_decompress_time(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_get_read_bytes(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_multiget_read_bytes(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_iter_read_bytes(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_internal_key_skipped_count(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_internal_delete_skipped_count(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_internal_recent_skipped_count(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_internal_merge_count(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_get_snapshot_time(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_get_from_memtable_time(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_get_from_memtable_count(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_get_post_process_time(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_get_from_output_files_time(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_seek_on_memtable_time(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_seek_on_memtable_count(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_next_on_memtable_count(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_prev_on_memtable_count(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_seek_child_seek_time(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_seek_child_seek_count(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_seek_min_heap_time(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_seek_max_heap_time(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_seek_internal_seek_time(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_find_next_user_entry_time(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_write_wal_time(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_write_memtable_time(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_write_delay_time(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_write_pre_and_post_process_time(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_db_mutex_lock_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_write_thread_wait_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_write_scheduling_flushes_compactions_time(
        ctx: *mut DBPerfContext,
    ) -> u64;
    pub fn crocksdb_perf_context_db_condition_wait_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_merge_operator_time_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_read_index_block_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_read_filter_block_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_new_table_block_iter_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_new_table_iterator_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_block_seek_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_find_table_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_bloom_memtable_hit_count(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_bloom_memtable_miss_count(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_bloom_sst_hit_count(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_bloom_sst_miss_count(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_env_new_sequential_file_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_env_new_random_access_file_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_env_new_writable_file_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_env_reuse_writable_file_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_env_new_random_rw_file_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_env_new_directory_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_env_file_exists_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_env_get_children_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_env_get_children_file_attributes_nanos(
        ctx: *mut DBPerfContext,
    ) -> u64;
    pub fn crocksdb_perf_context_env_delete_file_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_env_create_dir_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_env_create_dir_if_missing_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_env_delete_dir_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_env_get_file_size_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_env_get_file_modification_time_nanos(
        ctx: *mut DBPerfContext,
    ) -> u64;
    pub fn crocksdb_perf_context_env_rename_file_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_env_link_file_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_env_lock_file_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_env_unlock_file_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_env_new_logger_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_encrypt_data_nanos(ctx: *mut DBPerfContext) -> u64;
    pub fn crocksdb_perf_context_decrypt_data_nanos(ctx: *mut DBPerfContext) -> u64;

    pub fn crocksdb_get_iostats_context() -> *mut DBIOStatsContext;
    pub fn crocksdb_iostats_context_reset(ctx: *mut DBIOStatsContext);
    pub fn crocksdb_iostats_context_bytes_written(ctx: *mut DBIOStatsContext) -> u64;
    pub fn crocksdb_iostats_context_bytes_read(ctx: *mut DBIOStatsContext) -> u64;
    pub fn crocksdb_iostats_context_open_nanos(ctx: *mut DBIOStatsContext) -> u64;
    pub fn crocksdb_iostats_context_allocate_nanos(ctx: *mut DBIOStatsContext) -> u64;
    pub fn crocksdb_iostats_context_write_nanos(ctx: *mut DBIOStatsContext) -> u64;
    pub fn crocksdb_iostats_context_read_nanos(ctx: *mut DBIOStatsContext) -> u64;
    pub fn crocksdb_iostats_context_range_sync_nanos(ctx: *mut DBIOStatsContext) -> u64;
    pub fn crocksdb_iostats_context_fsync_nanos(ctx: *mut DBIOStatsContext) -> u64;
    pub fn crocksdb_iostats_context_prepare_write_nanos(ctx: *mut DBIOStatsContext) -> u64;
    pub fn crocksdb_iostats_context_logger_nanos(ctx: *mut DBIOStatsContext) -> u64;

    pub fn crocksdb_run_ldb_tool(argc: c_int, argv: *const *const c_char, opts: *const Options);
    pub fn crocksdb_run_sst_dump_tool(
        argc: c_int,
        argv: *const *const c_char,
        opts: *const Options,
    );
}

// Titan
extern "C" {
    pub fn ctitandb_open_column_families(
        path: *const c_char,
        titan_options: *const DBTitanDBOptions,
        num_column_families: c_int,
        column_family_names: *const *const c_char,
        titan_column_family_options: *const *const DBTitanDBOptions,
        column_family_handles: *const *mut DBCFHandle,
        err: *mut *mut c_char,
    ) -> *mut DBInstance;

    pub fn ctitandb_create_column_family(
        db: *mut DBInstance,
        titan_column_family_options: *const DBTitanDBOptions,
        column_family_name: *const c_char,
        err: *mut *mut c_char,
    ) -> *mut DBCFHandle;

    pub fn ctitandb_options_create() -> *mut DBTitanDBOptions;
    pub fn ctitandb_options_destroy(opts: *mut DBTitanDBOptions);
    pub fn ctitandb_options_copy(opts: *mut DBTitanDBOptions) -> *mut DBTitanDBOptions;
    pub fn ctitandb_options_set_rocksdb_options(
        opts: *mut DBTitanDBOptions,
        rocksdb_opts: *const Options,
    );
    pub fn ctitandb_get_titan_options_cf(
        db: *mut DBInstance,
        cf: *mut DBCFHandle,
    ) -> *mut DBTitanDBOptions;
    pub fn ctitandb_get_titan_db_options(db: *mut DBInstance) -> *mut DBTitanDBOptions;
    pub fn ctitandb_options_dirname(opts: *mut DBTitanDBOptions) -> *const c_char;
    pub fn ctitandb_options_set_dirname(opts: *mut DBTitanDBOptions, name: *const c_char);
    pub fn ctitandb_options_min_blob_size(opts: *mut DBTitanDBOptions) -> u64;
    pub fn ctitandb_options_set_min_blob_size(opts: *mut DBTitanDBOptions, size: u64);
    pub fn ctitandb_options_blob_file_compression(opts: *mut DBTitanDBOptions)
        -> DBCompressionType;
    pub fn ctitandb_options_set_gc_merge_rewrite(opts: *mut DBTitanDBOptions, enable: bool);
    pub fn ctitandb_options_set_blob_file_compression(
        opts: *mut DBTitanDBOptions,
        t: DBCompressionType,
    );

    pub fn ctitandb_decode_blob_index(
        value: *const u8,
        value_size: u64,
        index: *mut DBTitanBlobIndex,
        errptr: *mut *mut c_char,
    );
    pub fn ctitandb_encode_blob_index(
        index: &DBTitanBlobIndex,
        value: *mut *mut u8,
        value_size: *mut u64,
    );

    pub fn ctitandb_options_set_disable_background_gc(opts: *mut DBTitanDBOptions, disable: bool);
    pub fn ctitandb_options_set_level_merge(opts: *mut DBTitanDBOptions, enable: bool);
    pub fn ctitandb_options_set_range_merge(opts: *mut DBTitanDBOptions, enable: bool);
    pub fn ctitandb_options_set_max_sorted_runs(opts: *mut DBTitanDBOptions, size: i32);
    pub fn ctitandb_options_set_max_background_gc(opts: *mut DBTitanDBOptions, size: i32);
    pub fn ctitandb_options_set_purge_obsolete_files_period_sec(
        opts: *mut DBTitanDBOptions,
        period: usize,
    );
    pub fn ctitandb_options_set_min_gc_batch_size(opts: *mut DBTitanDBOptions, size: u64);
    pub fn ctitandb_options_set_max_gc_batch_size(opts: *mut DBTitanDBOptions, size: u64);
    pub fn ctitandb_options_set_blob_cache(opts: *mut DBTitanDBOptions, cache: *mut DBCache);
    pub fn ctitandb_options_get_blob_cache_usage(options: *const DBTitanDBOptions) -> usize;
    pub fn ctitandb_options_set_blob_cache_capacity(
        options: *const DBTitanDBOptions,
        capacity: usize,
        err: *mut *mut c_char,
    );
    pub fn ctitandb_options_get_blob_cache_capacity(options: *const DBTitanDBOptions) -> usize;

    pub fn ctitandb_options_set_discardable_ratio(opts: *mut DBTitanDBOptions, ratio: f64);
    pub fn ctitandb_options_set_sample_ratio(opts: *mut DBTitanDBOptions, ratio: f64);
    pub fn ctitandb_options_set_merge_small_file_threshold(opts: *mut DBTitanDBOptions, size: u64);
    pub fn ctitandb_options_set_blob_run_mode(opts: *mut DBTitanDBOptions, t: DBTitanDBBlobRunMode);

    pub fn ctitandb_readoptions_set_key_only(opts: *mut DBTitanReadOptions, v: bool);

    pub fn ctitandb_readoptions_create() -> *mut DBTitanReadOptions;
    pub fn ctitandb_readoptions_destroy(readopts: *mut DBTitanReadOptions);

    pub fn ctitandb_create_iterator(
        db: *mut DBInstance,
        readopts: *const DBReadOptions,
        titan_readopts: *const DBTitanReadOptions,
    ) -> *mut DBIterator;
    pub fn ctitandb_create_iterator_cf(
        db: *mut DBInstance,
        readopts: *const DBReadOptions,
        titan_readopts: *const DBTitanReadOptions,
        cf_handle: *mut DBCFHandle,
    ) -> *mut DBIterator;
    pub fn ctitandb_delete_files_in_range(
        db: *mut DBInstance,
        range_start_key: *const u8,
        range_start_key_len: size_t,
        range_limit_key: *const u8,
        range_limit_key_len: size_t,
        include_end: bool,
        err: *mut *mut c_char,
    );
    pub fn ctitandb_delete_files_in_range_cf(
        db: *mut DBInstance,
        cf: *mut DBCFHandle,
        range_start_key: *const u8,
        range_start_key_len: size_t,
        range_limit_key: *const u8,
        range_limit_key_len: size_t,
        include_end: bool,
        err: *mut *mut c_char,
    );
    pub fn ctitandb_delete_files_in_ranges_cf(
        db: *mut DBInstance,
        cf: *mut DBCFHandle,
        start_keys: *const *const u8,
        start_keys_lens: *const size_t,
        limit_keys: *const *const u8,
        limit_keys_lens: *const size_t,
        num_ranges: size_t,
        include_end: bool,
        errptr: *mut *mut c_char,
    );
}

#[cfg(test)]
mod test {
    use super::*;
    use libc::{self, c_void};
    use std::ffi::{CStr, CString};
    use std::{fs, ptr, slice};

    fn tempdir_with_prefix(prefix: &str) -> tempfile::TempDir {
        tempfile::Builder::new().prefix(prefix).tempdir().expect()
    }

    #[test]
    fn internal() {
        unsafe {
            let opts = crocksdb_options_create();
            assert!(!opts.is_null());

            crocksdb_options_increase_parallelism(opts, 0);
            crocksdb_options_optimize_level_style_compaction(opts, 0);
            crocksdb_options_set_create_if_missing(opts, true);
            let rustpath = tempdir_with_prefix("_rust_rocksdb_internaltest");
            let cpath = CString::new(rustpath.path().to_str().unwrap()).unwrap();
            let cpath_ptr = cpath.as_ptr();

            let mut err = ptr::null_mut();
            let db = crocksdb_open(opts, cpath_ptr, &mut err);
            assert!(err.is_null(), error_message(err));

            let writeopts = crocksdb_writeoptions_create();
            assert!(!writeopts.is_null());

            let key = b"name\x00";
            let val = b"spacejam\x00";
            crocksdb_put(db, writeopts, key.as_ptr(), 4, val.as_ptr(), 8, &mut err);
            crocksdb_writeoptions_destroy(writeopts);
            assert!(err.is_null(), error_message(err));

            let readopts = crocksdb_readoptions_create();
            assert!(!readopts.is_null());

            let mut val_len = 0;
            crocksdb_get(db, readopts, key.as_ptr(), 4, &mut val_len, &mut err);
            crocksdb_readoptions_destroy(readopts);
            assert!(err.is_null(), error_message(err));

            // flush first to get approximate size later.
            let flush_opt = crocksdb_flushoptions_create();
            crocksdb_flushoptions_set_wait(flush_opt, true);
            crocksdb_flush(db, flush_opt, &mut err);
            crocksdb_flushoptions_destroy(flush_opt);
            assert!(err.is_null(), error_message(err));

            let mut sizes = vec![0; 1];
            crocksdb_approximate_sizes(
                db,
                1,
                vec![b"\x00\x00".as_ptr()].as_ptr(),
                vec![1].as_ptr(),
                vec![b"\xff\x00".as_ptr()].as_ptr(),
                vec![1].as_ptr(),
                sizes.as_mut_ptr(),
            );
            assert_eq!(sizes.len(), 1);
            assert!(sizes[0] > 0);

            crocksdb_delete_files_in_range(
                db,
                b"\x00\x00".as_ptr(),
                2,
                b"\xff\x00".as_ptr(),
                2,
                true,
                &mut err,
            );
            assert!(err.is_null(), error_message(err));

            let propname = CString::new("rocksdb.total-sst-files-size").unwrap();
            let value = crocksdb_property_value(db, propname.as_ptr());
            assert!(!value.is_null());

            let sst_size = CStr::from_ptr(value)
                .to_str()
                .unwrap()
                .parse::<u64>()
                .unwrap();
            assert!(sst_size > 0);
            libc::free(value as *mut c_void);

            let propname = CString::new("fake_key").unwrap();
            let value = crocksdb_property_value(db, propname.as_ptr());
            assert!(value.is_null());
            libc::free(value as *mut c_void);

            crocksdb_close(db);
            crocksdb_destroy_db(opts, cpath_ptr, &mut err);
            assert!(err.is_null());
        }
    }

    unsafe fn check_get(
        db: *mut DBInstance,
        opt: *const DBReadOptions,
        key: &[u8],
        val: Option<&[u8]>,
    ) {
        let mut val_len = 0;
        let mut err = ptr::null_mut();
        let res_ptr = crocksdb_get(db, opt, key.as_ptr(), key.len(), &mut val_len, &mut err);
        assert!(err.is_null());
        let res = if res_ptr.is_null() {
            None
        } else {
            Some(slice::from_raw_parts(res_ptr, val_len))
        };
        assert_eq!(res, val);
        if !res_ptr.is_null() {
            libc::free(res_ptr as *mut libc::c_void);
        }
    }

    #[test]
    fn test_ingest_external_file() {
        unsafe {
            let opts = crocksdb_options_create();
            crocksdb_options_set_create_if_missing(opts, true);

            let rustpath = tempdir_with_prefix("_rust_rocksdb_internaltest");
            let cpath = CString::new(rustpath.path().to_str().unwrap()).unwrap();
            let cpath_ptr = cpath.as_ptr();

            let mut err = ptr::null_mut();
            let db = crocksdb_open(opts, cpath_ptr, &mut err);
            assert!(err.is_null(), error_message(err));

            let env_opt = crocksdb_envoptions_create();
            let io_options = crocksdb_options_create();
            let writer = crocksdb_sstfilewriter_create(env_opt, io_options);

            let sst_dir = tempdir_with_prefix("_rust_rocksdb_internaltest");
            let sst_path = sst_dir.path().join("sstfilename");
            let c_sst_path = CString::new(sst_path.to_str().unwrap()).unwrap();
            let c_sst_path_ptr = c_sst_path.as_ptr();

            crocksdb_sstfilewriter_open(writer, c_sst_path_ptr, &mut err);
            assert!(err.is_null(), error_message(err));
            crocksdb_sstfilewriter_put(writer, b"sstk1".as_ptr(), 5, b"v1".as_ptr(), 2, &mut err);
            assert!(err.is_null(), error_message(err));
            crocksdb_sstfilewriter_put(writer, b"sstk2".as_ptr(), 5, b"v2".as_ptr(), 2, &mut err);
            assert!(err.is_null(), error_message(err));
            crocksdb_sstfilewriter_put(writer, b"sstk3".as_ptr(), 5, b"v3".as_ptr(), 2, &mut err);
            assert!(err.is_null(), error_message(err));
            crocksdb_sstfilewriter_finish(writer, ptr::null_mut(), &mut err);
            assert!(err.is_null(), error_message(err));
            assert!(crocksdb_sstfilewriter_file_size(writer) > 0);

            let ing_opt = crocksdb_ingestexternalfileoptions_create();
            let file_list = &[c_sst_path_ptr];
            crocksdb_ingest_external_file(db, file_list.as_ptr(), 1, ing_opt, &mut err);
            assert!(err.is_null(), error_message(err));
            let roptions = crocksdb_readoptions_create();
            check_get(db, roptions, b"sstk1", Some(b"v1"));
            check_get(db, roptions, b"sstk2", Some(b"v2"));
            check_get(db, roptions, b"sstk3", Some(b"v3"));

            let snap = crocksdb_create_snapshot(db);

            fs::remove_file(sst_path).unwrap();
            crocksdb_sstfilewriter_open(writer, c_sst_path_ptr, &mut err);
            assert!(err.is_null(), error_message(err));
            crocksdb_sstfilewriter_put(writer, "sstk2".as_ptr(), 5, "v4".as_ptr(), 2, &mut err);
            assert!(err.is_null(), error_message(err));
            crocksdb_sstfilewriter_put(writer, "sstk22".as_ptr(), 6, "v5".as_ptr(), 2, &mut err);
            assert!(err.is_null(), error_message(err));
            crocksdb_sstfilewriter_put(writer, "sstk3".as_ptr(), 5, "v6".as_ptr(), 2, &mut err);
            assert!(err.is_null(), error_message(err));
            crocksdb_sstfilewriter_finish(writer, ptr::null_mut(), &mut err);
            assert!(err.is_null(), error_message(err));
            assert!(crocksdb_sstfilewriter_file_size(writer) > 0);

            crocksdb_ingest_external_file(db, file_list.as_ptr(), 1, ing_opt, &mut err);
            assert!(err.is_null(), error_message(err));
            check_get(db, roptions, b"sstk1", Some(b"v1"));
            check_get(db, roptions, b"sstk2", Some(b"v4"));
            check_get(db, roptions, b"sstk22", Some(b"v5"));
            check_get(db, roptions, b"sstk3", Some(b"v6"));

            let roptions2 = crocksdb_readoptions_create();
            crocksdb_readoptions_set_snapshot(roptions2, snap);
            check_get(db, roptions2, b"sstk1", Some(b"v1"));
            check_get(db, roptions2, b"sstk2", Some(b"v2"));
            check_get(db, roptions2, b"sstk22", None);
            check_get(db, roptions2, b"sstk3", Some(b"v3"));
            crocksdb_readoptions_destroy(roptions2);

            crocksdb_readoptions_destroy(roptions);
            crocksdb_release_snapshot(db, snap);
            crocksdb_ingestexternalfileoptions_destroy(ing_opt);
            crocksdb_sstfilewriter_destroy(writer);
            crocksdb_options_destroy(io_options);
            crocksdb_envoptions_destroy(env_opt);
        }
    }
}
