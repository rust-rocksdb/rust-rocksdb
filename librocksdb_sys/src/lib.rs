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
#[cfg(test)]
extern crate tempdir;

use libc::{c_char, c_uchar, c_int, c_void, size_t, uint64_t, c_double};
use std::ffi::CStr;

pub enum DBOptions {}
pub enum DBInstance {}
pub enum DBWriteOptions {}
pub enum DBReadOptions {}
pub enum DBMergeOperator {}
pub enum DBBlockBasedTableOptions {}
pub enum DBCache {}
pub enum DBFilterPolicy {}
pub enum DBSnapshot {}
pub enum DBIterator {}
pub enum DBCFHandle {}
pub enum DBWriteBatch {}
pub enum DBComparator {}
pub enum DBFlushOptions {}
pub enum DBCompactionFilter {}
pub enum EnvOptions {}
pub enum SstFileWriter {}
pub enum IngestExternalFileOptions {}
pub enum DBBackupEngine {}
pub enum DBRestoreOptions {}
pub enum DBSliceTransform {}

pub fn new_bloom_filter(bits: c_int) -> *mut DBFilterPolicy {
    unsafe { crocksdb_filterpolicy_create_bloom(bits) }
}

pub fn new_cache(capacity: size_t) -> *mut DBCache {
    unsafe { crocksdb_cache_create_lru(capacity) }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub enum DBCompressionType {
    DBNo = 0,
    DBSnappy = 1,
    DBZlib = 2,
    DBBz2 = 3,
    DBLz4 = 4,
    DBLz4hc = 5,
}

#[repr(C)]
pub enum DBCompactionStyle {
    DBLevel = 0,
    DBUniversal = 1,
    DBFifo = 2,
}

#[repr(C)]
pub enum DBUniversalCompactionStyle {
    crocksdb_similar_size_compaction_stop_style = 0,
    crocksdb_total_size_compaction_stop_style = 1,
}

#[derive(Copy, Clone, PartialEq)]
#[repr(C)]
pub enum DBRecoveryMode {
    TolerateCorruptedTailRecords = 0,
    AbsoluteConsistency = 1,
    PointInTime = 2,
    SkipAnyCorruptedRecords = 3,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub enum DBStatisticsTickerType {
    BlockCacheMiss = 0, // total block cache miss
    BlockCacheHit = 1, // total block cache hit
    BlockCacheIndexMiss = 4, // times cache miss when accessing index block from block cache
    BlockCacheIndexHit = 5,
    BlockCacheFilterMiss = 9, // times cache miss when accessing filter block from block cache
    BlockCacheFilterHit = 10,
    BloomFilterUseful = 20, // times bloom filter has avoided file reads
    MemtableHit = 25,
    MemtableMiss = 26,
    GetHitL0 = 27, // Get() queries served by L0
    GetHitL1 = 28, // Get() queries served by L1
    GetHitL2AndUp = 29, // Get() queries served by L2 and up
    NumberKeysWritten = 35, // number of keys written to the database via the Put and Write call's
    NumberKeysRead = 36, // number of keys read
    BytesWritten = 38, // the number of uncompressed bytes read from DB::Put, DB::Delete,
    // DB::Merge and DB::Write
    BytesRead = 39, // the number of uncompressed bytes read from DB::Get()
    NumberDbSeek = 40, // the number of calls to seek/next/prev
    NumberDbNext = 41,
    NumberDbPrev = 42,
    NumberDbSeekFound = 43, // the number of calls to seek/next/prev that returned data
    NumberDbNextFound = 44,
    NumberDbPrevFound = 45,
    IterBytesRead = 46, // the number of uncompressed bytes read from an iterator, include size of
    // key and value
    StallMicros = 53, // writer has to wait for compaction or flush to finish
    NoIterators = 56, // number of iterators currently open
    BloomFilterPrefixChecked = 62, // number of times bloom was checked before creating iterator
    // on a file
    BloomFilterPrefixUseful = 63, // number of times the check was useful in avoiding iterator
    // creating
    WalFileSynced = 70, // number of times WAL sync is done
    WalFileBytes = 71, // number of bytes written to WAL
    CompactReadBytes = 76, // bytes read during compaction
    CompactWriteBytes = 77, // bytes written during compaction
    FlushWriteBytes = 78, // bytes written during flush
}

#[derive(Copy, Clone)]
#[repr(C)]
pub enum DBStatisticsHistogramType {
    DbGetMicros = 0,
    DbWriteMicros = 1,
    DbSeekMicros = 19,
}

pub fn error_message(ptr: *mut c_char) -> String {
    let c_str = unsafe { CStr::from_ptr(ptr) };
    let s = format!("{}", c_str.to_string_lossy());
    unsafe {
        libc::free(ptr as *mut c_void);
    }
    s
}

#[macro_export]
macro_rules! ffi_try {
    ($func:ident($($arg:expr),*)) => ({
        use std::ptr;
        let mut err = ptr::null_mut();
        let res = $crate::$func($($arg),*, &mut err);
        if !err.is_null() {
            return Err($crate::error_message(err));
        }
        res
    })
}

// TODO audit the use of boolean arguments, b/c I think they need to be u8
// instead...
extern "C" {
    pub fn crocksdb_options_create() -> *mut DBOptions;
    pub fn crocksdb_options_destroy(opts: *mut DBOptions);
    pub fn crocksdb_cache_create_lru(capacity: size_t) -> *mut DBCache;
    pub fn crocksdb_cache_destroy(cache: *mut DBCache);
    pub fn crocksdb_block_based_options_create() -> *mut DBBlockBasedTableOptions;
    pub fn crocksdb_block_based_options_destroy(opts: *mut DBBlockBasedTableOptions);
    pub fn crocksdb_block_based_options_set_block_size(
        block_options: *mut DBBlockBasedTableOptions,
        block_size: size_t);
    pub fn crocksdb_block_based_options_set_block_size_deviation(
        block_options: *mut DBBlockBasedTableOptions,
        block_size_deviation: c_int);
    pub fn crocksdb_block_based_options_set_block_restart_interval(
        block_options: *mut DBBlockBasedTableOptions,
        block_restart_interval: c_int);
    pub fn crocksdb_block_based_options_set_cache_index_and_filter_blocks(
        block_options: *mut DBBlockBasedTableOptions, v: c_uchar);
    pub fn crocksdb_block_based_options_set_filter_policy(
        block_options: *mut DBBlockBasedTableOptions,
        filter_policy: *mut DBFilterPolicy);
    pub fn crocksdb_block_based_options_set_no_block_cache(
        block_options: *mut DBBlockBasedTableOptions, no_block_cache: bool);
    pub fn crocksdb_block_based_options_set_block_cache(
        block_options: *mut DBBlockBasedTableOptions, block_cache: *mut DBCache);
    pub fn crocksdb_block_based_options_set_block_cache_compressed(
        block_options: *mut DBBlockBasedTableOptions,
        block_cache_compressed: *mut DBCache);
    pub fn crocksdb_block_based_options_set_whole_key_filtering(
        ck_options: *mut DBBlockBasedTableOptions, doit: bool);
    pub fn crocksdb_options_set_block_based_table_factory(
        options: *mut DBOptions,
        block_options: *mut DBBlockBasedTableOptions);
    pub fn crocksdb_options_increase_parallelism(options: *mut DBOptions, threads: c_int);
    pub fn crocksdb_options_optimize_level_style_compaction(options: *mut DBOptions,
                                                            memtable_memory_budget: c_int);
    pub fn crocksdb_options_set_compaction_filter(options: *mut DBOptions,
                                                  filter: *mut DBCompactionFilter);
    pub fn crocksdb_options_set_create_if_missing(options: *mut DBOptions, v: bool);
    pub fn crocksdb_options_set_max_open_files(options: *mut DBOptions, files: c_int);
    pub fn crocksdb_options_set_max_total_wal_size(options: *mut DBOptions, size: u64);
    pub fn crocksdb_options_set_use_fsync(options: *mut DBOptions, v: c_int);
    pub fn crocksdb_options_set_bytes_per_sync(options: *mut DBOptions, bytes: u64);
    pub fn crocksdb_options_set_disable_data_sync(options: *mut DBOptions, v: c_int);
    pub fn crocksdb_options_optimize_for_point_lookup(options: *mut DBOptions,
                                                      block_cache_size_mb: u64);
    pub fn crocksdb_options_set_table_cache_numshardbits(options: *mut DBOptions, bits: c_int);
    pub fn crocksdb_options_set_max_write_buffer_number(options: *mut DBOptions, bufno: c_int);
    pub fn crocksdb_options_set_min_write_buffer_number_to_merge(options: *mut DBOptions,
                                                                 bufno: c_int);
    pub fn crocksdb_options_set_level0_file_num_compaction_trigger(options: *mut DBOptions,
                                                                   no: c_int);
    pub fn crocksdb_options_set_level0_slowdown_writes_trigger(options: *mut DBOptions,
                                                               no: c_int);
    pub fn crocksdb_options_set_level0_stop_writes_trigger(options: *mut DBOptions, no: c_int);
    pub fn crocksdb_options_set_write_buffer_size(options: *mut DBOptions, bytes: u64);
    pub fn crocksdb_options_set_target_file_size_base(options: *mut DBOptions, bytes: u64);
    pub fn crocksdb_options_set_target_file_size_multiplier(options: *mut DBOptions, mul: c_int);
    pub fn crocksdb_options_set_max_bytes_for_level_base(options: *mut DBOptions, bytes: u64);
    pub fn crocksdb_options_set_max_bytes_for_level_multiplier(options: *mut DBOptions,
                                                               mul: c_int);
    pub fn crocksdb_options_set_max_log_file_size(options: *mut DBOptions, bytes: size_t);
    pub fn crocksdb_options_set_keep_log_file_num(options: *mut DBOptions, num: size_t);
    pub fn crocksdb_options_set_max_manifest_file_size(options: *mut DBOptions, bytes: u64);
    pub fn crocksdb_options_set_hash_skip_list_rep(options: *mut DBOptions,
                                                   bytes: u64,
                                                   a1: i32,
                                                   a2: i32);
    pub fn crocksdb_options_set_compaction_style(options: *mut DBOptions, cs: DBCompactionStyle);
    pub fn crocksdb_options_set_compression(options: *mut DBOptions,
                                            compression_style_no: DBCompressionType);
    pub fn crocksdb_options_set_compression_per_level(options: *mut DBOptions,
                                                      level_values: *const DBCompressionType,
                                                      num_levels: size_t);
    pub fn crocksdb_options_set_max_background_compactions(options: *mut DBOptions,
                                                           max_bg_compactions: c_int);
    pub fn crocksdb_options_set_max_background_flushes(options: *mut DBOptions,
                                                       max_bg_flushes: c_int);
    pub fn crocksdb_options_set_disable_auto_compactions(options: *mut DBOptions, v: c_int);
    pub fn crocksdb_options_set_report_bg_io_stats(options: *mut DBOptions, v: c_int);
    pub fn crocksdb_options_set_compaction_readahead_size(options: *mut DBOptions, v: size_t);
    pub fn crocksdb_options_set_wal_recovery_mode(options: *mut DBOptions, mode: DBRecoveryMode);
    pub fn crocksdb_options_enable_statistics(options: *mut DBOptions);
    pub fn crocksdb_options_statistics_get_string(options: *mut DBOptions) -> *const c_char;
    pub fn crocksdb_options_statistics_get_ticker_count(options: *mut DBOptions,
                                                        ticker_type: DBStatisticsTickerType)
                                                        -> u64;
    pub fn crocksdb_options_statistics_get_and_reset_ticker_count(options: *mut DBOptions,
                                                        ticker_type: DBStatisticsTickerType)
                                                        -> u64;
    pub fn crocksdb_options_statistics_get_histogram_string(options: *mut DBOptions,
                                                            hist_type: DBStatisticsHistogramType)
                                                            -> *const c_char;
    pub fn crocksdb_options_statistics_get_histogram(options: *mut DBOptions,
                                                     hist_type: DBStatisticsHistogramType,
                                                     median: *mut c_double,
                                                     percentile95: *mut c_double,
                                                     percentile99: *mut c_double,
                                                     average: *mut c_double,
                                                     standard_deviation: *mut c_double)
                                                     -> bool;
    pub fn crocksdb_options_set_stats_dump_period_sec(options: *mut DBOptions, v: usize);
    pub fn crocksdb_options_set_num_levels(options: *mut DBOptions, v: c_int);
    pub fn crocksdb_options_set_db_log_dir(options: *mut DBOptions, path: *const c_char);
    pub fn crocksdb_options_set_prefix_extractor(options: *mut DBOptions,
                                                 prefix_extractor: *mut DBSliceTransform);
    pub fn crocksdb_options_set_memtable_insert_with_hint_prefix_extractor(options: *mut DBOptions,
                                                 prefix_extractor: *mut DBSliceTransform);
    pub fn crocksdb_options_set_memtable_prefix_bloom_size_ratio(options: *mut DBOptions,
                                                                 ratio: c_double);
    pub fn crocksdb_filterpolicy_create_bloom_full(bits_per_key: c_int) -> *mut DBFilterPolicy;
    pub fn crocksdb_filterpolicy_create_bloom(bits_per_key: c_int) -> *mut DBFilterPolicy;
    pub fn crocksdb_open(options: *mut DBOptions,
                         path: *const c_char,
                         err: *mut *mut c_char)
                         -> *mut DBInstance;
    pub fn crocksdb_writeoptions_create() -> *mut DBWriteOptions;
    pub fn crocksdb_writeoptions_destroy(writeopts: *mut DBWriteOptions);
    pub fn crocksdb_writeoptions_set_sync(writeopts: *mut DBWriteOptions, v: bool);
    pub fn crocksdb_writeoptions_disable_WAL(writeopts: *mut DBWriteOptions, v: c_int);
    pub fn crocksdb_put(db: *mut DBInstance,
                        writeopts: *mut DBWriteOptions,
                        k: *const u8,
                        kLen: size_t,
                        v: *const u8,
                        vLen: size_t,
                        err: *mut *mut c_char);
    pub fn crocksdb_put_cf(db: *mut DBInstance,
                           writeopts: *mut DBWriteOptions,
                           cf: *mut DBCFHandle,
                           k: *const u8,
                           kLen: size_t,
                           v: *const u8,
                           vLen: size_t,
                           err: *mut *mut c_char);
    pub fn crocksdb_readoptions_create() -> *mut DBReadOptions;
    pub fn crocksdb_readoptions_destroy(readopts: *mut DBReadOptions);
    pub fn crocksdb_readoptions_set_verify_checksums(readopts: *mut DBReadOptions, v: bool);
    pub fn crocksdb_readoptions_set_fill_cache(readopts: *mut DBReadOptions, v: bool);
    pub fn crocksdb_readoptions_set_snapshot(readopts: *mut DBReadOptions,
                                             snapshot: *const DBSnapshot);
    pub fn crocksdb_readoptions_set_iterate_upper_bound(readopts: *mut DBReadOptions,
                                                        k: *const u8,
                                                        kLen: size_t);
    pub fn crocksdb_readoptions_set_read_tier(readopts: *mut DBReadOptions, tier: c_int);
    pub fn crocksdb_readoptions_set_tailing(readopts: *mut DBReadOptions, v: bool);
    pub fn crocksdb_readoptions_set_total_order_seek(readopts: *mut DBReadOptions, v: bool);

    pub fn crocksdb_get(db: *const DBInstance,
                        readopts: *const DBReadOptions,
                        k: *const u8,
                        kLen: size_t,
                        valLen: *const size_t,
                        err: *mut *mut c_char)
                        -> *mut u8;
    pub fn crocksdb_get_cf(db: *const DBInstance,
                           readopts: *const DBReadOptions,
                           cf_handle: *mut DBCFHandle,
                           k: *const u8,
                           kLen: size_t,
                           valLen: *const size_t,
                           err: *mut *mut c_char)
                           -> *mut u8;
    pub fn crocksdb_create_iterator(db: *mut DBInstance,
                                    readopts: *const DBReadOptions)
                                    -> *mut DBIterator;
    pub fn crocksdb_create_iterator_cf(db: *mut DBInstance,
                                       readopts: *const DBReadOptions,
                                       cf_handle: *mut DBCFHandle)
                                       -> *mut DBIterator;
    pub fn crocksdb_create_snapshot(db: *mut DBInstance) -> *const DBSnapshot;
    pub fn crocksdb_release_snapshot(db: *mut DBInstance, snapshot: *const DBSnapshot);

    pub fn crocksdb_delete(db: *mut DBInstance,
                           writeopts: *const DBWriteOptions,
                           k: *const u8,
                           kLen: size_t,
                           err: *mut *mut c_char);
    pub fn crocksdb_delete_cf(db: *mut DBInstance,
                              writeopts: *const DBWriteOptions,
                              cf: *mut DBCFHandle,
                              k: *const u8,
                              kLen: size_t,
                              err: *mut *mut c_char);
    pub fn crocksdb_single_delete(db: *mut DBInstance,
                                  writeopts: *const DBWriteOptions,
                                  k: *const u8,
                                  kLen: size_t,
                                  err: *mut *mut c_char);
    pub fn crocksdb_single_delete_cf(db: *mut DBInstance,
                                     writeopts: *const DBWriteOptions,
                                     cf: *mut DBCFHandle,
                                     k: *const u8,
                                     kLen: size_t,
                                     err: *mut *mut c_char);
    pub fn crocksdb_close(db: *mut DBInstance);
    pub fn crocksdb_pause_bg_work(db: *mut DBInstance);
    pub fn crocksdb_continue_bg_work(db: *mut DBInstance);
    pub fn crocksdb_destroy_db(options: *const DBOptions,
                               path: *const c_char,
                               err: *mut *mut c_char);
    pub fn crocksdb_repair_db(options: *const DBOptions,
                              path: *const c_char,
                              err: *mut *mut c_char);
    // Merge
    pub fn crocksdb_merge(db: *mut DBInstance,
                          writeopts: *const DBWriteOptions,
                          k: *const u8,
                          kLen: size_t,
                          v: *const u8,
                          vLen: size_t,
                          err: *mut *mut c_char);
    pub fn crocksdb_merge_cf(db: *mut DBInstance,
                             writeopts: *const DBWriteOptions,
                             cf: *mut DBCFHandle,
                             k: *const u8,
                             kLen: size_t,
                             v: *const u8,
                             vLen: size_t,
                             err: *mut *mut c_char);
    pub fn crocksdb_mergeoperator_create(
        state: *mut c_void,
        destroy: extern fn(*mut c_void) -> (),
        full_merge: extern fn (arg: *mut c_void,
                               key: *const c_char, key_len: size_t,
                               existing_value: *const c_char,
                               existing_value_len: size_t,
                               operands_list: *const *const c_char,
                               operands_list_len: *const size_t,
                               num_operands: c_int, success: *mut u8,
                               new_value_length: *mut size_t
                               ) -> *const c_char,
        partial_merge: extern fn(arg: *mut c_void,
                                 key: *const c_char, key_len: size_t,
                                 operands_list: *const *const c_char,
                                 operands_list_len: *const size_t,
                                 num_operands: c_int, success: *mut u8,
                                 new_value_length: *mut size_t
                                 ) -> *const c_char,
        delete_value: Option<extern "C" fn(*mut c_void,
                                           value: *const c_char,
                                           value_len: *mut size_t
                                           ) -> ()>,
        name_fn: extern fn(*mut c_void) -> *const c_char,
    ) -> *mut DBMergeOperator;
    pub fn crocksdb_mergeoperator_destroy(mo: *mut DBMergeOperator);
    pub fn crocksdb_options_set_merge_operator(options: *mut DBOptions, mo: *mut DBMergeOperator);
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
    pub fn crocksdb_write(db: *mut DBInstance,
                          writeopts: *const DBWriteOptions,
                          batch: *mut DBWriteBatch,
                          err: *mut *mut c_char);
    pub fn crocksdb_writebatch_create() -> *mut DBWriteBatch;
    pub fn crocksdb_writebatch_create_from(rep: *const u8, size: size_t) -> *mut DBWriteBatch;
    pub fn crocksdb_writebatch_destroy(batch: *mut DBWriteBatch);
    pub fn crocksdb_writebatch_clear(batch: *mut DBWriteBatch);
    pub fn crocksdb_writebatch_count(batch: *mut DBWriteBatch) -> c_int;
    pub fn crocksdb_writebatch_put(batch: *mut DBWriteBatch,
                                   key: *const u8,
                                   klen: size_t,
                                   val: *const u8,
                                   vlen: size_t);
    pub fn crocksdb_writebatch_put_cf(batch: *mut DBWriteBatch,
                                      cf: *mut DBCFHandle,
                                      key: *const u8,
                                      klen: size_t,
                                      val: *const u8,
                                      vlen: size_t);
    pub fn crocksdb_writebatch_merge(batch: *mut DBWriteBatch,
                                     key: *const u8,
                                     klen: size_t,
                                     val: *const u8,
                                     vlen: size_t);
    pub fn crocksdb_writebatch_merge_cf(batch: *mut DBWriteBatch,
                                        cf: *mut DBCFHandle,
                                        key: *const u8,
                                        klen: size_t,
                                        val: *const u8,
                                        vlen: size_t);
    pub fn crocksdb_writebatch_delete(batch: *mut DBWriteBatch, key: *const u8, klen: size_t);
    pub fn crocksdb_writebatch_delete_cf(batch: *mut DBWriteBatch,
                                         cf: *mut DBCFHandle,
                                         key: *const u8,
                                         klen: size_t);
    pub fn crocksdb_writebatch_single_delete(batch: *mut DBWriteBatch,
                                             key: *const u8,
                                             klen: size_t);
    pub fn crocksdb_writebatch_single_delete_cf(batch: *mut DBWriteBatch,
                                                cf: *mut DBCFHandle,
                                                key: *const u8,
                                                klen: size_t);
    pub fn crocksdb_writebatch_iterate(batch: *mut DBWriteBatch,
                                       state: *mut c_void,
                                       put_fn: extern "C" fn(state: *mut c_void,
                                                             k: *const u8,
                                                             klen: size_t,
                                                             v: *const u8,
                                                             vlen: size_t),
                                       deleted_fn: extern "C" fn(state: *mut c_void,
                                                                 k: *const u8,
                                                                 klen: size_t));
    pub fn crocksdb_writebatch_data(batch: *mut DBWriteBatch, size: *mut size_t) -> *const u8;

    // Comparator
    pub fn crocksdb_options_set_comparator(options: *mut DBOptions, cb: *mut DBComparator);
    pub fn crocksdb_comparator_create(state: *mut c_void,
                                      destroy: extern "C" fn(*mut c_void) -> (),
                                      compare: extern "C" fn(arg: *mut c_void,
                                                             a: *const c_char,
                                                             alen: size_t,
                                                             b: *const c_char,
                                                             blen: size_t)
                                                             -> c_int,
                                      name_fn: extern "C" fn(*mut c_void) -> *const c_char)
                                      -> *mut DBComparator;
    pub fn crocksdb_comparator_destroy(cmp: *mut DBComparator);

    // Column Family
    pub fn crocksdb_open_column_families(options: *const DBOptions,
                                         path: *const c_char,
                                         num_column_families: c_int,
                                         column_family_names: *const *const c_char,
                                         column_family_options: *const *const DBOptions,
                                         column_family_handles: *const *mut DBCFHandle,
                                         err: *mut *mut c_char)
                                         -> *mut DBInstance;
    pub fn crocksdb_create_column_family(db: *mut DBInstance,
                                         column_family_options: *const DBOptions,
                                         column_family_name: *const c_char,
                                         err: *mut *mut c_char)
                                         -> *mut DBCFHandle;
    pub fn crocksdb_drop_column_family(db: *mut DBInstance,
                                       column_family_handle: *mut DBCFHandle,
                                       err: *mut *mut c_char);
    pub fn crocksdb_column_family_handle_destroy(column_family_handle: *mut DBCFHandle);
    pub fn crocksdb_list_column_families(db: *const DBOptions,
                                         path: *const c_char,
                                         lencf: *mut size_t,
                                         err: *mut *mut c_char)
                                         -> *mut *mut c_char;
    pub fn crocksdb_list_column_families_destroy(list: *mut *mut c_char, len: size_t);

    // Flush options
    pub fn crocksdb_flushoptions_create() -> *mut DBFlushOptions;
    pub fn crocksdb_flushoptions_destroy(opt: *mut DBFlushOptions);
    pub fn crocksdb_flushoptions_set_wait(opt: *mut DBFlushOptions, whether_wait: bool);

    pub fn crocksdb_flush(db: *mut DBInstance,
                          options: *const DBFlushOptions,
                          err: *mut *mut c_char);

    pub fn crocksdb_approximate_sizes(db: *mut DBInstance,
                                      num_ranges: c_int,
                                      range_start_key: *const *const u8,
                                      range_start_key_len: *const size_t,
                                      range_limit_key: *const *const u8,
                                      range_limit_key_len: *const size_t,
                                      sizes: *mut uint64_t);
    pub fn crocksdb_approximate_sizes_cf(db: *mut DBInstance,
                                         cf: *mut DBCFHandle,
                                         num_ranges: c_int,
                                         range_start_key: *const *const u8,
                                         range_start_key_len: *const size_t,
                                         range_limit_key: *const *const u8,
                                         range_limit_key_len: *const size_t,
                                         sizes: *mut uint64_t);
    pub fn crocksdb_compact_range(db: *mut DBInstance,
                                  start_key: *const u8,
                                  start_key_len: size_t,
                                  limit_key: *const u8,
                                  limit_key_len: size_t);
    pub fn crocksdb_compact_range_cf(db: *mut DBInstance,
                                     cf: *mut DBCFHandle,
                                     start_key: *const u8,
                                     start_key_len: size_t,
                                     limit_key: *const u8,
                                     limit_key_len: size_t);
    pub fn crocksdb_delete_file_in_range(db: *mut DBInstance,
                                         range_start_key: *const u8,
                                         range_start_key_len: size_t,
                                         range_limit_key: *const u8,
                                         range_limit_key_len: size_t,
                                         err: *mut *mut c_char);
    pub fn crocksdb_delete_file_in_range_cf(db: *mut DBInstance,
                                            cf: *mut DBCFHandle,
                                            range_start_key: *const u8,
                                            range_start_key_len: size_t,
                                            range_limit_key: *const u8,
                                            range_limit_key_len: size_t,
                                            err: *mut *mut c_char);
    pub fn crocksdb_property_value(db: *mut DBInstance, propname: *const c_char) -> *mut c_char;
    pub fn crocksdb_property_value_cf(db: *mut DBInstance,
                                      cf: *mut DBCFHandle,
                                      propname: *const c_char)
                                      -> *mut c_char;
    // Compaction filter
    pub fn crocksdb_compactionfilter_create(state: *mut c_void,
                                            destructor: extern "C" fn(*mut c_void),
                                            filter: extern "C" fn(*mut c_void,
                                                                  c_int,
                                                                  *const u8,
                                                                  size_t,
                                                                  *const u8,
                                                                  size_t,
                                                                  *mut *mut u8,
                                                                  *mut size_t,
                                                                  *mut bool)
                                                                  -> bool,
                                            name: extern "C" fn(*mut c_void) -> *const c_char)
                                            -> *mut DBCompactionFilter;
    pub fn crocksdb_compactionfilter_set_ignore_snapshots(filter: *mut DBCompactionFilter,
                                                          ignore_snapshot: bool);
    pub fn crocksdb_compactionfilter_destroy(filter: *mut DBCompactionFilter);

    // EnvOptions
    pub fn crocksdb_envoptions_create() -> *mut EnvOptions;
    pub fn crocksdb_envoptions_destroy(opt: *mut EnvOptions);

    // IngestExternalFileOptions
    pub fn crocksdb_ingestexternalfileoptions_create() -> *mut IngestExternalFileOptions;
    pub fn crocksdb_ingestexternalfileoptions_set_move_files(opt: *mut IngestExternalFileOptions,
                                                             move_files: bool);
    pub fn crocksdb_ingestexternalfileoptions_set_snapshot_consistency(
        opt: *mut IngestExternalFileOptions, snapshot_consistency: bool);
    pub fn crocksdb_ingestexternalfileoptions_set_allow_global_seqno(
        opt: *mut IngestExternalFileOptions, allow_global_seqno: bool);
    pub fn crocksdb_ingestexternalfileoptions_set_allow_blocking_flush(
        opt: *mut IngestExternalFileOptions, allow_blocking_flush: bool);
    pub fn crocksdb_ingestexternalfileoptions_destroy(opt: *mut IngestExternalFileOptions);

    // SstFileWriter
    pub fn crocksdb_sstfilewriter_create(env: *mut EnvOptions,
                                         io_options: *const DBOptions)
                                         -> *mut SstFileWriter;
    pub fn crocksdb_sstfilewriter_create_with_comparator(env: *mut EnvOptions,
                                                         io_options: *const DBOptions,
                                                         comparator: *const DBComparator)
                                                         -> *mut SstFileWriter;
    pub fn crocksdb_sstfilewriter_open(writer: *mut SstFileWriter,
                                       name: *const c_char,
                                       err: *mut *mut c_char);
    pub fn crocksdb_sstfilewriter_add(writer: *mut SstFileWriter,
                                      key: *const u8,
                                      key_len: size_t,
                                      val: *const u8,
                                      val_len: size_t,
                                      err: *mut *mut c_char);
    pub fn crocksdb_sstfilewriter_finish(writer: *mut SstFileWriter, err: *mut *mut c_char);
    pub fn crocksdb_sstfilewriter_destroy(writer: *mut SstFileWriter);

    pub fn crocksdb_ingest_external_file(db: *mut DBInstance,
                                         file_list: *const *const c_char,
                                         list_len: size_t,
                                         opt: *const IngestExternalFileOptions,
                                         err: *mut *mut c_char);
    pub fn crocksdb_ingest_external_file_cf(db: *mut DBInstance,
                                            handle: *const DBCFHandle,
                                            file_list: *const *const c_char,
                                            list_len: size_t,
                                            opt: *const IngestExternalFileOptions,
                                            err: *mut *mut c_char);

    // Restore Option
    pub fn crocksdb_restore_options_create() -> *mut DBRestoreOptions;
    pub fn crocksdb_restore_options_destroy(ropts: *mut DBRestoreOptions);
    pub fn crocksdb_restore_options_set_keep_log_files(ropts: *mut DBRestoreOptions, v: c_int);

    // Backup engine
    // TODO: add more ffis about backup engine.
    pub fn crocksdb_backup_engine_open(options: *const DBOptions,
                                       path: *const c_char,
                                       err: *mut *mut c_char)
                                       -> *mut DBBackupEngine;
    pub fn crocksdb_backup_engine_create_new_backup(be: *mut DBBackupEngine,
                                                    db: *mut DBInstance,
                                                    err: *mut *mut c_char);
    pub fn crocksdb_backup_engine_close(be: *mut DBBackupEngine);
    pub fn crocksdb_backup_engine_restore_db_from_latest_backup(be: *mut DBBackupEngine,
                                                                db_path: *const c_char,
                                                                wal_path: *const c_char,
                                                                ropts: *const DBRestoreOptions,
                                                                err: *mut *mut c_char);
    // SliceTransform
    pub fn crocksdb_slicetransform_create(state: *mut c_void,
                                          destructor: extern "C" fn(*mut c_void),
                                          transform: extern "C" fn(*mut c_void,
                                                                   *const u8,
                                                                   size_t,
                                                                   *mut size_t)
                                                                   -> *const u8,
                                          in_domain: extern "C" fn(*mut c_void,
                                                                   *const u8,
                                                                   size_t)
                                                                   -> u8,
                                          in_range: extern "C" fn(*mut c_void, *const u8, size_t)
                                                                  -> u8,
                                          name: extern "C" fn(*mut c_void) -> *const c_char)
                                          -> *mut DBSliceTransform;
    pub fn crocksdb_slicetransform_destroy(transform: *mut DBSliceTransform);
}

#[cfg(test)]
mod test {
    use libc::{self, c_void};
    use std::{ptr, slice, fs};
    use std::ffi::{CStr, CString};
    use super::*;
    use tempdir::TempDir;

    #[test]
    fn internal() {
        unsafe {
            let opts = crocksdb_options_create();
            assert!(!opts.is_null());

            crocksdb_options_increase_parallelism(opts, 0);
            crocksdb_options_optimize_level_style_compaction(opts, 0);
            crocksdb_options_set_create_if_missing(opts, true);

            let rustpath = TempDir::new("_rust_rocksdb_internaltest").expect("");
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
            crocksdb_approximate_sizes(db,
                                       1,
                                       vec![b"\x00\x00".as_ptr()].as_ptr(),
                                       vec![1].as_ptr(),
                                       vec![b"\xff\x00".as_ptr()].as_ptr(),
                                       vec![1].as_ptr(),
                                       sizes.as_mut_ptr());
            assert_eq!(sizes.len(), 1);
            assert!(sizes[0] > 0);

            crocksdb_delete_file_in_range(db,
                                          b"\x00\x00".as_ptr(),
                                          2,
                                          b"\xff\x00".as_ptr(),
                                          2,
                                          &mut err);
            assert!(err.is_null(), error_message(err));

            let propname = CString::new("rocksdb.total-sst-files-size").unwrap();
            let value = crocksdb_property_value(db, propname.as_ptr());
            assert!(!value.is_null());

            let sst_size = CStr::from_ptr(value).to_str().unwrap().parse::<u64>().unwrap();
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

    unsafe fn check_get(db: *mut DBInstance,
                        opt: *const DBReadOptions,
                        key: &[u8],
                        val: Option<&[u8]>) {
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

            let rustpath = TempDir::new("_rust_rocksdb_internaltest").expect("");
            let cpath = CString::new(rustpath.path().to_str().unwrap()).unwrap();
            let cpath_ptr = cpath.as_ptr();

            let mut err = ptr::null_mut();
            let db = crocksdb_open(opts, cpath_ptr, &mut err);
            assert!(err.is_null(), error_message(err));

            let env_opt = crocksdb_envoptions_create();
            let io_options = crocksdb_options_create();
            let writer = crocksdb_sstfilewriter_create(env_opt, io_options);

            let sst_dir = TempDir::new("_rust_rocksdb_internaltest").expect("");
            let sst_path = sst_dir.path().join("sstfilename");
            let c_sst_path = CString::new(sst_path.to_str().unwrap()).unwrap();
            let c_sst_path_ptr = c_sst_path.as_ptr();

            crocksdb_sstfilewriter_open(writer, c_sst_path_ptr, &mut err);
            assert!(err.is_null(), error_message(err));
            crocksdb_sstfilewriter_add(writer, b"sstk1".as_ptr(), 5, b"v1".as_ptr(), 2, &mut err);
            assert!(err.is_null(), error_message(err));
            crocksdb_sstfilewriter_add(writer, b"sstk2".as_ptr(), 5, b"v2".as_ptr(), 2, &mut err);
            assert!(err.is_null(), error_message(err));
            crocksdb_sstfilewriter_add(writer, b"sstk3".as_ptr(), 5, b"v3".as_ptr(), 2, &mut err);
            assert!(err.is_null(), error_message(err));
            crocksdb_sstfilewriter_finish(writer, &mut err);
            assert!(err.is_null(), error_message(err));

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
            crocksdb_sstfilewriter_add(writer, "sstk2".as_ptr(), 5, "v4".as_ptr(), 2, &mut err);
            assert!(err.is_null(), error_message(err));
            crocksdb_sstfilewriter_add(writer, "sstk22".as_ptr(), 6, "v5".as_ptr(), 2, &mut err);
            assert!(err.is_null(), error_message(err));
            crocksdb_sstfilewriter_add(writer, "sstk3".as_ptr(), 5, "v6".as_ptr(), 2, &mut err);
            assert!(err.is_null(), error_message(err));
            crocksdb_sstfilewriter_finish(writer, &mut err);
            assert!(err.is_null(), error_message(err));

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
