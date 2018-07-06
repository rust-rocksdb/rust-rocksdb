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
use self::libc::{c_int, size_t};
use std::ffi::CString;
use std::mem;

use rocksdb_ffi;
use merge_operator::{self, MergeOperands, MergeOperatorCallback,
                     full_merge_callback, partial_merge_callback};
use comparator::{self, ComparatorCallback, compare_callback};

pub enum IndexType {
    BinarySearch,
    HashSearch,
}

pub struct BlockBasedOptions {
    inner: rocksdb_ffi::DBBlockBasedTableOptions,
    filter: Option<rocksdb_ffi::DBFilterPolicy>,
}

pub struct Options {
    pub inner: rocksdb_ffi::DBOptions,
}

pub struct WriteOptions {
    pub inner: rocksdb_ffi::DBWriteOptions,
}

pub struct Cache {
    pub inner: rocksdb_ffi::DBCache,
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
    pub fn new() -> BlockBasedOptions {
        let block_opts = unsafe {
            rocksdb_ffi::rocksdb_block_based_options_create()
        };
        if block_opts.is_null() {
            panic!("Could not create rocksdb block based options".to_string());
        }
        BlockBasedOptions { inner: block_opts, filter: None }
    }

    pub fn set_block_size(&mut self, size: usize) {
        unsafe {
            rocksdb_ffi::rocksdb_block_based_options_set_block_size(self.inner,
                                                                    size);
        }
    }

    pub fn set_index_type(&mut self, index_type: IndexType) {
        let it = match index_type {
            IndexType::BinarySearch => rocksdb_ffi::BLOCK_BASED_INDEX_TYPE_BINARY_SEARCH,
            IndexType::HashSearch => rocksdb_ffi::BLOCK_BASED_INDEX_TYPE_HASH_SEARCH,
        };
        unsafe {
            rocksdb_ffi::rocksdb_block_based_options_set_index_type(self.inner, it);
        }
    }

    pub fn set_cache(&mut self, cache: Cache) {
        unsafe { rocksdb_ffi::rocksdb_block_based_options_set_block_cache(self.inner, cache.inner); }
    }

    pub fn set_filter(&mut self, bits: i32) {
        unsafe {
            let new_filter =  rocksdb_ffi::rocksdb_filterpolicy_create_bloom(bits);
            rocksdb_ffi::rocksdb_block_based_options_set_filter_policy(self.inner, new_filter);
            self.filter = Some(new_filter);
        }
    }
}

// rocksdb guarantees synchronization
unsafe impl Sync for BlockBasedOptions {}
// rocksdb guarantees synchronization
unsafe impl Send for BlockBasedOptions {}

// TODO figure out how to create these in a Rusty way
// /pub fn set_filter(&mut self, filter: rocksdb_ffi::DBFilterPolicy) {
// /    unsafe {
// /        rocksdb_ffi::rocksdb_block_based_options_set_filter_policy(
// /            self.inner, filter);
// /    }
// /}

/// /pub fn set_cache(&mut self, cache: rocksdb_ffi::DBCache) {
/// /    unsafe {
/// /        rocksdb_ffi::rocksdb_block_based_options_set_block_cache(
/// /            self.inner, cache);
/// /    }
/// /}

/// /pub fn set_cache_compressed(&mut self, cache: rocksdb_ffi::DBCache) {
/// /    unsafe {
/// /        rocksdb_ffi::
/// rocksdb_block_based_options_set_block_cache_compressed(
/// /            self.inner, cache);
/// /    }
/// /}


impl Options {
    pub fn new() -> Options {
        unsafe {
            let opts = rocksdb_ffi::rocksdb_options_create();
            if opts.is_null() {
                panic!("Could not create rocksdb options".to_string());
            }
            Options { inner: opts }
        }
    }

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

