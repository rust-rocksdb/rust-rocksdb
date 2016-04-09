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
use libc::{self, c_char, c_int, c_void, size_t, uint64_t};
use std::ffi::CStr;
use std::str::from_utf8;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct DBOptions(pub *const c_void);
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DBInstance(pub *const c_void);
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DBWriteOptions(pub *const c_void);
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DBReadOptions(pub *const c_void);
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DBMergeOperator(pub *const c_void);
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DBBlockBasedTableOptions(pub *const c_void);
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DBCache(pub *const c_void);
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DBFilterPolicy(pub *const c_void);
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DBSnapshot(pub *const c_void);
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DBIterator(pub *const c_void);
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DBCFHandle(pub *const c_void);
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DBWriteBatch(pub *const c_void);
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DBComparator(pub *const c_void);
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DBFlushOptions(pub *const c_void);

pub fn new_bloom_filter(bits: c_int) -> DBFilterPolicy {
    unsafe { rocksdb_filterpolicy_create_bloom(bits) }
}

pub fn new_cache(capacity: size_t) -> DBCache {
    unsafe { rocksdb_cache_create_lru(capacity) }
}

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

pub fn error_message(ptr: *const i8) -> String {
    let c_str = unsafe { CStr::from_ptr(ptr as *const _) };
    let s = from_utf8(c_str.to_bytes()).unwrap().to_owned();
    unsafe {
        libc::free(ptr as *mut c_void);
    }
    s
}

