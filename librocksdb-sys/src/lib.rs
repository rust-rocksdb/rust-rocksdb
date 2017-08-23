// Copyright 2014 Tyler Neely, 2016 Alex Regueiro
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

//! Raw bindings for RocksDB.
//!
//! This is simply a raw interface to the RocksDB C API. It is intended to underpin a higher-level library rather than for direct use.

#![allow(dead_code, non_camel_case_types, non_upper_case_globals, non_snake_case)]

extern crate libc;

#[cfg(test)]
mod test;

use libc::*;

extern "C" {
    // Database operations

    pub fn rocksdb_open(options: *const rocksdb_options_t,
                        name: *const c_char,
                        errptr: *mut *mut c_char)
                        -> *mut rocksdb_t;

    pub fn rocksdb_open_for_read_only(options: *const rocksdb_options_t,
                                      name: *const c_char,
                                      error_if_log_file_exist: c_uchar,
                                      errptr: *mut *mut c_char)
                                      -> *mut rocksdb_t;

    pub fn rocksdb_backup_engine_open(options: *const rocksdb_options_t,
                                      path: *const c_char,
                                      errptr: *mut *mut c_char)
                                      -> *mut rocksdb_backup_engine_t;

    pub fn rocksdb_backup_engine_create_new_backup(be: *mut rocksdb_backup_engine_t,
                                                   db: *mut rocksdb_t,
                                                   errptr: *mut *mut c_char);

    pub fn rocksdb_backup_engine_purge_old_backups(be: *mut rocksdb_backup_engine_t,
                                                   num_backups_to_keep: uint32_t,
                                                   errptr: *mut *mut c_char);

    pub fn rocksdb_restore_options_create() -> *mut rocksdb_restore_options_t;

    pub fn rocksdb_restore_options_destroy(opt: *mut rocksdb_restore_options_t);

    pub fn rocksdb_restore_options_set_keep_log_files(opt: *mut rocksdb_restore_options_t,
                                                      v: c_int);

    pub fn rocksdb_backup_engine_restore_db_from_latest_backup(be: *mut rocksdb_backup_engine_t, db_dir: *const c_char, wal_dir: *const c_char, restore_options: *const rocksdb_restore_options_t, errptr: *mut *mut c_char);

    pub fn rocksdb_backup_engine_get_backup_info(be: *mut rocksdb_backup_engine_t)
                                                 -> *const rocksdb_backup_engine_info_t;

    pub fn rocksdb_backup_engine_info_count(info: *const rocksdb_backup_engine_info_t) -> c_int;

    pub fn rocksdb_backup_engine_info_timestamp(info: *const rocksdb_backup_engine_info_t,
                                                index: c_int)
                                                -> int64_t;

    pub fn rocksdb_backup_engine_info_backup_id(info: *const rocksdb_backup_engine_info_t,
                                                index: c_int)
                                                -> uint32_t;

    pub fn rocksdb_backup_engine_info_size(info: *const rocksdb_backup_engine_info_t,
                                           index: c_int)
                                           -> uint64_t;

    pub fn rocksdb_backup_engine_info_number_files(info: *const rocksdb_backup_engine_info_t,
                                                   index: c_int)
                                                   -> uint32_t;

    pub fn rocksdb_backup_engine_info_destroy(info: *const rocksdb_backup_engine_info_t);

    pub fn rocksdb_backup_engine_close(be: *mut rocksdb_backup_engine_t);

    pub fn rocksdb_open_column_families(options: *const rocksdb_options_t, name: *const c_char, num_column_families: c_int, column_family_names: *const *const c_char, column_family_options: *const *const rocksdb_options_t, column_family_handles: *mut *mut rocksdb_column_family_handle_t, errptr: *mut *mut c_char) -> *mut rocksdb_t;

    pub fn rocksdb_open_for_read_only_column_families(options: *const rocksdb_options_t, name: *const c_char, num_column_families: c_int, column_family_names: *const *const c_char, column_family_options: *const *const rocksdb_options_t, column_family_handles: *mut *mut rocksdb_column_family_handle_t, error_if_log_file_exist: c_uchar, errptr: *mut *mut c_char) -> *mut rocksdb_t;

    pub fn rocksdb_list_column_families(options: *const rocksdb_options_t,
                                        name: *const c_char,
                                        lencf: *mut size_t,
                                        errptr: *mut *mut c_char)
                                        -> *mut *mut c_char;

    pub fn rocksdb_list_column_families_destroy(list: *mut *mut c_char, len: size_t);

    pub fn rocksdb_create_column_family(db: *mut rocksdb_t,
                                        column_family_options: *const rocksdb_options_t,
                                        column_family_name: *const c_char,
                                        errptr: *mut *mut c_char)
                                        -> *mut rocksdb_column_family_handle_t;

    pub fn rocksdb_drop_column_family(db: *mut rocksdb_t,
                                      handle: *mut rocksdb_column_family_handle_t,
                                      errptr: *mut *mut c_char);

    pub fn rocksdb_column_family_handle_destroy(handle: *mut rocksdb_column_family_handle_t);

    pub fn rocksdb_close(db: *mut rocksdb_t);

    pub fn rocksdb_put(db: *mut rocksdb_t,
                       options: *const rocksdb_writeoptions_t,
                       key: *const c_char,
                       keylen: size_t,
                       val: *const c_char,
                       vallen: size_t,
                       errptr: *mut *mut c_char);

    pub fn rocksdb_put_cf(db: *mut rocksdb_t,
                          options: *const rocksdb_writeoptions_t,
                          column_family: *mut rocksdb_column_family_handle_t,
                          key: *const c_char,
                          keylen: size_t,
                          val: *const c_char,
                          vallen: size_t,
                          errptr: *mut *mut c_char);

    pub fn rocksdb_delete(db: *mut rocksdb_t,
                          options: *const rocksdb_writeoptions_t,
                          key: *const c_char,
                          keylen: size_t,
                          errptr: *mut *mut c_char);

    pub fn rocksdb_delete_cf(db: *mut rocksdb_t,
                             options: *const rocksdb_writeoptions_t,
                             column_family: *mut rocksdb_column_family_handle_t,
                             key: *const c_char,
                             keylen: size_t,
                             errptr: *mut *mut c_char);

    pub fn rocksdb_merge(db: *mut rocksdb_t,
                         options: *const rocksdb_writeoptions_t,
                         key: *const c_char,
                         keylen: size_t,
                         val: *const c_char,
                         vallen: size_t,
                         errptr: *mut *mut c_char);

    pub fn rocksdb_merge_cf(db: *mut rocksdb_t,
                            options: *const rocksdb_writeoptions_t,
                            column_family: *mut rocksdb_column_family_handle_t,
                            key: *const c_char,
                            keylen: size_t,
                            val: *const c_char,
                            vallen: size_t,
                            errptr: *mut *mut c_char);

    pub fn rocksdb_write(db: *mut rocksdb_t,
                         options: *const rocksdb_writeoptions_t,
                         batch: *mut rocksdb_writebatch_t,
                         errptr: *mut *mut c_char);

    pub fn rocksdb_get(db: *mut rocksdb_t,
                       options: *const rocksdb_readoptions_t,
                       key: *const c_char,
                       keylen: size_t,
                       vallen: *mut size_t,
                       errptr: *mut *mut c_char)
                       -> *mut c_char;

    pub fn rocksdb_get_cf(db: *mut rocksdb_t,
                          options: *const rocksdb_readoptions_t,
                          column_family: *mut rocksdb_column_family_handle_t,
                          key: *const c_char,
                          keylen: size_t,
                          vallen: *mut size_t,
                          errptr: *mut *mut c_char)
                          -> *mut c_char;

    pub fn rocksdb_multi_get(db: *mut rocksdb_t,
                             options: *const rocksdb_readoptions_t,
                             num_keys: size_t,
                             keys_list: *const *const c_char,
                             keys_list_sizes: *const size_t,
                             values_list: *mut *mut c_char,
                             values_list_sizes: *mut size_t,
                             errs: *mut *mut c_char);

    pub fn rocksdb_multi_get_cf(db: *mut rocksdb_t,
                                options: *const rocksdb_readoptions_t,
                                column_families: *const *const rocksdb_column_family_handle_t,
                                num_keys: size_t,
                                keys_list: *const *const c_char,
                                keys_list_sizes: *const size_t,
                                values_list: *mut *mut c_char,
                                values_list_sizes: *mut size_t,
                                errs: *mut *mut c_char);

    pub fn rocksdb_create_iterator(db: *mut rocksdb_t,
                                   options: *const rocksdb_readoptions_t)
                                   -> *mut rocksdb_iterator_t;

    pub fn rocksdb_create_iterator_cf(db: *mut rocksdb_t,
                                      options: *const rocksdb_readoptions_t,
                                      column_family: *mut rocksdb_column_family_handle_t)
                                      -> *mut rocksdb_iterator_t;

    pub fn rocksdb_create_iterators(db: *mut rocksdb_t,
                                    opts: *mut rocksdb_readoptions_t,
                                    column_families: *mut *mut rocksdb_column_family_handle_t,
                                    iterators: *mut *mut rocksdb_iterator_t,
                                    size: size_t,
                                    errptr: *mut *mut c_char);

    pub fn rocksdb_create_snapshot(db: *mut rocksdb_t) -> *const rocksdb_snapshot_t;

    pub fn rocksdb_release_snapshot(db: *mut rocksdb_t, snapshot: *const rocksdb_snapshot_t);

    pub fn rocksdb_property_value(db: *mut rocksdb_t, propname: *const c_char) -> *mut c_char;

    pub fn rocksdb_property_value_cf(db: *mut rocksdb_t,
                                     column_family: *mut rocksdb_column_family_handle_t,
                                     propname: *const c_char)
                                     -> *mut c_char;

    pub fn rocksdb_approximate_sizes(db: *mut rocksdb_t,
                                     num_ranges: c_int,
                                     range_start_key: *const *const c_char,
                                     range_start_key_len: *const size_t,
                                     range_limit_key: *const *const c_char,
                                     range_limit_key_len: *const size_t,
                                     sizes: *mut uint64_t);

    pub fn rocksdb_approximate_sizes_cf(db: *mut rocksdb_t,
                                        column_family: *mut rocksdb_column_family_handle_t,
                                        num_ranges: c_int,
                                        range_start_key: *const *const c_char,
                                        range_start_key_len: *const size_t,
                                        range_limit_key: *const *const c_char,
                                        range_limit_key_len: *const size_t,
                                        sizes: *mut uint64_t);

    pub fn rocksdb_compact_range(db: *mut rocksdb_t,
                                 start_key: *const c_char,
                                 start_key_len: size_t,
                                 limit_key: *const c_char,
                                 limit_key_len: size_t);

    pub fn rocksdb_compact_range_cf(db: *mut rocksdb_t,
                                    column_family: *mut rocksdb_column_family_handle_t,
                                    start_key: *const c_char,
                                    start_key_len: size_t,
                                    limit_key: *const c_char,
                                    limit_key_len: size_t);

    pub fn rocksdb_delete_file(db: *mut rocksdb_t, name: *const c_char);

    pub fn rocksdb_livefiles(db: *mut rocksdb_t) -> *const rocksdb_livefiles_t;

    pub fn rocksdb_flush(db: *mut rocksdb_t,
                         options: *const rocksdb_flushoptions_t,
                         errptr: *mut *mut c_char);

    pub fn rocksdb_disable_file_deletions(db: *mut rocksdb_t, errptr: *mut *mut c_char);

    pub fn rocksdb_enable_file_deletions(db: *mut rocksdb_t,
                                         force: c_uchar,
                                         errptr: *mut *mut c_char);

    // Management operations

    pub fn rocksdb_destroy_db(options: *const rocksdb_options_t,
                              name: *const c_char,
                              errptr: *mut *mut c_char);

    pub fn rocksdb_repair_db(options: *const rocksdb_options_t,
                             name: *const c_char,
                             errptr: *mut *mut c_char);

    // Iterator

    pub fn rocksdb_iter_destroy(iterator: *mut rocksdb_iterator_t);

    pub fn rocksdb_iter_valid(iterator: *const rocksdb_iterator_t) -> c_uchar;

    pub fn rocksdb_iter_seek_to_first(iterator: *mut rocksdb_iterator_t);

    pub fn rocksdb_iter_seek_to_last(iterator: *mut rocksdb_iterator_t);

    pub fn rocksdb_iter_seek(iterator: *mut rocksdb_iterator_t, k: *const c_char, klen: size_t);

    pub fn rocksdb_iter_seek_for_prev(iterator: *mut rocksdb_iterator_t, k: *const c_char, klen: size_t);

    pub fn rocksdb_iter_next(iterator: *mut rocksdb_iterator_t);

    pub fn rocksdb_iter_prev(iterator: *mut rocksdb_iterator_t);

    pub fn rocksdb_iter_key(iterator: *const rocksdb_iterator_t,
                            klen: *mut size_t)
                            -> *const c_char;

    pub fn rocksdb_iter_value(iterator: *const rocksdb_iterator_t,
                              vlen: *mut size_t)
                              -> *const c_char;

    pub fn rocksdb_iter_get_error(iterator: *const rocksdb_iterator_t, errptr: *mut *mut c_char);

    // Write batch

    pub fn rocksdb_writebatch_create() -> *mut rocksdb_writebatch_t;

    pub fn rocksdb_writebatch_create_from(rep: *const c_char,
                                          size: size_t)
                                          -> *mut rocksdb_writebatch_t;

    pub fn rocksdb_writebatch_destroy(batch: *mut rocksdb_writebatch_t);

    pub fn rocksdb_writebatch_clear(batch: *mut rocksdb_writebatch_t);

    pub fn rocksdb_writebatch_count(batch: *mut rocksdb_writebatch_t) -> c_int;

    pub fn rocksdb_writebatch_put(batch: *mut rocksdb_writebatch_t,
                                  key: *const c_char,
                                  klen: size_t,
                                  val: *const c_char,
                                  vlen: size_t);

    pub fn rocksdb_writebatch_put_cf(batch: *mut rocksdb_writebatch_t,
                                     column_family: *mut rocksdb_column_family_handle_t,
                                     key: *const c_char,
                                     klen: size_t,
                                     val: *const c_char,
                                     vlen: size_t);

    pub fn rocksdb_writebatch_putv(b: *mut rocksdb_writebatch_t,
                                   num_keys: c_int,
                                   keys_list: *const *const c_char,
                                   keys_list_sizes: *const size_t,
                                   num_values: c_int,
                                   values_list: *const *const c_char,
                                   values_list_sizes: *const size_t);

    pub fn rocksdb_writebatch_putv_cf(b: *mut rocksdb_writebatch_t,
                                      column_family: *mut rocksdb_column_family_handle_t,
                                      num_keys: c_int,
                                      keys_list: *const *const c_char,
                                      keys_list_sizes: *const size_t,
                                      num_values: c_int,
                                      values_list: *const *const c_char,
                                      values_list_sizes: *const size_t);

    pub fn rocksdb_writebatch_merge(batch: *mut rocksdb_writebatch_t,
                                    key: *const c_char,
                                    klen: size_t,
                                    val: *const c_char,
                                    vlen: size_t);

    pub fn rocksdb_writebatch_merge_cf(batch: *mut rocksdb_writebatch_t,
                                       column_family: *mut rocksdb_column_family_handle_t,
                                       key: *const c_char,
                                       klen: size_t,
                                       val: *const c_char,
                                       vlen: size_t);

    pub fn rocksdb_writebatch_mergev(b: *mut rocksdb_writebatch_t,
                                     num_keys: c_int,
                                     keys_list: *const *const c_char,
                                     keys_list_sizes: *const size_t,
                                     num_values: c_int,
                                     values_list: *const *const c_char,
                                     values_list_sizes: *const size_t);

    pub fn rocksdb_writebatch_mergev_cf(b: *mut rocksdb_writebatch_t,
                                        column_family: *mut rocksdb_column_family_handle_t,
                                        num_keys: c_int,
                                        keys_list: *const *const c_char,
                                        keys_list_sizes: *const size_t,
                                        num_values: c_int,
                                        values_list: *const *const c_char,
                                        values_list_sizes: *const size_t);

    pub fn rocksdb_writebatch_delete(batch: *mut rocksdb_writebatch_t,
                                     key: *const c_char,
                                     klen: size_t);

    pub fn rocksdb_writebatch_delete_cf(batch: *mut rocksdb_writebatch_t,
                                        column_family: *mut rocksdb_column_family_handle_t,
                                        key: *const c_char,
                                        klen: size_t);

    pub fn rocksdb_writebatch_deletev(b: *mut rocksdb_writebatch_t,
                                      num_keys: c_int,
                                      keys_list: *const *const c_char,
                                      keys_list_sizes: *const size_t);

    pub fn rocksdb_writebatch_deletev_cf(b: *mut rocksdb_writebatch_t,
                                         column_family: *mut rocksdb_column_family_handle_t,
                                         num_keys: c_int,
                                         keys_list: *const *const c_char,
                                         keys_list_sizes: *const size_t);

    pub fn rocksdb_writebatch_put_log_data(batch: *mut rocksdb_writebatch_t,
                                           blob: *const c_char,
                                           len: size_t);

    pub fn rocksdb_writebatch_iterate(batch: *mut rocksdb_writebatch_t,
                                      state: *mut c_void,
                                      put: Option<unsafe extern "C" fn(state: *mut c_void,
                                                                       k: *const c_char,
                                                                       klen: size_t,
                                                                       v: *const c_char,
                                                                       vlen: size_t)>,
                                      deleted: Option<unsafe extern "C" fn(state: *mut c_void,
                                                                           k: *const c_char,
                                                                           klen: size_t)>);

    pub fn rocksdb_writebatch_data(batch: *mut rocksdb_writebatch_t,
                                   size: *mut size_t)
                                   -> *const c_char;

    // Block-based table options

    pub fn rocksdb_block_based_options_create() -> *mut rocksdb_block_based_table_options_t;

    pub fn rocksdb_block_based_options_destroy(options: *mut rocksdb_block_based_table_options_t);

    pub fn rocksdb_block_based_options_set_block_size(options: *mut rocksdb_block_based_table_options_t, v: size_t);

    pub fn rocksdb_block_based_options_set_block_size_deviation(options: *mut rocksdb_block_based_table_options_t, v: c_int);

    pub fn rocksdb_block_based_options_set_block_restart_interval(options: *mut rocksdb_block_based_table_options_t, v: c_int);

    pub fn rocksdb_block_based_options_set_filter_policy(options: *mut rocksdb_block_based_table_options_t, v: *mut rocksdb_filterpolicy_t);

    pub fn rocksdb_block_based_options_set_no_block_cache(options: *mut rocksdb_block_based_table_options_t, v: c_uchar);

    pub fn rocksdb_block_based_options_set_block_cache(options: *mut rocksdb_block_based_table_options_t, v: *mut rocksdb_cache_t);

    pub fn rocksdb_block_based_options_set_block_cache_compressed(options: *mut rocksdb_block_based_table_options_t, v: *mut rocksdb_cache_t);

    pub fn rocksdb_block_based_options_set_whole_key_filtering(options: *mut rocksdb_block_based_table_options_t, v: c_uchar);

    pub fn rocksdb_block_based_options_set_format_version(options: *mut rocksdb_block_based_table_options_t, v: c_int);

    pub fn rocksdb_block_based_options_set_index_type(options: *mut rocksdb_block_based_table_options_t, v: c_int);

    pub fn rocksdb_block_based_options_set_hash_index_allow_collision(options: *mut rocksdb_block_based_table_options_t, v: c_uchar);

    pub fn rocksdb_block_based_options_set_cache_index_and_filter_blocks(options: *mut rocksdb_block_based_table_options_t, v: c_uchar);

    pub fn rocksdb_block_based_options_set_pin_l0_filter_and_index_blocks_in_cache
        (options: *mut rocksdb_block_based_table_options_t,
         v: c_uchar);

    pub fn rocksdb_block_based_options_set_skip_table_builder_flush(options: *mut rocksdb_block_based_table_options_t, skip_table_builder_flush: c_uchar);

    pub fn rocksdb_options_set_block_based_table_factory(opt: *mut rocksdb_options_t, table_options: *mut rocksdb_block_based_table_options_t);

    // Cuckoo table options

    pub fn rocksdb_cuckoo_options_create() -> *mut rocksdb_cuckoo_table_options_t;

    pub fn rocksdb_cuckoo_options_destroy(options: *mut rocksdb_cuckoo_table_options_t);

    pub fn rocksdb_cuckoo_options_set_hash_ratio(options: *mut rocksdb_cuckoo_table_options_t,
                                                 v: f64);

    pub fn rocksdb_cuckoo_options_set_max_search_depth(options: *mut rocksdb_cuckoo_table_options_t,
                                                       v: uint32_t);

    pub fn rocksdb_cuckoo_options_set_cuckoo_block_size(options: *mut rocksdb_cuckoo_table_options_t, v: uint32_t);

    pub fn rocksdb_cuckoo_options_set_identity_as_first_hash(options: *mut rocksdb_cuckoo_table_options_t, v: c_uchar);

    pub fn rocksdb_cuckoo_options_set_use_module_hash(options: *mut rocksdb_cuckoo_table_options_t,
                                                      v: c_uchar);

    pub fn rocksdb_options_set_cuckoo_table_factory(opt: *mut rocksdb_options_t, table_options: *mut rocksdb_cuckoo_table_options_t);

    // Options

    pub fn rocksdb_options_create() -> *mut rocksdb_options_t;

    pub fn rocksdb_options_destroy(opt: *mut rocksdb_options_t);

    pub fn rocksdb_options_increase_parallelism(opt: *mut rocksdb_options_t,
                                                total_threads: c_int);

    pub fn rocksdb_options_optimize_for_point_lookup(opt: *mut rocksdb_options_t,
                                                     block_cache_size_mb: uint64_t);

    pub fn rocksdb_options_optimize_level_style_compaction(opt: *mut rocksdb_options_t,
                                                           memtable_memory_budget: uint64_t);

    pub fn rocksdb_options_optimize_universal_style_compaction(opt: *mut rocksdb_options_t,
                                                               memtable_memory_budget: uint64_t);

    pub fn rocksdb_options_set_compaction_filter(opt: *mut rocksdb_options_t,
                                                 filter: *mut rocksdb_compactionfilter_t);

    pub fn rocksdb_options_set_compaction_filter_factory(opt: *mut rocksdb_options_t, factory: *mut rocksdb_compactionfilterfactory_t);

    pub fn rocksdb_options_compaction_readahead_size(opt: *mut rocksdb_options_t, s: size_t);

    pub fn rocksdb_options_set_comparator(opt: *mut rocksdb_options_t,
                                          cmp: *mut rocksdb_comparator_t);

    pub fn rocksdb_options_set_merge_operator(opt: *mut rocksdb_options_t,
                                              merge_operator: *mut rocksdb_mergeoperator_t);

    pub fn rocksdb_options_set_uint64add_merge_operator(opt: *mut rocksdb_options_t);

    pub fn rocksdb_options_set_compression_per_level(opt: *mut rocksdb_options_t,
                                                     level_values: *const c_int,
                                                     num_levels: size_t);

    pub fn rocksdb_options_set_create_if_missing(opt: *mut rocksdb_options_t, v: c_uchar);

    pub fn rocksdb_options_set_create_missing_column_families(opt: *mut rocksdb_options_t,
                                                              v: c_uchar);

    pub fn rocksdb_options_set_error_if_exists(opt: *mut rocksdb_options_t, v: c_uchar);

    pub fn rocksdb_options_set_paranoid_checks(opt: *mut rocksdb_options_t, v: c_uchar);

    pub fn rocksdb_options_set_env(opt: *mut rocksdb_options_t, env: *mut rocksdb_env_t);

    pub fn rocksdb_options_set_info_log(opt: *mut rocksdb_options_t, l: *mut rocksdb_logger_t);

    pub fn rocksdb_options_set_info_log_level(opt: *mut rocksdb_options_t, v: c_int);

    pub fn rocksdb_options_set_write_buffer_size(opt: *mut rocksdb_options_t, s: size_t);

    pub fn rocksdb_options_set_db_write_buffer_size(opt: *mut rocksdb_options_t, s: size_t);

    pub fn rocksdb_options_set_max_open_files(opt: *mut rocksdb_options_t, n: c_int);

    pub fn rocksdb_options_set_max_total_wal_size(opt: *mut rocksdb_options_t, n: uint64_t);

    pub fn rocksdb_options_set_compression_options(opt: *mut rocksdb_options_t,
                                                   w_bits: c_int,
                                                   level: c_int,
                                                   strategy: c_int,
                                                   max_dict_bytes: c_int);

    pub fn rocksdb_options_set_prefix_extractor(opt: *mut rocksdb_options_t,
                                                prefix_extractor: *mut rocksdb_slicetransform_t);

    pub fn rocksdb_options_set_num_levels(opt: *mut rocksdb_options_t, n: c_int);

    pub fn rocksdb_options_set_level0_file_num_compaction_trigger(opt: *mut rocksdb_options_t,
                                                                  n: c_int);

    pub fn rocksdb_options_set_level0_slowdown_writes_trigger(opt: *mut rocksdb_options_t,
                                                              n: c_int);

    pub fn rocksdb_options_set_level0_stop_writes_trigger(opt: *mut rocksdb_options_t, n: c_int);

    pub fn rocksdb_options_set_max_mem_compaction_level(opt: *mut rocksdb_options_t, n: c_int);

    pub fn rocksdb_options_set_target_file_size_base(opt: *mut rocksdb_options_t, n: uint64_t);

    pub fn rocksdb_options_set_target_file_size_multiplier(opt: *mut rocksdb_options_t, n: c_int);

    pub fn rocksdb_options_set_max_bytes_for_level_base(opt: *mut rocksdb_options_t, n: uint64_t);

    pub fn rocksdb_options_set_max_bytes_for_level_multiplier(opt: *mut rocksdb_options_t,
                                                              n: c_int);

    pub fn rocksdb_options_set_expanded_compaction_factor(opt: *mut rocksdb_options_t, v: c_int);

    pub fn rocksdb_options_set_max_grandparent_overlap_factor(opt: *mut rocksdb_options_t,
                                                              v: c_int);

    pub fn rocksdb_options_set_max_bytes_for_level_multiplier_additional(opt: *mut rocksdb_options_t, level_values: *mut c_int, num_levels: size_t);

    pub fn rocksdb_options_enable_statistics(opt: *mut rocksdb_options_t);

    pub fn rocksdb_options_statistics_get_string(opt: *mut rocksdb_options_t) -> *mut c_char;

    pub fn rocksdb_options_set_max_write_buffer_number(opt: *mut rocksdb_options_t, n: c_int);

    pub fn rocksdb_options_set_min_write_buffer_number_to_merge(opt: *mut rocksdb_options_t,
                                                                n: c_int);

    pub fn rocksdb_options_set_max_write_buffer_number_to_maintain(opt: *mut rocksdb_options_t,
                                                                   n: c_int);

    pub fn rocksdb_options_set_max_background_compactions(opt: *mut rocksdb_options_t, n: c_int);

    pub fn rocksdb_options_set_max_background_flushes(opt: *mut rocksdb_options_t, n: c_int);

    pub fn rocksdb_options_set_max_log_file_size(opt: *mut rocksdb_options_t, v: size_t);

    pub fn rocksdb_options_set_log_file_time_to_roll(opt: *mut rocksdb_options_t, v: size_t);

    pub fn rocksdb_options_set_keep_log_file_num(opt: *mut rocksdb_options_t, v: size_t);

    pub fn rocksdb_options_set_recycle_log_file_num(opt: *mut rocksdb_options_t, v: size_t);

    pub fn rocksdb_options_set_soft_rate_limit(opt: *mut rocksdb_options_t, v: f64);

    pub fn rocksdb_options_set_hard_rate_limit(opt: *mut rocksdb_options_t, v: f64);

    pub fn rocksdb_options_set_rate_limit_delay_max_milliseconds(opt: *mut rocksdb_options_t,
                                                                 v: c_uint);

    pub fn rocksdb_options_set_max_manifest_file_size(opt: *mut rocksdb_options_t, v: size_t);

    pub fn rocksdb_options_set_table_cache_numshardbits(opt: *mut rocksdb_options_t, v: c_int);

    pub fn rocksdb_options_set_table_cache_remove_scan_count_limit(opt: *mut rocksdb_options_t,
                                                                   v: c_int);

    pub fn rocksdb_options_set_arena_block_size(opt: *mut rocksdb_options_t, v: size_t);

    pub fn rocksdb_options_set_use_fsync(opt: *mut rocksdb_options_t, v: c_int);

    pub fn rocksdb_options_set_db_log_dir(opt: *mut rocksdb_options_t, v: *const c_char);

    pub fn rocksdb_options_set_wal_dir(opt: *mut rocksdb_options_t, v: *const c_char);

    pub fn rocksdb_options_set_WAL_ttl_seconds(opt: *mut rocksdb_options_t, ttl: uint64_t);

    pub fn rocksdb_options_set_WAL_size_limit_MB(opt: *mut rocksdb_options_t, limit: uint64_t);

    pub fn rocksdb_options_set_manifest_preallocation_size(opt: *mut rocksdb_options_t,
                                                           v: size_t);

    pub fn rocksdb_options_set_purge_redundant_kvs_while_flush(opt: *mut rocksdb_options_t,
                                                               v: c_uchar);

    pub fn rocksdb_options_set_use_direct_reads(opt: *mut rocksdb_options_t, v: c_uchar);

    pub fn rocksdb_options_set_use_direct_io_for_flush_and_compaction(opt: *mut rocksdb_options_t,
                                                                      v: c_uchar);

    pub fn rocksdb_options_set_allow_mmap_reads(opt: *mut rocksdb_options_t, v: c_uchar);

    pub fn rocksdb_options_set_allow_mmap_writes(opt: *mut rocksdb_options_t, v: c_uchar);

    pub fn rocksdb_options_set_is_fd_close_on_exec(opt: *mut rocksdb_options_t, v: c_uchar);

    pub fn rocksdb_options_set_skip_log_error_on_recovery(opt: *mut rocksdb_options_t,
                                                          v: c_uchar);

    pub fn rocksdb_options_set_stats_dump_period_sec(opt: *mut rocksdb_options_t, v: c_uint);

    pub fn rocksdb_options_set_advise_random_on_open(opt: *mut rocksdb_options_t, v: c_uchar);

    pub fn rocksdb_options_set_access_hint_on_compaction_start(opt: *mut rocksdb_options_t,
                                                               v: c_int);

    pub fn rocksdb_options_set_use_adaptive_mutex(opt: *mut rocksdb_options_t, v: c_uchar);

    pub fn rocksdb_options_set_bytes_per_sync(opt: *mut rocksdb_options_t, v: uint64_t);

    pub fn rocksdb_options_set_allow_concurrent_memtable_write(opt: *mut rocksdb_options_t,
                                                               v: c_uchar);

    pub fn rocksdb_options_set_verify_checksums_in_compaction(opt: *mut rocksdb_options_t,
                                                              v: c_uchar);

    pub fn rocksdb_options_set_filter_deletes(opt: *mut rocksdb_options_t, v: c_uchar);

    pub fn rocksdb_options_set_max_sequential_skip_in_iterations(opt: *mut rocksdb_options_t,
                                                                 v: uint64_t);

    pub fn rocksdb_options_set_disable_auto_compactions(opt: *mut rocksdb_options_t, v: c_int);

    pub fn rocksdb_options_set_delete_obsolete_files_period_micros(opt: *mut rocksdb_options_t,
                                                                   v: uint64_t);

    pub fn rocksdb_options_set_source_compaction_factor(opt: *mut rocksdb_options_t, v: c_int);

    pub fn rocksdb_options_prepare_for_bulk_load(opt: *mut rocksdb_options_t);

    pub fn rocksdb_options_set_memtable_vector_rep(opt: *mut rocksdb_options_t);

    pub fn rocksdb_options_set_hash_skip_list_rep(opt: *mut rocksdb_options_t,
                                                  bucket_count: size_t,
                                                  skiplist_height: int32_t,
                                                  skiplist_branching_factor: int32_t);

    pub fn rocksdb_options_set_hash_link_list_rep(opt: *mut rocksdb_options_t,
                                                  bucket_count: size_t);

    pub fn rocksdb_options_set_plain_table_factory(opt: *mut rocksdb_options_t,
                                                   user_key_len: uint32_t,
                                                   bloom_bits_per_key: c_int,
                                                   hash_table_ratio: f64,
                                                   index_sparseness: size_t);

    pub fn rocksdb_options_set_min_level_to_compress(opt: *mut rocksdb_options_t, level: c_int);

    pub fn rocksdb_options_set_memtable_prefix_bloom_bits(opt: *mut rocksdb_options_t,
                                                          v: uint32_t);

    pub fn rocksdb_options_set_memtable_prefix_bloom_probes(opt: *mut rocksdb_options_t,
                                                            v: uint32_t);

    pub fn rocksdb_options_set_memtable_huge_page_size(opt: *mut rocksdb_options_t, v: size_t);

    pub fn rocksdb_options_set_max_successive_merges(opt: *mut rocksdb_options_t, v: size_t);

    pub fn rocksdb_options_set_min_partial_merge_operands(opt: *mut rocksdb_options_t,
                                                          v: uint32_t);

    pub fn rocksdb_options_set_bloom_locality(opt: *mut rocksdb_options_t, v: uint32_t);

    pub fn rocksdb_options_set_inplace_update_support(opt: *mut rocksdb_options_t, v: c_uchar);

    pub fn rocksdb_options_set_inplace_update_num_locks(opt: *mut rocksdb_options_t, v: size_t);

    pub fn rocksdb_options_set_report_bg_io_stats(opt: *mut rocksdb_options_t, v: c_int);

    pub fn rocksdb_options_set_wal_recovery_mode(opt: *mut rocksdb_options_t, v: c_int);

    pub fn rocksdb_options_set_compression(opt: *mut rocksdb_options_t, t: c_int);

    pub fn rocksdb_options_set_compaction_style(opt: *mut rocksdb_options_t, style: c_int);

    pub fn rocksdb_options_set_universal_compaction_options(opt: *mut rocksdb_options_t, uco: *mut rocksdb_universal_compaction_options_t);

    pub fn rocksdb_options_set_fifo_compaction_options(opt: *mut rocksdb_options_t,
                                                       fifo: *mut rocksdb_fifo_compaction_options_t);

    // Compaction filter

    pub fn rocksdb_compactionfilter_create
        (state: *mut c_void,
         destructor: Option<unsafe extern "C" fn(state: *mut c_void)>,
         filter: Option<unsafe extern "C" fn(state: *mut c_void,
                                             level: c_int,
                                             key: *const c_char,
                                             key_length: size_t,
                                             existing_value: *const c_char,
                                             value_length: size_t,
                                             new_value: *mut *mut c_char,
                                             new_value_length: *mut size_t,
                                             value_changed: *mut c_uchar)
                                             -> c_uchar>,
         name: Option<unsafe extern "C" fn(state: *mut c_void) -> *const c_char>)
         -> *mut rocksdb_compactionfilter_t;

    pub fn rocksdb_compactionfilter_set_ignore_snapshots(filter: *mut rocksdb_compactionfilter_t,
                                                         v: c_uchar);

    pub fn rocksdb_compactionfilter_destroy(filter: *mut rocksdb_compactionfilter_t);

    // Compaction Filter context

    pub fn rocksdb_compactionfiltercontext_is_full_compaction(context: *mut rocksdb_compactionfiltercontext_t) -> c_uchar;

    pub fn rocksdb_compactionfiltercontext_is_manual_compaction(context: *mut rocksdb_compactionfiltercontext_t) -> c_uchar;

    // Compaction Filter factory

    pub fn rocksdb_compactionfilterfactory_create(state: *mut c_void, destructor: Option<unsafe extern "C" fn(state: *mut c_void)>, create_compaction_filter: Option<unsafe extern "C" fn(state: *mut c_void, context: *mut rocksdb_compactionfiltercontext_t) -> *mut rocksdb_compactionfilter_t>, name: Option<unsafe extern "C" fn(state: *mut c_void) -> *const c_char>) -> *mut rocksdb_compactionfilterfactory_t;

    pub fn rocksdb_compactionfilterfactory_destroy(factory: *mut rocksdb_compactionfilterfactory_t);

    // Comparator

    pub fn rocksdb_comparator_create(state: *mut c_void,
                                     destructor: Option<unsafe extern "C" fn(state: *mut c_void)>,
                                     compare: Option<unsafe extern "C" fn(state: *mut c_void,
                                                                          a: *const c_char,
                                                                          alen: size_t,
                                                                          b: *const c_char,
                                                                          blen: size_t)
                                                                          -> c_int>,
                                     name: Option<unsafe extern "C" fn(state: *mut c_void)
                                                                       -> *const c_char>)
                                     -> *mut rocksdb_comparator_t;

    pub fn rocksdb_comparator_destroy(cmp: *mut rocksdb_comparator_t);

    // Filter policy

    pub fn rocksdb_filterpolicy_create(state: *mut c_void, destructor: Option<unsafe extern "C" fn(state: *mut c_void)>, create_filter: Option<unsafe extern "C" fn(state: *mut c_void, key_array: *const *const c_char, key_length_array: *const size_t, num_keys: c_int, filter_length: *mut size_t) -> *mut c_char>, key_may_match: Option<unsafe extern "C" fn(state: *mut c_void, key: *const c_char, length: size_t, filter: *const c_char, filter_length: size_t) -> c_uchar>, delete_filter: Option<unsafe extern "C" fn(state: *mut c_void, filter: *const c_char, filter_length: size_t)>, name: Option<unsafe extern "C" fn(state: *mut c_void) -> *const c_char>) -> *mut rocksdb_filterpolicy_t;

    pub fn rocksdb_filterpolicy_destroy(filter: *mut rocksdb_filterpolicy_t);

    pub fn rocksdb_filterpolicy_create_bloom(bits_per_key: c_int) -> *mut rocksdb_filterpolicy_t;

    pub fn rocksdb_filterpolicy_create_bloom_full(bits_per_key: c_int) -> *mut rocksdb_filterpolicy_t;

    // Merge Operator

    pub fn rocksdb_mergeoperator_create(state: *mut c_void, destructor: Option<unsafe extern "C" fn(state: *mut c_void)>, full_merge: Option<unsafe extern "C" fn(state: *mut c_void, key: *const c_char, key_length: size_t, existing_value: *const c_char, existing_value_length: size_t, operands_list: *const *const c_char, operands_list_length: *const size_t, num_operands: c_int, success: *mut c_uchar, new_value_length: *mut size_t) -> *mut c_char>, partial_merge: Option<unsafe extern "C" fn(state: *mut c_void, key: *const c_char, key_length: size_t, operands_list: *const *const c_char, operands_list_length: *const size_t, num_operands: c_int, success: *mut c_uchar, new_value_length: *mut size_t) -> *mut c_char>, delete_value: Option<unsafe extern "C" fn(state: *mut c_void, value: *const c_char, value_length: size_t)>, name: Option<unsafe extern "C" fn(state: *mut c_void) -> *const c_char>) -> *mut rocksdb_mergeoperator_t;

    pub fn rocksdb_mergeoperator_destroy(merge_operator: *mut rocksdb_mergeoperator_t);

    // Read options

    pub fn rocksdb_readoptions_create() -> *mut rocksdb_readoptions_t;

    pub fn rocksdb_readoptions_destroy(opt: *mut rocksdb_readoptions_t);

    pub fn rocksdb_readoptions_set_verify_checksums(opt: *mut rocksdb_readoptions_t, v: c_uchar);

    pub fn rocksdb_readoptions_set_fill_cache(opt: *mut rocksdb_readoptions_t, v: c_uchar);

    pub fn rocksdb_readoptions_set_snapshot(opt: *mut rocksdb_readoptions_t,
                                            v: *const rocksdb_snapshot_t);

    pub fn rocksdb_readoptions_set_iterate_upper_bound(opt: *mut rocksdb_readoptions_t,
                                                       key: *const c_char,
                                                       keylen: size_t);

    pub fn rocksdb_readoptions_set_read_tier(opt: *mut rocksdb_readoptions_t, v: c_int);

    pub fn rocksdb_readoptions_set_tailing(opt: *mut rocksdb_readoptions_t, v: c_uchar);

    pub fn rocksdb_readoptions_set_readahead_size(opt: *mut rocksdb_readoptions_t, v: size_t);

    // Write options

    pub fn rocksdb_writeoptions_create() -> *mut rocksdb_writeoptions_t;

    pub fn rocksdb_writeoptions_destroy(opt: *mut rocksdb_writeoptions_t);

    pub fn rocksdb_writeoptions_set_sync(opt: *mut rocksdb_writeoptions_t, v: c_uchar);

    pub fn rocksdb_writeoptions_disable_WAL(opt: *mut rocksdb_writeoptions_t, disable: c_int);

    // Flush options

    pub fn rocksdb_flushoptions_create() -> *mut rocksdb_flushoptions_t;

    pub fn rocksdb_flushoptions_destroy(opt: *mut rocksdb_flushoptions_t);

    pub fn rocksdb_flushoptions_set_wait(opt: *mut rocksdb_flushoptions_t, v: c_uchar);

    // Cache

    pub fn rocksdb_cache_create_lru(capacity: size_t) -> *mut rocksdb_cache_t;

    pub fn rocksdb_cache_destroy(cache: *mut rocksdb_cache_t);

    pub fn rocksdb_cache_set_capacity(cache: *mut rocksdb_cache_t, capacity: size_t);

    // Environment

    pub fn rocksdb_create_default_env() -> *mut rocksdb_env_t;

    pub fn rocksdb_create_mem_env() -> *mut rocksdb_env_t;

    pub fn rocksdb_env_set_background_threads(env: *mut rocksdb_env_t, n: c_int);

    pub fn rocksdb_env_set_high_priority_background_threads(env: *mut rocksdb_env_t, n: c_int);

    pub fn rocksdb_env_join_all_threads(env: *mut rocksdb_env_t);

    pub fn rocksdb_env_destroy(env: *mut rocksdb_env_t);

    // Slice Transform

    pub fn rocksdb_slicetransform_create(state: *mut c_void, destructor: Option<unsafe extern "C" fn(state: *mut c_void)>, transform: Option<unsafe extern "C" fn(state: *mut c_void, key: *const c_char, length: size_t, dst_length: *mut size_t) -> *mut c_char>, in_domain: Option<unsafe extern "C" fn(state: *mut c_void, key: *const c_char, length: size_t) -> c_uchar>, in_range: Option<unsafe extern "C" fn(state: *mut c_void, key: *const c_char, length: size_t) -> c_uchar>, name: Option<unsafe extern "C" fn(state: *mut c_void) -> *const c_char>) -> *mut rocksdb_slicetransform_t;

    pub fn rocksdb_slicetransform_create_fixed_prefix(len: size_t) -> *mut rocksdb_slicetransform_t;

    pub fn rocksdb_slicetransform_create_noop() -> *mut rocksdb_slicetransform_t;

    pub fn rocksdb_slicetransform_destroy(st: *mut rocksdb_slicetransform_t);

    // Universal Compaction options

    pub fn rocksdb_universal_compaction_options_create
        ()
        -> *mut rocksdb_universal_compaction_options_t;

    pub fn rocksdb_universal_compaction_options_set_size_ratio(uco: *mut rocksdb_universal_compaction_options_t, ratio: c_int);

    pub fn rocksdb_universal_compaction_options_set_min_merge_width(uco: *mut rocksdb_universal_compaction_options_t, w: c_int);

    pub fn rocksdb_universal_compaction_options_set_max_merge_width(uco: *mut rocksdb_universal_compaction_options_t, w: c_int);

    pub fn rocksdb_universal_compaction_options_set_max_size_amplification_percent
        (uco: *mut rocksdb_universal_compaction_options_t,
         p: c_int);

    pub fn rocksdb_universal_compaction_options_set_compression_size_percent
        (uco: *mut rocksdb_universal_compaction_options_t,
         p: c_int);

    pub fn rocksdb_universal_compaction_options_set_stop_style(uco: *mut rocksdb_universal_compaction_options_t, style: c_int);

    pub fn rocksdb_universal_compaction_options_destroy(uco: *mut rocksdb_universal_compaction_options_t);

    pub fn rocksdb_fifo_compaction_options_create() -> *mut rocksdb_fifo_compaction_options_t;

    pub fn rocksdb_fifo_compaction_options_set_max_table_files_size(fifo_opts: *mut rocksdb_fifo_compaction_options_t, size: uint64_t);

    pub fn rocksdb_fifo_compaction_options_destroy(fifo_opts: *mut rocksdb_fifo_compaction_options_t);

    pub fn rocksdb_livefiles_count(files: *const rocksdb_livefiles_t) -> c_int;

    pub fn rocksdb_livefiles_name(files: *const rocksdb_livefiles_t,
                                  index: c_int)
                                  -> *const c_char;

    pub fn rocksdb_livefiles_level(files: *const rocksdb_livefiles_t, index: c_int) -> c_int;

    pub fn rocksdb_livefiles_size(files: *const rocksdb_livefiles_t, index: c_int) -> size_t;

    pub fn rocksdb_livefiles_smallestkey(files: *const rocksdb_livefiles_t,
                                         index: c_int,
                                         size: *mut size_t)
                                         -> *const c_char;

    pub fn rocksdb_livefiles_largestkey(files: *const rocksdb_livefiles_t,
                                        index: c_int,
                                        size: *mut size_t)
                                        -> *const c_char;

    pub fn rocksdb_livefiles_destroy(files: *const rocksdb_livefiles_t);

    // Utilities

    pub fn rocksdb_get_options_from_string(base_options: *const rocksdb_options_t,
                                           opts_str: *const c_char,
                                           new_options: *mut rocksdb_options_t,
                                           errptr: *mut *mut c_char);

    pub fn rocksdb_delete_file_in_range(db: *mut rocksdb_t,
                                        start_key: *const c_char,
                                        start_key_len: size_t,
                                        limit_key: *const c_char,
                                        limit_key_len: size_t,
                                        errptr: *mut *mut c_char);

    pub fn rocksdb_delete_file_in_range_cf(db: *mut rocksdb_t,
                                           column_family: *mut rocksdb_column_family_handle_t,
                                           start_key: *const c_char,
                                           start_key_len: size_t,
                                           limit_key: *const c_char,
                                           limit_key_len: size_t,
                                           errptr: *mut *mut c_char);

    pub fn rocksdb_free(ptr: *mut c_void);
}

