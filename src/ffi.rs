/*
   Copyright 2014 Tyler Neely

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/
extern crate libc;
use self::libc::{c_char, c_int, c_void, size_t};
use std::ffi::CString;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct RocksDBOptions(pub *const c_void);
#[derive(Copy, Clone)]
#[repr(C)]
pub struct RocksDBInstance(pub *const c_void);
#[derive(Copy, Clone)]
#[repr(C)]
pub struct RocksDBWriteOptions(pub *const c_void);
#[derive(Copy, Clone)]
#[repr(C)]
pub struct RocksDBReadOptions(pub *const c_void);
#[derive(Copy, Clone)]
#[repr(C)]
pub struct RocksDBMergeOperator(pub *const c_void);
#[derive(Copy, Clone)]
#[repr(C)]
pub struct RocksDBBlockBasedTableOptions(pub *const c_void);
#[derive(Copy, Clone)]
#[repr(C)]
pub struct RocksDBCache(pub *const c_void);
#[derive(Copy, Clone)]
#[repr(C)]
pub struct RocksDBFilterPolicy(pub *const c_void);
#[derive(Copy, Clone)]
#[repr(C)]
pub struct RocksDBSnapshot(pub *const c_void);
#[derive(Copy, Clone)]
#[repr(C)]
pub struct RocksDBIterator(pub *const c_void);
#[derive(Copy, Clone)]
#[repr(C)]
pub struct RocksDBCFHandle(pub *const c_void);
#[derive(Copy, Clone)]
#[repr(C)]
pub struct RocksDBWriteBatch(pub *const c_void);

pub fn new_bloom_filter(bits: c_int) -> RocksDBFilterPolicy {
    unsafe {
        rocksdb_filterpolicy_create_bloom(bits)
    }
}

pub fn new_cache(capacity: size_t) -> RocksDBCache {
    unsafe {
        rocksdb_cache_create_lru(capacity)
    }
}

#[repr(C)]
pub enum RocksDBCompressionType {
    RocksDBNoCompression     = 0,
    RocksDBSnappyCompression = 1,
    RocksDBZlibCompression   = 2,
    RocksDBBz2Compression    = 3,
    RocksDBLz4Compression    = 4,
    RocksDBLz4hcCompression  = 5
}

#[repr(C)]
pub enum RocksDBCompactionStyle {
    RocksDBLevelCompaction     = 0,
    RocksDBUniversalCompaction = 1,
    RocksDBFifoCompaction      = 2
}

#[repr(C)]
pub enum RocksDBUniversalCompactionStyle {
    rocksdb_similar_size_compaction_stop_style = 0,
    rocksdb_total_size_compaction_stop_style   = 1
}

#[link(name = "rocksdb")]
extern {
    pub fn rocksdb_options_create() -> RocksDBOptions;
    pub fn rocksdb_cache_create_lru(capacity: size_t) -> RocksDBCache;
    pub fn rocksdb_cache_destroy(cache: RocksDBCache);
    pub fn rocksdb_block_based_options_create() -> RocksDBBlockBasedTableOptions;
    pub fn rocksdb_block_based_options_destroy(
        block_options: RocksDBBlockBasedTableOptions);
    pub fn rocksdb_block_based_options_set_block_size(
        block_options: RocksDBBlockBasedTableOptions,
        block_size: size_t);
    pub fn rocksdb_block_based_options_set_block_size_deviation(
        block_options: RocksDBBlockBasedTableOptions,
        block_size_deviation: c_int);
    pub fn rocksdb_block_based_options_set_block_restart_interval(
        block_options: RocksDBBlockBasedTableOptions,
        block_restart_interval: c_int);
    pub fn rocksdb_block_based_options_set_filter_policy(
        block_options: RocksDBBlockBasedTableOptions,
        filter_policy: RocksDBFilterPolicy);
    pub fn rocksdb_block_based_options_set_no_block_cache(
        block_options: RocksDBBlockBasedTableOptions, no_block_cache: bool);
    pub fn rocksdb_block_based_options_set_block_cache(
        block_options: RocksDBBlockBasedTableOptions, block_cache: RocksDBCache);
    pub fn rocksdb_block_based_options_set_block_cache_compressed(
        block_options: RocksDBBlockBasedTableOptions,
        block_cache_compressed: RocksDBCache);
    pub fn rocksdb_block_based_options_set_whole_key_filtering(
        ck_options: RocksDBBlockBasedTableOptions, doit: bool);
    pub fn rocksdb_options_set_block_based_table_factory(
        options: RocksDBOptions,
        block_options: RocksDBBlockBasedTableOptions);
    pub fn rocksdb_options_increase_parallelism(
        options: RocksDBOptions, threads: c_int);
    pub fn rocksdb_options_optimize_level_style_compaction(
        options: RocksDBOptions, memtable_memory_budget: c_int);
    pub fn rocksdb_options_set_create_if_missing(
        options: RocksDBOptions, v: bool);
    pub fn rocksdb_options_set_max_open_files(
        options: RocksDBOptions, files: c_int);
    pub fn rocksdb_options_set_use_fsync(
        options: RocksDBOptions, v: c_int);
    pub fn rocksdb_options_set_bytes_per_sync(
        options: RocksDBOptions, bytes: u64);
    pub fn rocksdb_options_set_disable_data_sync(
        options: RocksDBOptions, v: c_int);
    pub fn rocksdb_options_optimize_for_point_lookup(
        options: RocksDBOptions, block_cache_size_mb: u64);
    pub fn rocksdb_options_set_table_cache_numshardbits(
        options: RocksDBOptions, bits: c_int);
    pub fn rocksdb_options_set_max_write_buffer_number(
        options: RocksDBOptions, bufno: c_int);
    pub fn rocksdb_options_set_min_write_buffer_number_to_merge(
        options: RocksDBOptions, bufno: c_int);
    pub fn rocksdb_options_set_level0_file_num_compaction_trigger(
        options: RocksDBOptions, no: c_int);
    pub fn rocksdb_options_set_level0_slowdown_writes_trigger(
        options: RocksDBOptions, no: c_int);
    pub fn rocksdb_options_set_level0_stop_writes_trigger(
        options: RocksDBOptions, no: c_int);
    pub fn rocksdb_options_set_write_buffer_size(
        options: RocksDBOptions, bytes: u64);
    pub fn rocksdb_options_set_target_file_size_base(
        options: RocksDBOptions, bytes: u64);
    pub fn rocksdb_options_set_target_file_size_multiplier(
        options: RocksDBOptions, mul: c_int);
    pub fn rocksdb_options_set_max_log_file_size(
        options: RocksDBOptions, bytes: u64);
    pub fn rocksdb_options_set_max_manifest_file_size(
        options: RocksDBOptions, bytes: u64);
    pub fn rocksdb_options_set_hash_skip_list_rep(
        options: RocksDBOptions, bytes: u64, a1: i32, a2: i32);
    pub fn rocksdb_options_set_compaction_style(
        options: RocksDBOptions, cs: RocksDBCompactionStyle);
    pub fn rocksdb_options_set_compression(
        options: RocksDBOptions, compression_style_no: c_int);
    pub fn rocksdb_options_set_max_background_compactions(
        options: RocksDBOptions, max_bg_compactions: c_int);
    pub fn rocksdb_options_set_max_background_flushes(
        options: RocksDBOptions, max_bg_flushes: c_int);
    pub fn rocksdb_options_set_filter_deletes(
        options: RocksDBOptions, v: bool);
    pub fn rocksdb_options_set_disable_auto_compactions(
        options: RocksDBOptions, v: c_int);
    pub fn rocksdb_filterpolicy_create_bloom(
        bits_per_key: c_int) -> RocksDBFilterPolicy;
    pub fn rocksdb_open(options: RocksDBOptions,
                        path: *const i8,
                        err: *mut i8
                        ) -> RocksDBInstance;
    pub fn rocksdb_writeoptions_create() -> RocksDBWriteOptions;
    pub fn rocksdb_writeoptions_destroy(writeopts: RocksDBWriteOptions);
    pub fn rocksdb_put(db: RocksDBInstance,
                       writeopts: RocksDBWriteOptions,
                       k: *const u8, kLen: size_t,
                       v: *const u8, vLen: size_t,
                       err: *mut i8);
    pub fn rocksdb_readoptions_create() -> RocksDBReadOptions;
    pub fn rocksdb_readoptions_destroy(readopts: RocksDBReadOptions);
    pub fn rocksdb_readoptions_set_snapshot(read_opts: RocksDBReadOptions,
                                            snapshot: RocksDBSnapshot);
    pub fn rocksdb_get(db: RocksDBInstance,
                       readopts: RocksDBReadOptions,
                       k: *const u8, kLen: size_t,
                       valLen: *const size_t,
                       err: *mut i8
                       ) -> *mut c_void;
    pub fn rocksdb_get_cf(db: RocksDBInstance,
                       readopts: RocksDBReadOptions,
                       cf_handle: RocksDBCFHandle,
                       k: *const u8, kLen: size_t,
                       valLen: *const size_t,
                       err: *mut i8
                       ) -> *mut c_void;
    pub fn rocksdb_create_iterator(db: RocksDBInstance,
                                   readopts: RocksDBReadOptions
                                   ) -> RocksDBIterator;
    pub fn rocksdb_create_iterator_cf(db: RocksDBInstance,
                                      readopts: RocksDBReadOptions,
                                      cf_handle: RocksDBCFHandle
                                      ) -> RocksDBIterator;
    pub fn rocksdb_create_snapshot(db: RocksDBInstance) -> RocksDBSnapshot;
    pub fn rocksdb_release_snapshot(db: RocksDBInstance,
                                    snapshot: RocksDBSnapshot);

    pub fn rocksdb_delete(db: RocksDBInstance,
                          writeopts: RocksDBWriteOptions,
                          k: *const u8, kLen: size_t,
                          err: *mut i8
                          ) -> *mut c_void;
    pub fn rocksdb_close(db: RocksDBInstance);
    pub fn rocksdb_destroy_db(options: RocksDBOptions,
                              path: *const i8, err: *mut i8);
    pub fn rocksdb_repair_db(options: RocksDBOptions,
                             path: *const i8, err: *mut i8);
    // Merge
    pub fn rocksdb_merge(db: RocksDBInstance,
                         writeopts: RocksDBWriteOptions,
                         k: *const u8, kLen: size_t,
                         v: *const u8, vLen: size_t,
                         err: *mut i8);
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
    ) -> RocksDBMergeOperator;
    pub fn rocksdb_mergeoperator_destroy(mo: RocksDBMergeOperator);
    pub fn rocksdb_options_set_merge_operator(options: RocksDBOptions,
                                              mo: RocksDBMergeOperator);
    // Iterator
    pub fn rocksdb_iter_destroy(iter: RocksDBIterator);
    pub fn rocksdb_iter_valid(iter: RocksDBIterator) -> bool;
    pub fn rocksdb_iter_seek_to_first(iter: RocksDBIterator);
    pub fn rocksdb_iter_seek_to_last(iter: RocksDBIterator);
    pub fn rocksdb_iter_seek(iter: RocksDBIterator,
                             key: *mut u8, klen: size_t);
    pub fn rocksdb_iter_next(iter: RocksDBIterator);
    pub fn rocksdb_iter_prev(iter: RocksDBIterator);
    pub fn rocksdb_iter_key(iter: RocksDBIterator,
                            klen: *mut size_t) -> *mut u8;
    pub fn rocksdb_iter_value(iter: RocksDBIterator,
                              vlen: *mut size_t) -> *mut u8;
    pub fn rocksdb_iter_get_error(iter: RocksDBIterator,
                                  err: *const *const u8);
    // Write batch
    pub fn rocksdb_write(db: RocksDBInstance,
                         writeopts: RocksDBWriteOptions,
                         batch : RocksDBWriteBatch,
                         err: *mut i8);
    pub fn rocksdb_writebatch_create() -> RocksDBWriteBatch;
    pub fn rocksdb_writebatch_create_from(rep: *const u8,
                                          size: size_t) -> RocksDBWriteBatch;
    pub fn rocksdb_writebatch_destroy(batch: RocksDBWriteBatch);
    pub fn rocksdb_writebatch_clear(batch: RocksDBWriteBatch);
    pub fn rocksdb_writebatch_count(batch: RocksDBWriteBatch) -> c_int;
    pub fn rocksdb_writebatch_put(batch: RocksDBWriteBatch,
                                  key: *const u8, klen: size_t,
                                  val: *const u8, vlen: size_t);
    pub fn rocksdb_writebatch_put_cf(batch: RocksDBWriteBatch,
                                     cf: RocksDBCFHandle,
                                     key: *const u8, klen: size_t,
                                     val: *const u8, vlen: size_t);
    pub fn rocksdb_writebatch_merge(
        batch: RocksDBWriteBatch,
        key: *const u8, klen: size_t,
        val: *const u8, vlen: size_t);
    pub fn rocksdb_writebatch_merge_cf(
        batch: RocksDBWriteBatch,
        cf: RocksDBCFHandle,
        key: *const u8, klen: size_t,
        val: *const u8, vlen: size_t);
    pub fn rocksdb_writebatch_delete(
        batch: RocksDBWriteBatch,
        key: *const u8, klen: size_t);
    pub fn rocksdb_writebatch_delete_cf(
        batch: RocksDBWriteBatch,
        cf: RocksDBCFHandle,
        key: *const u8, klen: size_t);
    pub fn rocksdb_writebatch_iterate(
        batch: RocksDBWriteBatch,
        state: *mut c_void,
        put_fn: extern fn(state: *mut c_void,
                          k: *const u8, klen: size_t,
                          v: *const u8, vlen: size_t),
        deleted_fn: extern fn(state: *mut c_void,
                              k: *const u8, klen: size_t));
    pub fn rocksdb_writebatch_data(batch: RocksDBWriteBatch,
                                   size: *mut size_t) -> *const u8;
    }

#[allow(dead_code)]
#[test]
fn internal() {
    unsafe {
        let opts = rocksdb_options_create();
        let RocksDBOptions(opt_ptr) = opts;
        assert!(!opt_ptr.is_null());

        rocksdb_options_increase_parallelism(opts, 0);
        rocksdb_options_optimize_level_style_compaction(opts, 0);
        rocksdb_options_set_create_if_missing(opts, true);

        let rustpath = "_rust_rocksdb_internaltest";
        let cpath = CString::new(rustpath).unwrap();
        let cpath_ptr = cpath.as_ptr();

        let err = 0 as *mut i8;
        let db = rocksdb_open(opts, cpath_ptr, err);
        assert!(err.is_null());

        let writeopts = rocksdb_writeoptions_create();
        assert!(!writeopts.0.is_null());

        let key = b"name\x00";
        let val = b"spacejam\x00";

        rocksdb_put(db, writeopts.clone(), key.as_ptr(), 4, val.as_ptr(), 8, err);
        rocksdb_writeoptions_destroy(writeopts);
        assert!(err.is_null());

        let readopts = rocksdb_readoptions_create();
        assert!(!readopts.0.is_null());

        let val_len: size_t = 0;
        let val_len_ptr = &val_len as *const size_t;
        rocksdb_get(db, readopts.clone(), key.as_ptr(), 4, val_len_ptr, err);
        rocksdb_readoptions_destroy(readopts);
        assert!(err.is_null());
        rocksdb_close(db);
        rocksdb_destroy_db(opts, cpath_ptr, err);
        assert!(err.is_null());
    }
}