// TODO audit the use of boolean arguments, b/c I think they need to be u8
// instead...
#[link(name = "rocksdb")]
extern "C" {
    pub fn rocksdb_options_create() -> DBOptions;
    pub fn rocksdb_options_destroy(opts: DBOptions);
    pub fn rocksdb_cache_create_lru(capacity: size_t) -> DBCache;
    pub fn rocksdb_cache_destroy(cache: DBCache);
    pub fn rocksdb_block_based_options_create() -> DBBlockBasedTableOptions;
    pub fn rocksdb_block_based_options_destroy(opts: DBBlockBasedTableOptions);
    pub fn rocksdb_block_based_options_set_block_size(
        block_options: DBBlockBasedTableOptions,
        block_size: size_t);
    pub fn rocksdb_block_based_options_set_block_size_deviation(
        block_options: DBBlockBasedTableOptions,
        block_size_deviation: c_int);
    pub fn rocksdb_block_based_options_set_block_restart_interval(
        block_options: DBBlockBasedTableOptions,
        block_restart_interval: c_int);
    pub fn rocksdb_block_based_options_set_filter_policy(
        block_options: DBBlockBasedTableOptions,
        filter_policy: DBFilterPolicy);
    pub fn rocksdb_block_based_options_set_no_block_cache(
        block_options: DBBlockBasedTableOptions, no_block_cache: bool);
    pub fn rocksdb_block_based_options_set_block_cache(
        block_options: DBBlockBasedTableOptions, block_cache: DBCache);
    pub fn rocksdb_block_based_options_set_block_cache_compressed(
        block_options: DBBlockBasedTableOptions,
        block_cache_compressed: DBCache);
    pub fn rocksdb_block_based_options_set_whole_key_filtering(
        ck_options: DBBlockBasedTableOptions, doit: bool);
    pub fn rocksdb_options_set_block_based_table_factory(
        options: DBOptions,
        block_options: DBBlockBasedTableOptions);
    pub fn rocksdb_options_increase_parallelism(options: DBOptions,
                                                threads: c_int);
    pub fn rocksdb_options_optimize_level_style_compaction(
        options: DBOptions, memtable_memory_budget: c_int);
    pub fn rocksdb_options_set_create_if_missing(options: DBOptions, v: bool);
    pub fn rocksdb_options_set_max_open_files(options: DBOptions,
                                              files: c_int);
    pub fn rocksdb_options_set_use_fsync(options: DBOptions, v: c_int);
    pub fn rocksdb_options_set_bytes_per_sync(options: DBOptions, bytes: u64);
    pub fn rocksdb_options_set_disable_data_sync(options: DBOptions,
                                                 v: c_int);
    pub fn rocksdb_options_optimize_for_point_lookup(options: DBOptions,
                                                     block_cache_size_mb: u64);
    pub fn rocksdb_options_set_table_cache_numshardbits(options: DBOptions,
                                                        bits: c_int);
    pub fn rocksdb_options_set_max_write_buffer_number(options: DBOptions,
                                                       bufno: c_int);
    pub fn rocksdb_options_set_min_write_buffer_number_to_merge(
        options: DBOptions, bufno: c_int);
    pub fn rocksdb_options_set_level0_file_num_compaction_trigger(
        options: DBOptions, no: c_int);
    pub fn rocksdb_options_set_level0_slowdown_writes_trigger(
        options: DBOptions, no: c_int);
    pub fn rocksdb_options_set_level0_stop_writes_trigger(options: DBOptions,
                                                          no: c_int);
    pub fn rocksdb_options_set_write_buffer_size(options: DBOptions,
                                                 bytes: u64);
    pub fn rocksdb_options_set_target_file_size_base(options: DBOptions,
                                                     bytes: u64);
    pub fn rocksdb_options_set_target_file_size_multiplier(options: DBOptions,
                                                           mul: c_int);
    pub fn rocksdb_options_set_max_log_file_size(options: DBOptions,
                                                 bytes: u64);
    pub fn rocksdb_options_set_max_manifest_file_size(options: DBOptions,
                                                      bytes: u64);
    pub fn rocksdb_options_set_hash_skip_list_rep(options: DBOptions,
                                                  bytes: u64,
                                                  a1: i32,
                                                  a2: i32);
    pub fn rocksdb_options_set_compaction_style(options: DBOptions,
                                                cs: DBCompactionStyle);
    pub fn rocksdb_options_set_compression(options: DBOptions,
                                           compression_style_no: c_int);
    pub fn rocksdb_options_set_max_background_compactions(
        options: DBOptions, max_bg_compactions: c_int);
    pub fn rocksdb_options_set_max_background_flushes(options: DBOptions,
                                                      max_bg_flushes: c_int);
    pub fn rocksdb_options_set_filter_deletes(options: DBOptions, v: bool);
    pub fn rocksdb_options_set_disable_auto_compactions(options: DBOptions,
                                                        v: c_int);
    pub fn rocksdb_filterpolicy_create_bloom(bits_per_key: c_int)
                                             -> DBFilterPolicy;
    pub fn rocksdb_open(options: DBOptions,
                        path: *const i8,
                        err: *mut *const i8)
                        -> DBInstance;
    pub fn rocksdb_writeoptions_create() -> DBWriteOptions;
    pub fn rocksdb_writeoptions_destroy(writeopts: DBWriteOptions);
    pub fn rocksdb_writeoptions_set_sync(writeopts: DBWriteOptions, v: bool);
    pub fn rocksdb_writeoptions_disable_WAL(writeopts: DBWriteOptions,
                                            v: c_int);
    pub fn rocksdb_put(db: DBInstance,
                       writeopts: DBWriteOptions,
                       k: *const u8,
                       kLen: size_t,
                       v: *const u8,
                       vLen: size_t,
                       err: *mut *const i8);
    pub fn rocksdb_put_cf(db: DBInstance,
                          writeopts: DBWriteOptions,
                          cf: DBCFHandle,
                          k: *const u8,
                          kLen: size_t,
                          v: *const u8,
                          vLen: size_t,
                          err: *mut *const i8);
    pub fn rocksdb_readoptions_create() -> DBReadOptions;
    pub fn rocksdb_readoptions_destroy(readopts: DBReadOptions);
    pub fn rocksdb_readoptions_set_verify_checksums(readopts: DBReadOptions,
                                                    v: bool);
    pub fn rocksdb_readoptions_set_fill_cache(readopts: DBReadOptions,
                                              v: bool);
    pub fn rocksdb_readoptions_set_snapshot(readopts: DBReadOptions,
                                            snapshot: DBSnapshot); //TODO how do I make this a const ref?
    pub fn rocksdb_readoptions_set_iterate_upper_bound(readopts: DBReadOptions,
                                                       k: *const u8,
                                                       kLen: size_t);
    pub fn rocksdb_readoptions_set_read_tier(readopts: DBReadOptions,
                                             tier: c_int);
    pub fn rocksdb_readoptions_set_tailing(readopts: DBReadOptions, v: bool);

    pub fn rocksdb_get(db: DBInstance,
                       readopts: DBReadOptions,
                       k: *const u8,
                       kLen: size_t,
                       valLen: *const size_t,
                       err: *mut *const i8)
                       -> *mut c_void;
    pub fn rocksdb_get_cf(db: DBInstance,
                          readopts: DBReadOptions,
                          cf_handle: DBCFHandle,
                          k: *const u8,
                          kLen: size_t,
                          valLen: *const size_t,
                          err: *mut *const i8)
                          -> *mut c_void;
    pub fn rocksdb_create_iterator(db: DBInstance,
                                   readopts: DBReadOptions)
                                   -> DBIterator;
    pub fn rocksdb_create_iterator_cf(db: DBInstance,
                                      readopts: DBReadOptions,
                                      cf_handle: DBCFHandle)
                                      -> DBIterator;
    pub fn rocksdb_create_snapshot(db: DBInstance) -> DBSnapshot;
    pub fn rocksdb_release_snapshot(db: DBInstance, snapshot: DBSnapshot);

    pub fn rocksdb_delete(db: DBInstance,
                          writeopts: DBWriteOptions,
                          k: *const u8,
                          kLen: size_t,
                          err: *mut *const i8)
                          -> *mut c_void;
    pub fn rocksdb_delete_cf(db: DBInstance,
                             writeopts: DBWriteOptions,
                             cf: DBCFHandle,
                             k: *const u8,
                             kLen: size_t,
                             err: *mut *const i8)
                             -> *mut c_void;
    pub fn rocksdb_close(db: DBInstance);
    pub fn rocksdb_destroy_db(options: DBOptions,
                              path: *const i8,
                              err: *mut *const i8);
    pub fn rocksdb_repair_db(options: DBOptions,
                             path: *const i8,
                             err: *mut *const i8);
    // Merge
    pub fn rocksdb_merge(db: DBInstance,
                         writeopts: DBWriteOptions,
                         k: *const u8,
                         kLen: size_t,
                         v: *const u8,
                         vLen: size_t,
                         err: *mut *const i8);
    pub fn rocksdb_merge_cf(db: DBInstance,
                            writeopts: DBWriteOptions,
                            cf: DBCFHandle,
                            k: *const u8,
                            kLen: size_t,
                            v: *const u8,
                            vLen: size_t,
                            err: *mut *const i8);
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
    ) -> DBMergeOperator;
    pub fn rocksdb_mergeoperator_destroy(mo: DBMergeOperator);
    pub fn rocksdb_options_set_merge_operator(options: DBOptions,
                                              mo: DBMergeOperator);
    // Iterator
    pub fn rocksdb_iter_destroy(iter: DBIterator);
    pub fn rocksdb_iter_valid(iter: DBIterator) -> bool;
    pub fn rocksdb_iter_seek_to_first(iter: DBIterator);
    pub fn rocksdb_iter_seek_to_last(iter: DBIterator);
    pub fn rocksdb_iter_seek(iter: DBIterator, key: *const u8, klen: size_t);
    pub fn rocksdb_iter_next(iter: DBIterator);
    pub fn rocksdb_iter_prev(iter: DBIterator);
    pub fn rocksdb_iter_key(iter: DBIterator, klen: *mut size_t) -> *mut u8;
    pub fn rocksdb_iter_value(iter: DBIterator, vlen: *mut size_t) -> *mut u8;
    pub fn rocksdb_iter_get_error(iter: DBIterator, err: *mut *const u8);
    // Write batch
    pub fn rocksdb_write(db: DBInstance,
                         writeopts: DBWriteOptions,
                         batch: DBWriteBatch,
                         err: *mut *const i8);
    pub fn rocksdb_writebatch_create() -> DBWriteBatch;
    pub fn rocksdb_writebatch_create_from(rep: *const u8,
                                          size: size_t)
                                          -> DBWriteBatch;
    pub fn rocksdb_writebatch_destroy(batch: DBWriteBatch);
    pub fn rocksdb_writebatch_clear(batch: DBWriteBatch);
    pub fn rocksdb_writebatch_count(batch: DBWriteBatch) -> c_int;
    pub fn rocksdb_writebatch_put(batch: DBWriteBatch,
                                  key: *const u8,
                                  klen: size_t,
                                  val: *const u8,
                                  vlen: size_t);
    pub fn rocksdb_writebatch_put_cf(batch: DBWriteBatch,
                                     cf: DBCFHandle,
                                     key: *const u8,
                                     klen: size_t,
                                     val: *const u8,
                                     vlen: size_t);
    pub fn rocksdb_writebatch_merge(batch: DBWriteBatch,
                                    key: *const u8,
                                    klen: size_t,
                                    val: *const u8,
                                    vlen: size_t);
    pub fn rocksdb_writebatch_merge_cf(batch: DBWriteBatch,
                                       cf: DBCFHandle,
                                       key: *const u8,
                                       klen: size_t,
                                       val: *const u8,
                                       vlen: size_t);
    pub fn rocksdb_writebatch_delete(batch: DBWriteBatch,
                                     key: *const u8,
                                     klen: size_t);
    pub fn rocksdb_writebatch_delete_cf(batch: DBWriteBatch,
                                        cf: DBCFHandle,
                                        key: *const u8,
                                        klen: size_t);
    pub fn rocksdb_writebatch_iterate(
        batch: DBWriteBatch,
        state: *mut c_void,
        put_fn: extern fn(state: *mut c_void,
                          k: *const u8, klen: size_t,
                          v: *const u8, vlen: size_t),
        deleted_fn: extern fn(state: *mut c_void,
                              k: *const u8, klen: size_t));
    pub fn rocksdb_writebatch_data(batch: DBWriteBatch,
                                   size: *mut size_t)
                                   -> *const u8;

    // Comparator
    pub fn rocksdb_options_set_comparator(options: DBOptions,
                                          cb: DBComparator);
    pub fn rocksdb_comparator_create(state: *mut c_void,
                                     destroy: extern "C" fn(*mut c_void) -> (),
                                     compare: extern "C" fn(arg: *mut c_void,
                                                            a: *const c_char,
                                                            alen: size_t,
                                                            b: *const c_char,
                                                            blen: size_t)
                                                            -> c_int,
                                     name_fn: extern "C" fn(*mut c_void)
                                                            -> *const c_char)
                                     -> DBComparator;
    pub fn rocksdb_comparator_destroy(cmp: DBComparator);

    // Column Family
    pub fn rocksdb_open_column_families(options: DBOptions,
                                        path: *const i8,
                                        num_column_families: c_int,
                                        column_family_names: *const *const i8,
                                        column_family_options: *const DBOptions,
                                        column_family_handles: *const DBCFHandle,
                                        err: *mut *const i8
                                        ) -> DBInstance;
    pub fn rocksdb_create_column_family(db: DBInstance,
                                        column_family_options: DBOptions,
                                        column_family_name: *const i8,
                                        err: *mut *const i8)
                                        -> DBCFHandle;
    pub fn rocksdb_drop_column_family(db: DBInstance,
                                      column_family_handle: DBCFHandle,
                                      err: *mut *const i8);
    pub fn rocksdb_column_family_handle_destroy(column_family_handle: DBCFHandle);

    // Flush options
    pub fn rocksdb_flushoptions_create() -> DBFlushOptions;
    pub fn rocksdb_flushoptions_destroy(opt: DBFlushOptions);
    pub fn rocksdb_flushoptions_set_wait(opt: DBFlushOptions,
                                         whether_wait: bool);

    pub fn rocksdb_flush(db: DBInstance,
                         options: DBFlushOptions,
                         err: *mut *const i8);

    pub fn rocksdb_approximate_sizes(db: DBInstance,
                                     num_ranges: c_int,
                                     range_start_key: *const *const u8,
                                     range_start_key_len: *const size_t,
                                     range_limit_key: *const *const u8,
                                     range_limit_key_len: *const size_t,
                                     sizes: *mut uint64_t);
    pub fn rocksdb_approximate_sizes_cf(db: DBInstance,
                                        cf: DBCFHandle,
                                        num_ranges: c_int,
                                        range_start_key: *const *const u8,
                                        range_start_key_len: *const size_t,
                                        range_limit_key: *const *const u8,
                                        range_limit_key_len: *const size_t,
                                        sizes: *mut uint64_t);
}

