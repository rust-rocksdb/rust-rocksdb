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
use self::libc::{c_int, size_t};
use std::ffi::CString;
use std::mem;

use rocksdb_ffi;
use merge_operator::{MergeOperatorCallback, MergeOperands, destructor_callback, full_merge_callback,
              partial_merge_callback, name_callback};

pub struct RocksDBOptions {
    pub inner: rocksdb_ffi::RocksDBOptions,
    block_options: rocksdb_ffi::RocksDBBlockBasedTableOptions,
}

impl Copy for RocksDBOptions {}

impl RocksDBOptions {
    pub fn new() -> RocksDBOptions {
        unsafe {
            let opts = rocksdb_ffi::rocksdb_options_create();
            let rocksdb_ffi::RocksDBOptions(opt_ptr) = opts;
            if opt_ptr.is_null() {
                panic!("Could not create rocksdb options".to_string());
            }
            let block_opts = rocksdb_ffi::rocksdb_block_based_options_create();

            RocksDBOptions{
                inner: opts,
                block_options: block_opts,
            }
        }
    }

    pub fn increase_parallelism(&self, parallelism: i32) {
        unsafe {
            rocksdb_ffi::rocksdb_options_increase_parallelism(
                self.inner, parallelism);
        }
    }

    pub fn optimize_level_style_compaction(&self,
        memtable_memory_budget: i32) {
        unsafe {
            rocksdb_ffi::rocksdb_options_optimize_level_style_compaction(
                self.inner, memtable_memory_budget);
        }
    }

    pub fn create_if_missing(&self, create_if_missing: bool) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_create_if_missing(
                self.inner, create_if_missing);
        }
    }

    pub fn add_merge_operator<'a>( &self, name: &str,
        merge_fn: fn (&[u8], Option<&[u8]>, &mut MergeOperands) -> Vec<u8>) {
        let cb = Box::new(MergeOperatorCallback {
            name: CString::new(name.as_bytes()).unwrap(),
            merge_fn: merge_fn,
        });

        unsafe {
            let mo = rocksdb_ffi::rocksdb_mergeoperator_create(
                mem::transmute(cb),
                destructor_callback,
                full_merge_callback,
                partial_merge_callback,
                None,
                name_callback);
            rocksdb_ffi::rocksdb_options_set_merge_operator(self.inner, mo);
        }
    }

    pub fn set_block_size(&self, size: u64) {
        unsafe {
            rocksdb_ffi::rocksdb_block_based_options_set_block_size(
                self.block_options, size);
            rocksdb_ffi::rocksdb_options_set_block_based_table_factory(
                self.inner,
                self.block_options);
        }
    }

    pub fn set_block_cache_size_mb(&self, cache_size: u64) {
        unsafe {
            rocksdb_ffi::rocksdb_options_optimize_for_point_lookup(
                self.inner, cache_size);
        }
    }

    pub fn set_filter(&self, filter: rocksdb_ffi::RocksDBFilterPolicy) {
        unsafe {
            rocksdb_ffi::rocksdb_block_based_options_set_filter_policy(
                self.block_options, filter);
            rocksdb_ffi::rocksdb_options_set_block_based_table_factory(
                self.inner,
                self.block_options);
        }
    }

    pub fn set_cache(&self, cache: rocksdb_ffi::RocksDBCache) {
        unsafe {
            rocksdb_ffi::rocksdb_block_based_options_set_block_cache(
                self.block_options, cache);
            rocksdb_ffi::rocksdb_options_set_block_based_table_factory(
                self.inner,
                self.block_options);
        }
    }

    pub fn set_cache_compressed(&self, cache: rocksdb_ffi::RocksDBCache) {
        unsafe {
            rocksdb_ffi::rocksdb_block_based_options_set_block_cache_compressed(
                self.block_options, cache);
            rocksdb_ffi::rocksdb_options_set_block_based_table_factory(
                self.inner,
                self.block_options);
        }
    }

    pub fn set_max_open_files(&self, nfiles: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_max_open_files(self.inner, nfiles);
        }
    }

    pub fn set_use_fsync(&self, useit: bool) {
        unsafe {
            match useit {
                true =>
                    rocksdb_ffi::rocksdb_options_set_use_fsync(self.inner, 1),
                false =>
                    rocksdb_ffi::rocksdb_options_set_use_fsync(self.inner, 0),
            }
        }
    }

    pub fn set_bytes_per_sync(&self, nbytes: u64) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_bytes_per_sync(
                self.inner, nbytes);
        }
    }

    pub fn set_disable_data_sync(&self, disable: bool) {
        unsafe {
            match disable {
                true =>
                    rocksdb_ffi::rocksdb_options_set_disable_data_sync(
                        self.inner, 1),
                false =>
                    rocksdb_ffi::rocksdb_options_set_disable_data_sync(
                        self.inner, 0),
            }
        }
    }

    pub fn set_table_cache_num_shard_bits(&self, nbits: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_table_cache_numshardbits(
                self.inner, nbits);
        }
    }

    pub fn set_min_write_buffer_number(&self, nbuf: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_min_write_buffer_number_to_merge(
                self.inner, nbuf);
        }
    }

    pub fn set_max_write_buffer_number(&self, nbuf: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_max_write_buffer_number(
                self.inner, nbuf);
        }
    }

    pub fn set_write_buffer_size(&self, size: size_t) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_write_buffer_size(
                self.inner, size);
        }
    }

    pub fn set_target_file_size_base(&self, size: u64) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_target_file_size_base(
                self.inner, size);
        }
    }

    pub fn set_min_write_buffer_number_to_merge(&self, to_merge: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_min_write_buffer_number_to_merge(
                self.inner, to_merge);
        }
    }

    pub fn set_level_zero_slowdown_writes_trigger(&self, n: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_level0_slowdown_writes_trigger(
                self.inner, n);
        }
    }

    pub fn set_level_zero_stop_writes_trigger(&self, n: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_level0_stop_writes_trigger(
                self.inner, n);
        }
    }

    pub fn set_compaction_style(&self, style:
                                rocksdb_ffi::RocksDBCompactionStyle) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_compaction_style(
                self.inner, style);
        }
    }

    pub fn set_max_background_compactions(&self, n: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_max_background_compactions(
                self.inner, n);
        }
    }

    pub fn set_max_background_flushes(&self, n: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_max_background_flushes(
                self.inner, n);
        }
    }

    pub fn set_filter_deletes(&self, filter: bool) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_filter_deletes(
                self.inner, filter);
        }
    }

    pub fn set_disable_auto_compactions(&self, disable: bool) {
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
}


