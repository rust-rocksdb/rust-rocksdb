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

use libc::{c_char, c_uchar, c_int, c_void, size_t, uint64_t};
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
pub enum DBRateLimiter {}

pub fn new_bloom_filter(bits: c_int) -> *mut DBFilterPolicy {
    unsafe { rocksdb_filterpolicy_create_bloom(bits) }
}

pub fn new_cache(capacity: size_t) -> *mut DBCache {
    unsafe { rocksdb_cache_create_lru(capacity) }
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
    rocksdb_similar_size_compaction_stop_style = 0,
    rocksdb_total_size_compaction_stop_style = 1,
}

#[derive(Copy, Clone, PartialEq)]
#[repr(C)]
pub enum DBRecoveryMode {
    TolerateCorruptedTailRecords = 0,
    AbsoluteConsistency = 1,
    PointInTime = 2,
    SkipAnyCorruptedRecords = 3,
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
#[link(name = "rocksdb")]
extern "C" {
    pub fn rocksdb_options_create() -> *mut DBOptions;
    pub fn rocksdb_options_destroy(opts: *mut DBOptions);
    pub fn rocksdb_cache_create_lru(capacity: size_t) -> *mut DBCache;
    pub fn rocksdb_cache_destroy(cache: *mut DBCache);
    pub fn rocksdb_block_based_options_create() -> *mut DBBlockBasedTableOptions;
    pub fn rocksdb_block_based_options_destroy(opts: *mut DBBlockBasedTableOptions);
    pub fn rocksdb_block_based_options_set_block_size(
        block_options: *mut DBBlockBasedTableOptions,
        block_size: size_t);
    pub fn rocksdb_block_based_options_set_block_size_deviation(
        block_options: *mut DBBlockBasedTableOptions,
        block_size_deviation: c_int);
    pub fn rocksdb_block_based_options_set_block_restart_interval(
        block_options: *mut DBBlockBasedTableOptions,
        block_restart_interval: c_int);
    pub fn rocksdb_block_based_options_set_cache_index_and_filter_blocks(
        block_options: *mut DBBlockBasedTableOptions, v: c_uchar);
    pub fn rocksdb_block_based_options_set_filter_policy(
        block_options: *mut DBBlockBasedTableOptions,
        filter_policy: *mut DBFilterPolicy);
    pub fn rocksdb_block_based_options_set_no_block_cache(
        block_options: *mut DBBlockBasedTableOptions, no_block_cache: bool);
    pub fn rocksdb_block_based_options_set_block_cache(
        block_options: *mut DBBlockBasedTableOptions, block_cache: *mut DBCache);
    pub fn rocksdb_block_based_options_set_block_cache_compressed(
        block_options: *mut DBBlockBasedTableOptions,
        block_cache_compressed: *mut DBCache);
    pub fn rocksdb_block_based_options_set_whole_key_filtering(
        ck_options: *mut DBBlockBasedTableOptions, doit: bool);
    pub fn rocksdb_options_set_block_based_table_factory(
        options: *mut DBOptions,
        block_options: *mut DBBlockBasedTableOptions);
    pub fn rocksdb_options_increase_parallelism(options: *mut DBOptions, threads: c_int);
    pub fn rocksdb_options_optimize_level_style_compaction(options: *mut DBOptions,
                                                           memtable_memory_budget: c_int);
    pub fn rocksdb_options_set_compaction_filter(options: *mut DBOptions,
                                                 filter: *mut DBCompactionFilter);
    pub fn rocksdb_options_set_create_if_missing(options: *mut DBOptions, v: bool);
    pub fn rocksdb_options_set_max_open_files(options: *mut DBOptions, files: c_int);
    pub fn rocksdb_options_set_use_fsync(options: *mut DBOptions, v: c_int);
    pub fn rocksdb_options_set_bytes_per_sync(options: *mut DBOptions, bytes: u64);
    pub fn rocksdb_options_set_disable_data_sync(options: *mut DBOptions, v: c_int);
    pub fn rocksdb_options_set_allow_os_buffer(options: *mut DBOptions, is_allow: bool);
    pub fn rocksdb_options_optimize_for_point_lookup(options: *mut DBOptions,
                                                     block_cache_size_mb: u64);
    pub fn rocksdb_options_set_table_cache_numshardbits(options: *mut DBOptions, bits: c_int);
    pub fn rocksdb_options_set_max_write_buffer_number(options: *mut DBOptions, bufno: c_int);
    pub fn rocksdb_options_set_min_write_buffer_number_to_merge(options: *mut DBOptions,
                                                                bufno: c_int);
    pub fn rocksdb_options_set_level0_file_num_compaction_trigger(options: *mut DBOptions,
                                                                  no: c_int);
    pub fn rocksdb_options_set_level0_slowdown_writes_trigger(options: *mut DBOptions, no: c_int);
    pub fn rocksdb_options_set_level0_stop_writes_trigger(options: *mut DBOptions, no: c_int);
    pub fn rocksdb_options_set_write_buffer_size(options: *mut DBOptions, bytes: u64);
    pub fn rocksdb_options_set_target_file_size_base(options: *mut DBOptions, bytes: u64);
    pub fn rocksdb_options_set_target_file_size_multiplier(options: *mut DBOptions, mul: c_int);
    pub fn rocksdb_options_set_max_bytes_for_level_base(options: *mut DBOptions, bytes: u64);
    pub fn rocksdb_options_set_max_bytes_for_level_multiplier(options: *mut DBOptions,
                                                              mul: c_int);
    pub fn rocksdb_options_set_max_log_file_size(options: *mut DBOptions, bytes: u64);
    pub fn rocksdb_options_set_max_manifest_file_size(options: *mut DBOptions, bytes: u64);
    pub fn rocksdb_options_set_hash_skip_list_rep(options: *mut DBOptions,
                                                  bytes: u64,
                                                  a1: i32,
                                                  a2: i32);
    pub fn rocksdb_options_set_compaction_style(options: *mut DBOptions, cs: DBCompactionStyle);
    pub fn rocksdb_options_set_compression(options: *mut DBOptions,
                                           compression_style_no: DBCompressionType);
    pub fn rocksdb_options_set_compression_per_level(options: *mut DBOptions,
                                                     level_values: *const DBCompressionType,
                                                     num_levels: size_t);
    pub fn rocksdb_options_set_max_background_compactions(options: *mut DBOptions,
                                                          max_bg_compactions: c_int);
    pub fn rocksdb_options_set_max_background_flushes(options: *mut DBOptions,
                                                      max_bg_flushes: c_int);
    pub fn rocksdb_options_set_filter_deletes(options: *mut DBOptions, v: bool);
    pub fn rocksdb_options_set_disable_auto_compactions(options: *mut DBOptions, v: c_int);
    pub fn rocksdb_options_set_report_bg_io_stats(options: *mut DBOptions, v: c_int);
    pub fn rocksdb_options_set_wal_recovery_mode(options: *mut DBOptions, mode: DBRecoveryMode);
    pub fn rocksdb_options_enable_statistics(options: *mut DBOptions);
    pub fn rocksdb_options_statistics_get_string(options: *mut DBOptions) -> *const c_char;
    pub fn rocksdb_options_set_stats_dump_period_sec(options: *mut DBOptions, v: usize);
    pub fn rocksdb_options_set_num_levels(options: *mut DBOptions, v: c_int);
    pub fn rocksdb_options_set_ratelimiter(options: *mut DBOptions, limiter: *mut DBRateLimiter);
    pub fn rocksdb_ratelimiter_create(rate_bytes_per_sec: i64,
                                      refill_period_us: i64,
                                      fairness: i32)
                                      -> *mut DBRateLimiter;
    pub fn rocksdb_ratelimiter_destroy(limiter: *mut DBRateLimiter);
    pub fn rocksdb_filterpolicy_create_bloom_full(bits_per_key: c_int) -> *mut DBFilterPolicy;
    pub fn rocksdb_filterpolicy_create_bloom(bits_per_key: c_int) -> *mut DBFilterPolicy;
    pub fn rocksdb_open(options: *mut DBOptions,
                        path: *const c_char,
                        err: *mut *mut c_char)
                        -> *mut DBInstance;
    pub fn rocksdb_writeoptions_create() -> *mut DBWriteOptions;
    pub fn rocksdb_writeoptions_destroy(writeopts: *mut DBWriteOptions);
    pub fn rocksdb_writeoptions_set_sync(writeopts: *mut DBWriteOptions, v: bool);
    pub fn rocksdb_writeoptions_disable_WAL(writeopts: *mut DBWriteOptions, v: c_int);
    pub fn rocksdb_put(db: *mut DBInstance,
                       writeopts: *mut DBWriteOptions,
                       k: *const u8,
                       kLen: size_t,
                       v: *const u8,
                       vLen: size_t,
                       err: *mut *mut c_char);
    pub fn rocksdb_put_cf(db: *mut DBInstance,
                          writeopts: *mut DBWriteOptions,
                          cf: *mut DBCFHandle,
                          k: *const u8,
                          kLen: size_t,
                          v: *const u8,
                          vLen: size_t,
                          err: *mut *mut c_char);
    pub fn rocksdb_readoptions_create() -> *mut DBReadOptions;
    pub fn rocksdb_readoptions_destroy(readopts: *mut DBReadOptions);
    pub fn rocksdb_readoptions_set_verify_checksums(readopts: *mut DBReadOptions, v: bool);
    pub fn rocksdb_readoptions_set_fill_cache(readopts: *mut DBReadOptions, v: bool);
    pub fn rocksdb_readoptions_set_snapshot(readopts: *mut DBReadOptions,
                                            snapshot: *const DBSnapshot);
    pub fn rocksdb_readoptions_set_iterate_upper_bound(readopts: *mut DBReadOptions,
                                                       k: *const u8,
                                                       kLen: size_t);
    pub fn rocksdb_readoptions_set_read_tier(readopts: *mut DBReadOptions, tier: c_int);
    pub fn rocksdb_readoptions_set_tailing(readopts: *mut DBReadOptions, v: bool);

    pub fn rocksdb_get(db: *const DBInstance,
                       readopts: *const DBReadOptions,
                       k: *const u8,
                       kLen: size_t,
                       valLen: *const size_t,
                       err: *mut *mut c_char)
                       -> *mut u8;
    pub fn rocksdb_get_cf(db: *const DBInstance,
                          readopts: *const DBReadOptions,
                          cf_handle: *mut DBCFHandle,
                          k: *const u8,
                          kLen: size_t,
                          valLen: *const size_t,
                          err: *mut *mut c_char)
                          -> *mut u8;
    pub fn rocksdb_create_iterator(db: *mut DBInstance,
                                   readopts: *const DBReadOptions)
                                   -> *mut DBIterator;
    pub fn rocksdb_create_iterator_cf(db: *mut DBInstance,
                                      readopts: *const DBReadOptions,
                                      cf_handle: *mut DBCFHandle)
                                      -> *mut DBIterator;
    pub fn rocksdb_create_snapshot(db: *mut DBInstance) -> *const DBSnapshot;
    pub fn rocksdb_release_snapshot(db: *mut DBInstance, snapshot: *const DBSnapshot);

    pub fn rocksdb_delete(db: *mut DBInstance,
                          writeopts: *const DBWriteOptions,
                          k: *const u8,
                          kLen: size_t,
                          err: *mut *mut c_char);
    pub fn rocksdb_delete_cf(db: *mut DBInstance,
                             writeopts: *const DBWriteOptions,
                             cf: *mut DBCFHandle,
                             k: *const u8,
                             kLen: size_t,
                             err: *mut *mut c_char);
    pub fn rocksdb_close(db: *mut DBInstance);
    pub fn rocksdb_destroy_db(options: *const DBOptions,
                              path: *const c_char,
                              err: *mut *mut c_char);
    pub fn rocksdb_repair_db(options: *const DBOptions,
                             path: *const c_char,
                             err: *mut *mut c_char);
    // Merge
    pub fn rocksdb_merge(db: *mut DBInstance,
                         writeopts: *const DBWriteOptions,
                         k: *const u8,
                         kLen: size_t,
                         v: *const u8,
                         vLen: size_t,
                         err: *mut *mut c_char);
    pub fn rocksdb_merge_cf(db: *mut DBInstance,
                            writeopts: *const DBWriteOptions,
                            cf: *mut DBCFHandle,
                            k: *const u8,
                            kLen: size_t,
                            v: *const u8,
                            vLen: size_t,
                            err: *mut *mut c_char);
    pub fn rocksdb_mergeoperator_create(
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
    pub fn rocksdb_mergeoperator_destroy(mo: *mut DBMergeOperator);
    pub fn rocksdb_options_set_merge_operator(options: *mut DBOptions, mo: *mut DBMergeOperator);
    // Iterator
    pub fn rocksdb_iter_destroy(iter: *mut DBIterator);
    pub fn rocksdb_iter_valid(iter: *const DBIterator) -> bool;
    pub fn rocksdb_iter_seek_to_first(iter: *mut DBIterator);
    pub fn rocksdb_iter_seek_to_last(iter: *mut DBIterator);
    pub fn rocksdb_iter_seek(iter: *mut DBIterator, key: *const u8, klen: size_t);
    pub fn rocksdb_iter_next(iter: *mut DBIterator);
    pub fn rocksdb_iter_prev(iter: *mut DBIterator);
    pub fn rocksdb_iter_key(iter: *const DBIterator, klen: *mut size_t) -> *mut u8;
    pub fn rocksdb_iter_value(iter: *const DBIterator, vlen: *mut size_t) -> *mut u8;
    pub fn rocksdb_iter_get_error(iter: *const DBIterator, err: *mut *mut c_char);
    // Write batch
    pub fn rocksdb_write(db: *mut DBInstance,
                         writeopts: *const DBWriteOptions,
                         batch: *mut DBWriteBatch,
                         err: *mut *mut c_char);
    pub fn rocksdb_writebatch_create() -> *mut DBWriteBatch;
    pub fn rocksdb_writebatch_create_from(rep: *const u8, size: size_t) -> *mut DBWriteBatch;
    pub fn rocksdb_writebatch_destroy(batch: *mut DBWriteBatch);
    pub fn rocksdb_writebatch_clear(batch: *mut DBWriteBatch);
    pub fn rocksdb_writebatch_count(batch: *mut DBWriteBatch) -> c_int;
    pub fn rocksdb_writebatch_put(batch: *mut DBWriteBatch,
                                  key: *const u8,
                                  klen: size_t,
                                  val: *const u8,
                                  vlen: size_t);
    pub fn rocksdb_writebatch_put_cf(batch: *mut DBWriteBatch,
                                     cf: *mut DBCFHandle,
                                     key: *const u8,
                                     klen: size_t,
                                     val: *const u8,
                                     vlen: size_t);
    pub fn rocksdb_writebatch_merge(batch: *mut DBWriteBatch,
                                    key: *const u8,
                                    klen: size_t,
                                    val: *const u8,
                                    vlen: size_t);
    pub fn rocksdb_writebatch_merge_cf(batch: *mut DBWriteBatch,
                                       cf: *mut DBCFHandle,
                                       key: *const u8,
                                       klen: size_t,
                                       val: *const u8,
                                       vlen: size_t);
    pub fn rocksdb_writebatch_delete(batch: *mut DBWriteBatch, key: *const u8, klen: size_t);
    pub fn rocksdb_writebatch_delete_cf(batch: *mut DBWriteBatch,
                                        cf: *mut DBCFHandle,
                                        key: *const u8,
                                        klen: size_t);
    pub fn rocksdb_writebatch_iterate(batch: *mut DBWriteBatch,
                                      state: *mut c_void,
                                      put_fn: extern "C" fn(state: *mut c_void,
                                                            k: *const u8,
                                                            klen: size_t,
                                                            v: *const u8,
                                                            vlen: size_t),
                                      deleted_fn: extern "C" fn(state: *mut c_void,
                                                                k: *const u8,
                                                                klen: size_t));
    pub fn rocksdb_writebatch_data(batch: *mut DBWriteBatch, size: *mut size_t) -> *const u8;

    // Comparator
    pub fn rocksdb_options_set_comparator(options: *mut DBOptions, cb: *mut DBComparator);
    pub fn rocksdb_comparator_create(state: *mut c_void,
                                     destroy: extern "C" fn(*mut c_void) -> (),
                                     compare: extern "C" fn(arg: *mut c_void,
                                                            a: *const c_char,
                                                            alen: size_t,
                                                            b: *const c_char,
                                                            blen: size_t)
                                                            -> c_int,
                                     name_fn: extern "C" fn(*mut c_void) -> *const c_char)
                                     -> *mut DBComparator;
    pub fn rocksdb_comparator_destroy(cmp: *mut DBComparator);

    // Column Family
    pub fn rocksdb_open_column_families(options: *const DBOptions,
                                        path: *const c_char,
                                        num_column_families: c_int,
                                        column_family_names: *const *const c_char,
                                        column_family_options: *const *const DBOptions,
                                        column_family_handles: *const *mut DBCFHandle,
                                        err: *mut *mut c_char)
                                        -> *mut DBInstance;
    pub fn rocksdb_create_column_family(db: *mut DBInstance,
                                        column_family_options: *const DBOptions,
                                        column_family_name: *const c_char,
                                        err: *mut *mut c_char)
                                        -> *mut DBCFHandle;
    pub fn rocksdb_drop_column_family(db: *mut DBInstance,
                                      column_family_handle: *mut DBCFHandle,
                                      err: *mut *mut c_char);
    pub fn rocksdb_column_family_handle_destroy(column_family_handle: *mut DBCFHandle);
    pub fn rocksdb_list_column_families(db: *const DBOptions,
                                        path: *const c_char,
                                        lencf: *mut size_t,
                                        err: *mut *mut c_char)
                                        -> *mut *mut c_char;
    pub fn rocksdb_list_column_families_destroy(list: *mut *mut c_char, len: size_t);

    // Flush options
    pub fn rocksdb_flushoptions_create() -> *mut DBFlushOptions;
    pub fn rocksdb_flushoptions_destroy(opt: *mut DBFlushOptions);
    pub fn rocksdb_flushoptions_set_wait(opt: *mut DBFlushOptions, whether_wait: bool);

    pub fn rocksdb_flush(db: *mut DBInstance,
                         options: *const DBFlushOptions,
                         err: *mut *mut c_char);

    pub fn rocksdb_approximate_sizes(db: *mut DBInstance,
                                     num_ranges: c_int,
                                     range_start_key: *const *const u8,
                                     range_start_key_len: *const size_t,
                                     range_limit_key: *const *const u8,
                                     range_limit_key_len: *const size_t,
                                     sizes: *mut uint64_t);
    pub fn rocksdb_approximate_sizes_cf(db: *mut DBInstance,
                                        cf: *mut DBCFHandle,
                                        num_ranges: c_int,
                                        range_start_key: *const *const u8,
                                        range_start_key_len: *const size_t,
                                        range_limit_key: *const *const u8,
                                        range_limit_key_len: *const size_t,
                                        sizes: *mut uint64_t);
    pub fn rocksdb_compact_range(db: *mut DBInstance,
                                 start_key: *const u8,
                                 start_key_len: size_t,
                                 limit_key: *const u8,
                                 limit_key_len: size_t);
    pub fn rocksdb_compact_range_cf(db: *mut DBInstance,
                                    cf: *mut DBCFHandle,
                                    start_key: *const u8,
                                    start_key_len: size_t,
                                    limit_key: *const u8,
                                    limit_key_len: size_t);
    pub fn rocksdb_delete_file_in_range(db: *mut DBInstance,
                                        range_start_key: *const u8,
                                        range_start_key_len: size_t,
                                        range_limit_key: *const u8,
                                        range_limit_key_len: size_t,
                                        err: *mut *mut c_char);
    pub fn rocksdb_delete_file_in_range_cf(db: *mut DBInstance,
                                           cf: *mut DBCFHandle,
                                           range_start_key: *const u8,
                                           range_start_key_len: size_t,
                                           range_limit_key: *const u8,
                                           range_limit_key_len: size_t,
                                           err: *mut *mut c_char);
    pub fn rocksdb_property_value(db: *mut DBInstance, propname: *const c_char) -> *mut c_char;
    pub fn rocksdb_property_value_cf(db: *mut DBInstance,
                                     cf: *mut DBCFHandle,
                                     propname: *const c_char)
                                     -> *mut c_char;
    // Compaction filter
    pub fn rocksdb_compactionfilter_create(state: *mut c_void,
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
    pub fn rocksdb_compactionfilter_set_ignore_snapshots(filter: *mut DBCompactionFilter,
                                                         ignore_snapshot: bool);
    pub fn rocksdb_compactionfilter_destroy(filter: *mut DBCompactionFilter);
}

#[cfg(test)]
mod test {
    use libc::{self, c_void};
    use std::ffi::{CStr, CString};
    use std::ptr;
    use super::*;
    use tempdir::TempDir;

    #[test]
    fn internal() {
        unsafe {
            let opts = rocksdb_options_create();
            assert!(!opts.is_null());

            rocksdb_options_increase_parallelism(opts, 0);
            rocksdb_options_optimize_level_style_compaction(opts, 0);
            rocksdb_options_set_create_if_missing(opts, true);

            let rustpath = TempDir::new("_rust_rocksdb_internaltest").expect("");
            let cpath = CString::new(rustpath.path().to_str().unwrap()).unwrap();
            let cpath_ptr = cpath.as_ptr();

            let mut err = ptr::null_mut();
            let db = rocksdb_open(opts, cpath_ptr, &mut err);
            assert!(err.is_null(), error_message(err));

            let writeopts = rocksdb_writeoptions_create();
            assert!(!writeopts.is_null());

            let key = b"name\x00";
            let val = b"spacejam\x00";
            rocksdb_put(db, writeopts, key.as_ptr(), 4, val.as_ptr(), 8, &mut err);
            rocksdb_writeoptions_destroy(writeopts);
            assert!(err.is_null(), error_message(err));

            let readopts = rocksdb_readoptions_create();
            assert!(!readopts.is_null());

            let mut val_len = 0;
            rocksdb_get(db, readopts, key.as_ptr(), 4, &mut val_len, &mut err);
            rocksdb_readoptions_destroy(readopts);
            assert!(err.is_null(), error_message(err));

            // flush first to get approximate size later.
            let flush_opt = rocksdb_flushoptions_create();
            rocksdb_flushoptions_set_wait(flush_opt, true);
            rocksdb_flush(db, flush_opt, &mut err);
            rocksdb_flushoptions_destroy(flush_opt);
            assert!(err.is_null(), error_message(err));

            let mut sizes = vec![0; 1];
            rocksdb_approximate_sizes(db,
                                      1,
                                      vec![b"\x00\x00".as_ptr()].as_ptr(),
                                      vec![1].as_ptr(),
                                      vec![b"\xff\x00".as_ptr()].as_ptr(),
                                      vec![1].as_ptr(),
                                      sizes.as_mut_ptr());
            assert_eq!(sizes.len(), 1);
            assert!(sizes[0] > 0);

            rocksdb_delete_file_in_range(db,
                                         b"\x00\x00".as_ptr(),
                                         2,
                                         b"\xff\x00".as_ptr(),
                                         2,
                                         &mut err);
            assert!(err.is_null(), error_message(err));

            let propname = CString::new("rocksdb.total-sst-files-size").unwrap();
            let value = rocksdb_property_value(db, propname.as_ptr());
            assert!(!value.is_null());

            let sst_size = CStr::from_ptr(value).to_str().unwrap().parse::<u64>().unwrap();
            assert!(sst_size > 0);
            libc::free(value as *mut c_void);

            let propname = CString::new("fake_key").unwrap();
            let value = rocksdb_property_value(db, propname.as_ptr());
            assert!(value.is_null());
            libc::free(value as *mut c_void);

            rocksdb_close(db);
            rocksdb_destroy_db(opts, cpath_ptr, &mut err);
            assert!(err.is_null());
        }
    }
}