#[cfg(test)]
mod test {
    use super::*;
    use std::ffi::CString;
    use tempdir::TempDir;

    #[test]
    fn internal() {
        unsafe {
            let opts = rocksdb_options_create();
            assert!(!opts.0.is_null());

            rocksdb_options_increase_parallelism(opts, 0);
            rocksdb_options_optimize_level_style_compaction(opts, 0);
            rocksdb_options_set_create_if_missing(opts, true);

            let rustpath = TempDir::new("_rust_rocksdb_internaltest")
                               .expect("");
            let cpath = CString::new(rustpath.path().to_str().unwrap())
                            .unwrap();
            let cpath_ptr = cpath.as_ptr();

            let mut err = 0 as *const i8;
            let db = rocksdb_open(opts, cpath_ptr, &mut err);
            assert!(err.is_null(), error_message(err));

            let writeopts = rocksdb_writeoptions_create();
            assert!(!writeopts.0.is_null());

            let key = b"name\x00";
            let val = b"spacejam\x00";
            rocksdb_put(db,
                        writeopts.clone(),
                        key.as_ptr(),
                        4,
                        val.as_ptr(),
                        8,
                        &mut err);
            rocksdb_writeoptions_destroy(writeopts);
            assert!(err.is_null(), error_message(err));

            let readopts = rocksdb_readoptions_create();
            assert!(!readopts.0.is_null());

            let mut val_len = 0;
            rocksdb_get(db,
                        readopts.clone(),
                        key.as_ptr(),
                        4,
                        &mut val_len,
                        &mut err);
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

            rocksdb_close(db);
            rocksdb_destroy_db(opts, cpath_ptr, &mut err);
            assert!(err.is_null());
        }
    }
}
