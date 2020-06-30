// Copyright 2020 Tran Tuan Linh
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

use libc::{c_char, c_int, c_uchar, c_void};
use std::ffi::CStr;

use crate::ffi;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PerfStatsLevel {
    /// Unknown settings
    Uninitialized = 0 as isize,
    /// Disable perf stats
    Disable = 1 as isize,
    /// Enables only count stats
    EnableCount = 2 as isize,
    /// Count stats and enable time stats except for mutexes
    EnableTimeExceptForMutex = 3 as isize,
    /// Other than time, also measure CPU time counters. Still don't measure
    /// time (neither wall time nor CPU time) for mutexes
    EnableTimeAndCPUTimeExceptForMutex = 4 as isize,
    /// Enables count and time stats
    EnableTime = 5 as isize,
    /// N.B must always be the last value!
    OutOfBound = 6 as isize,
}

/// Sets the perf stats level for current thread.
pub fn set_perf_stats(lvl: PerfStatsLevel) {
    unsafe {
        ffi::rocksdb_set_perf_level(lvl as c_int);
    }
}

/// Thread local context for gathering performance counter efficiently
/// and transparently.
pub struct PerfContext {
    pub(crate) inner: *mut ffi::rocksdb_perfcontext_t,
}

impl Default for PerfContext {
    fn default() -> PerfContext {
        let ctx = unsafe { ffi::rocksdb_perfcontext_create() };
        if ctx.is_null() {
            panic!("Could not create Perf Context");
        }
        PerfContext { inner: ctx }
    }
}

impl Drop for PerfContext {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_perfcontext_destroy(self.inner);
        }
    }
}

fn convert(ptr: *const c_char) -> String {
    let cstr = unsafe { CStr::from_ptr(ptr as *const _) };
    let s = String::from_utf8_lossy(cstr.to_bytes()).into_owned();
    unsafe {
        libc::free(ptr as *mut c_void);
    }
    s
}

impl PerfContext {
    /// Reset context
    pub fn reset(&mut self) {
        unsafe {
            ffi::rocksdb_perfcontext_reset(self.inner);
        }
    }

    /// Get the report on perf
    pub fn report(&self, exclude_zero_counters: bool) -> String {
        unsafe {
            convert(ffi::rocksdb_perfcontext_report(
                self.inner,
                exclude_zero_counters as c_uchar,
            ))
        }
    }

    // Metric returns value of a metric by its id.
    //
    // Id is one of:
    //
    // enum {
    // 	rocksdb_user_key_comparison_count = 0,
    // 	rocksdb_block_cache_hit_count,
    // 	rocksdb_block_read_count,
    // 	rocksdb_block_read_byte,
    // 	rocksdb_block_read_time,
    // 	rocksdb_block_checksum_time,
    // 	rocksdb_block_decompress_time,
    // 	rocksdb_get_read_bytes,
    // 	rocksdb_multiget_read_bytes,
    // 	rocksdb_iter_read_bytes,
    // 	rocksdb_internal_key_skipped_count,
    // 	rocksdb_internal_delete_skipped_count,
    // 	rocksdb_internal_recent_skipped_count,
    // 	rocksdb_internal_merge_count,
    // 	rocksdb_get_snapshot_time,
    // 	rocksdb_get_from_memtable_time,
    // 	rocksdb_get_from_memtable_count,
    // 	rocksdb_get_post_process_time,
    // 	rocksdb_get_from_output_files_time,
    // 	rocksdb_seek_on_memtable_time,
    // 	rocksdb_seek_on_memtable_count,
    // 	rocksdb_next_on_memtable_count,
    // 	rocksdb_prev_on_memtable_count,
    // 	rocksdb_seek_child_seek_time,
    // 	rocksdb_seek_child_seek_count,
    // 	rocksdb_seek_min_heap_time,
    // 	rocksdb_seek_max_heap_time,
    // 	rocksdb_seek_internal_seek_time,
    // 	rocksdb_find_next_user_entry_time,
    // 	rocksdb_write_wal_time,
    // 	rocksdb_write_memtable_time,
    // 	rocksdb_write_delay_time,
    // 	rocksdb_write_pre_and_post_process_time,
    // 	rocksdb_db_mutex_lock_nanos,
    // 	rocksdb_db_condition_wait_nanos,
    // 	rocksdb_merge_operator_time_nanos,
    // 	rocksdb_read_index_block_nanos,
    // 	rocksdb_read_filter_block_nanos,
    // 	rocksdb_new_table_block_iter_nanos,
    // 	rocksdb_new_table_iterator_nanos,
    // 	rocksdb_block_seek_nanos,
    // 	rocksdb_find_table_nanos,
    // 	rocksdb_bloom_memtable_hit_count,
    // 	rocksdb_bloom_memtable_miss_count,
    // 	rocksdb_bloom_sst_hit_count,
    // 	rocksdb_bloom_sst_miss_count,
    // 	rocksdb_key_lock_wait_time,
    // 	rocksdb_key_lock_wait_count,
    // 	rocksdb_env_new_sequential_file_nanos,
    // 	rocksdb_env_new_random_access_file_nanos,
    // 	rocksdb_env_new_writable_file_nanos,
    // 	rocksdb_env_reuse_writable_file_nanos,
    // 	rocksdb_env_new_random_rw_file_nanos,
    // 	rocksdb_env_new_directory_nanos,
    // 	rocksdb_env_file_exists_nanos,
    // 	rocksdb_env_get_children_nanos,
    // 	rocksdb_env_get_children_file_attributes_nanos,
    // 	rocksdb_env_delete_file_nanos,
    // 	rocksdb_env_create_dir_nanos,
    // 	rocksdb_env_create_dir_if_missing_nanos,
    // 	rocksdb_env_delete_dir_nanos,
    // 	rocksdb_env_get_file_size_nanos,
    // 	rocksdb_env_get_file_modification_time_nanos,
    // 	rocksdb_env_rename_file_nanos,
    // 	rocksdb_env_link_file_nanos,
    // 	rocksdb_env_lock_file_nanos,
    // 	rocksdb_env_unlock_file_nanos,
    // 	rocksdb_env_new_logger_nanos,
    // 	rocksdb_total_metric_count = 68
    // }
    pub fn metric(&self, id: c_int) -> u64 {
        unsafe { ffi::rocksdb_perfcontext_metric(self.inner, id) }
    }
}