pub const rocksdb_block_based_table_index_type_binary_search: c_int = 0;
pub const rocksdb_block_based_table_index_type_hash_search: c_int = 1;

pub const rocksdb_no_compression: c_int = 0;
pub const rocksdb_snappy_compression: c_int = 1;
pub const rocksdb_zlib_compression: c_int = 2;
pub const rocksdb_bz2_compression: c_int = 3;
pub const rocksdb_lz4_compression: c_int = 4;
pub const rocksdb_lz4hc_compression: c_int = 5;

pub const rocksdb_level_compaction: c_int = 0;
pub const rocksdb_universal_compaction: c_int = 1;
pub const rocksdb_fifo_compaction: c_int = 2;

pub const rocksdb_similar_size_compaction_stop_style: c_int = 0;
pub const rocksdb_total_size_compaction_stop_style: c_int = 1;

pub const rocksdb_recovery_mode_tolerate_corrupted_tail_records: c_int = 0;
pub const rocksdb_recovery_mode_absolute_consistency: c_int = 1;
pub const rocksdb_recovery_mode_point_in_time: c_int = 2;
pub const rocksdb_recovery_mode_skip_any_corrupted_record: c_int = 3;

pub enum rocksdb_t { }

pub enum rocksdb_backup_engine_t { }