    pub fn create_if_missing(&mut self, create_if_missing: bool) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_create_if_missing(
                self.inner, create_if_missing);
        }
    }

    pub fn add_merge_operator<'a>(&mut self,
                                  name: &str,
                                  merge_fn: fn(&[u8],
                                               Option<&[u8]>,
                                               &mut MergeOperands)
                                               -> Vec<u8>) {
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

    pub fn add_comparator<'a>(&mut self,
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

    pub fn set_prefix_extractor_fixed_size<'a>(&mut self, size: usize) {
        unsafe {
            let st = rocksdb_ffi::rocksdb_slicetransform_create_fixed_prefix(size);
            rocksdb_ffi::rocksdb_options_set_prefix_extractor(self.inner, st);
        }
    }

    pub fn set_block_cache_size_mb(&mut self, cache_size: u64) {
        unsafe {
            rocksdb_ffi::rocksdb_options_optimize_for_point_lookup(self.inner,
                                                                   cache_size);
        }
    }

    pub fn set_max_open_files(&mut self, nfiles: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_max_open_files(self.inner, nfiles);
        }
    }

    pub fn set_use_fsync(&mut self, useit: bool) {
        unsafe {
            match useit {
                true => {
                    rocksdb_ffi::rocksdb_options_set_use_fsync(self.inner, 1)
                }
                false => {
                    rocksdb_ffi::rocksdb_options_set_use_fsync(self.inner, 0)
                }
            }
        }
    }

    pub fn set_bytes_per_sync(&mut self, nbytes: u64) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_bytes_per_sync(self.inner, nbytes);
        }
    }

    pub fn set_table_cache_num_shard_bits(&mut self, nbits: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_table_cache_numshardbits(self.inner,
                                                                      nbits);
        }
    }

    pub fn set_min_write_buffer_number(&mut self, nbuf: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_min_write_buffer_number_to_merge(
                self.inner, nbuf);
        }
    }

    pub fn set_max_write_buffer_number(&mut self, nbuf: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_max_write_buffer_number(self.inner,
                                                                     nbuf);
        }
    }

    pub fn set_write_buffer_size(&mut self, size: size_t) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_write_buffer_size(self.inner,
                                                               size);
        }
    }

    pub fn set_db_write_buffer_size(&mut self, size: size_t) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_db_write_buffer_size(self.inner,
                                                               size);
        }
    }

    pub fn set_target_file_size_base(&mut self, size: u64) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_target_file_size_base(self.inner,
                                                                   size);
        }
    }

    pub fn set_target_file_size_multiplier(&mut self, size: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_target_file_size_multiplier(self.inner, size);
        }
    }

    pub fn set_min_write_buffer_number_to_merge(&mut self, to_merge: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_min_write_buffer_number_to_merge(
                self.inner, to_merge);
        }
    }

    pub fn set_level_zero_slowdown_writes_trigger(&mut self, n: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_level0_slowdown_writes_trigger(
                self.inner, n);
        }
    }

    pub fn set_level_zero_stop_writes_trigger(&mut self, n: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_level0_stop_writes_trigger(
                self.inner, n);
        }
    }

    pub fn set_compaction_style(&mut self,
                                style: rocksdb_ffi::DBCompactionStyle) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_compaction_style(self.inner,
                                                              style);
        }
    }

    pub fn set_max_background_compactions(&mut self, n: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_max_background_compactions(
                self.inner, n);
        }
    }

    pub fn set_max_background_flushes(&mut self, n: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_max_background_flushes(self.inner,
                                                                    n);
        }
    }

    pub fn set_disable_auto_compactions(&mut self, disable: bool) {
        unsafe {
            match disable {
                true =>
                    rocksdb_ffi::rocksdb_options_set_disable_auto_compactions(
                        self.inner, 1),
                false =>
                    rocksdb_ffi::rocksdb_options_set_disable_auto_compactions(
                        self.inner, 0),
            }
        }
    }

    pub fn set_block_based_table_factory(&mut self,
                                         factory: &BlockBasedOptions) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_block_based_table_factory(self.inner, factory.inner);
        }
    }

    pub fn set_parsed_options(&mut self, opts: &str) -> Result<(), String> {
        unsafe {
            let new_inner_options = rocksdb_ffi::rocksdb_options_create();
            if new_inner_options.is_null() {
                panic!("Could not create rocksdb options".to_string());
            }

            let mut err: *const i8 = 0 as *const i8;
            let err_ptr: *mut *const i8 = &mut err;

            let c_opts = CString::new(opts.as_bytes()).unwrap();
            let c_opts_ptr = c_opts.as_ptr();

            rocksdb_ffi::rocksdb_get_options_from_string(self.inner, c_opts_ptr as *const _, new_inner_options, err_ptr);
            if !err.is_null() {
                return Err(rocksdb_ffi::error_message(err))
            }
            rocksdb_ffi::rocksdb_options_destroy(mem::replace(&mut self.inner, new_inner_options));
            Ok(())
        }
    }
}

impl WriteOptions {
    pub fn new() -> WriteOptions {
        let write_opts = unsafe { rocksdb_ffi::rocksdb_writeoptions_create() };
        if write_opts.is_null() {
            panic!("Could not create rocksdb write options".to_string());
        }
        WriteOptions { inner: write_opts }
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

// rocksdb guarantees synchronization
unsafe impl Sync for WriteOptions {}
// rocksdb guarantees synchronization
unsafe impl Send for WriteOptions {}

impl Cache {
    pub fn new(bytes: usize) -> Cache {
        Cache { inner: unsafe { rocksdb_ffi::rocksdb_cache_create_lru(bytes) } }
    }
}

impl Drop for Cache {
    fn drop(&mut self) {
        unsafe { rocksdb_ffi::rocksdb_cache_destroy(self.inner); }
    }
}

// rocksdb guarantees synchronization
unsafe impl Sync for Cache {}
// rocksdb guarantees synchronization
unsafe impl Send for Cache {}
