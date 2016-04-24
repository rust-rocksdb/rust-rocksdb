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
use self::libc::c_int;
use std::ffi::CString;
use std::mem;

use rocksdb_ffi::{self, DBCompressionType};
use merge_operator::{self, MergeFn, MergeOperatorCallback,
                     full_merge_callback, partial_merge_callback};
use comparator::{self, ComparatorCallback, compare_callback};

pub struct BlockBasedOptions {
    inner: rocksdb_ffi::DBBlockBasedTableOptions,
}

pub struct Options {
    pub inner: rocksdb_ffi::DBOptions,
}

pub struct WriteOptions {
    pub inner: rocksdb_ffi::DBWriteOptions,
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
    pub fn set_block_size(&mut self, size: usize) {
        unsafe {
            rocksdb_ffi::rocksdb_block_based_options_set_block_size(self.inner,
                                                                    size);
        }
    }

    pub fn set_lru_cache(&mut self, size: size_t) {
        let cache = rocksdb_ffi::new_cache(size);
        unsafe {
            // because cache is wrapped in shared_ptr, so we don't need to call
            // rocksdb_cache_destroy explicitly.
            rocksdb_ffi::rocksdb_block_based_options_set_block_cache(self.inner, cache);
        }
    }
}

impl Default for BlockBasedOptions {
    fn default() -> BlockBasedOptions {
        let block_opts = unsafe {
            rocksdb_ffi::rocksdb_block_based_options_create()
        };
        if block_opts.is_null() {
            panic!("Could not create rocksdb block based options".to_string());
        }
        BlockBasedOptions { inner: block_opts }
    }
}

impl Options {
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

    pub fn compression(&mut self, t: DBCompressionType) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_compression(self.inner, t);
        }
    }

    pub fn add_merge_operator(&mut self,
                              name: &str,
                              merge_fn: MergeFn) {
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

    pub fn add_comparator(&mut self,
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
            if useit {
                rocksdb_ffi::rocksdb_options_set_use_fsync(self.inner, 1)
            } else {
                rocksdb_ffi::rocksdb_options_set_use_fsync(self.inner, 0)
            }
        }
    }

    pub fn set_bytes_per_sync(&mut self, nbytes: u64) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_bytes_per_sync(self.inner, nbytes);
        }
    }

    pub fn set_disable_data_sync(&mut self, disable: bool) {
        unsafe {
            if disable {
                rocksdb_ffi::rocksdb_options_set_disable_data_sync(self.inner, 1)
            } else {
                rocksdb_ffi::rocksdb_options_set_disable_data_sync(self.inner, 0)
            }
        }
    }

    pub fn allow_os_buffer(&mut self, is_allow: bool) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_allow_os_buffer(self.inner,
                                                             is_allow);
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

    pub fn set_write_buffer_size(&mut self, size: usize) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_write_buffer_size(self.inner,
                                                               size);
        }
    }

    pub fn set_max_bytes_for_level_base(&mut self, size: u64) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_max_bytes_for_level_base(self.inner, size);
        }
    }

    pub fn set_max_bytes_for_level_multiplier(&mut self, mul: i32) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_max_bytes_for_level_multiplier(self.inner, mul);
        }
    }

    pub fn set_target_file_size_base(&mut self, size: u64) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_target_file_size_base(self.inner,
                                                                   size);
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

    pub fn set_filter_deletes(&mut self, filter: bool) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_filter_deletes(self.inner, filter);
        }
    }

    pub fn set_disable_auto_compactions(&mut self, disable: bool) {
        let c_bool = if disable {
            1
        } else {
            0
        };
        unsafe {
            rocksdb_ffi::rocksdb_options_set_disable_auto_compactions(self.inner, c_bool)
        }
    }

    pub fn set_block_based_table_factory(&mut self,
                                         factory: &BlockBasedOptions) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_block_based_table_factory(self.inner, factory.inner);
        }
    }
}

impl Default for Options {
    fn default() -> Options {
        unsafe {
            let opts = rocksdb_ffi::rocksdb_options_create();
            if opts.is_null() {
                panic!("Could not create rocksdb options".to_string());
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
            rocksdb_ffi::rocksdb_writeoptions_set_sync(self.inner, sync);
        }
    }
}

impl Default for WriteOptions {
    fn default() -> WriteOptions {
        let write_opts = unsafe { rocksdb_ffi::rocksdb_writeoptions_create() };
        if write_opts.is_null() {
            panic!("Could not create rocksdb write options".to_string());
        }
        WriteOptions { inner: write_opts }
    }
}