pub enum rocksdb_backup_engine_info_t { }

pub enum rocksdb_restore_options_t { }

pub enum rocksdb_cache_t { }

pub enum rocksdb_compactionfilter_t { }

pub enum rocksdb_compactionfiltercontext_t { }

pub enum rocksdb_compactionfilterfactory_t { }

pub enum rocksdb_comparator_t { }

pub enum rocksdb_env_t { }

pub enum rocksdb_fifo_compaction_options_t { }

pub enum rocksdb_filelock_t { }

pub enum rocksdb_filterpolicy_t { }

pub enum rocksdb_flushoptions_t { }

pub enum rocksdb_iterator_t { }

pub enum rocksdb_logger_t { }

pub enum rocksdb_mergeoperator_t { }

pub enum rocksdb_options_t { }

pub enum rocksdb_block_based_table_options_t { }

pub enum rocksdb_cuckoo_table_options_t { }

pub enum rocksdb_randomfile_t { }

pub enum rocksdb_readoptions_t { }

pub enum rocksdb_seqfile_t { }

pub enum rocksdb_slicetransform_t { }

pub enum rocksdb_snapshot_t { }

pub enum rocksdb_writablefile_t { }

pub enum rocksdb_writebatch_t { }

pub enum rocksdb_writeoptions_t { }

pub enum rocksdb_universal_compaction_options_t { }

pub enum rocksdb_livefiles_t { }

pub enum rocksdb_column_family_handle_t { }
