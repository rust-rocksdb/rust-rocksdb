// Copyright 2026 Arni Dagur
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

use std::ptr::NonNull;
use std::sync::{Arc, Mutex, PoisonError};

use crate::{ffi, Env};

pub(crate) struct SstFileManagerWrapper {
    pub(crate) inner: NonNull<ffi::rocksdb_sst_file_manager_t>,
    // RocksDB starts its deletion thread from the delete rate setter without synchronizing
    // concurrent callers.
    delete_rate_lock: Mutex<()>,
    // The manager's file system and clock may point back at the environment.
    _outlive: Env,
}

impl Drop for SstFileManagerWrapper {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_sst_file_manager_destroy(self.inner.as_ptr());
        }
    }
}

// SAFETY: RocksDB documents every public `SstFileManager` function as thread-safe, and
// `delete_rate_lock` serializes the one that is not.
unsafe impl Send for SstFileManagerWrapper {}
unsafe impl Sync for SstFileManagerWrapper {}

/// Used to track SST and blob files in the DB and control their deletion rate.
///
/// <https://github.com/facebook/rocksdb/wiki/Managing-Disk-Space-Utilization>
///
/// A manager can be shared among multiple RocksDB instances to track SST and blob files and
/// control their deletion rate; attach it with
/// [`Options::set_sst_file_manager`](crate::Options::set_sst_file_manager). Even though the
/// manager does not track WAL files, it still controls their deletion rate.
///
/// All `SstFileManager` public functions are thread-safe.
#[derive(Clone)]
pub struct SstFileManager(pub(crate) Arc<SstFileManagerWrapper>);

impl SstFileManager {
    /// Creates a manager for `env` with RocksDB's defaults: deletion rate limiting disabled
    /// and a trash/DB size ratio of 0.25.
    ///
    /// # Examples
    ///
    /// ```
    /// use rocksdb::{Env, Options, SstFileManager};
    ///
    /// let env = Env::new().unwrap();
    /// let sst_file_manager = SstFileManager::new(&env);
    /// let mut opts = Options::default();
    /// opts.set_env(&env);
    /// opts.set_sst_file_manager(&sst_file_manager);
    /// ```
    pub fn new(env: &Env) -> SstFileManager {
        let inner = unsafe { ffi::rocksdb_sst_file_manager_create(env.0.inner) };
        let inner = NonNull::new(inner).expect("rocksdb_sst_file_manager_create returned null");
        SstFileManager(Arc::new(SstFileManagerWrapper {
            inner,
            delete_rate_lock: Mutex::new(()),
            _outlive: env.clone(),
        }))
    }

    /// Updates the maximum allowed space that should be used by RocksDB: if the total size of
    /// the SST and blob files exceeds `max_allowed_space`, writes to RocksDB will fail.
    ///
    /// Setting `max_allowed_space` to `0` disables this feature; the maximum allowed space is
    /// then infinite (the default).
    pub fn set_max_allowed_space_usage(&self, max_allowed_space: u64) {
        unsafe {
            ffi::rocksdb_sst_file_manager_set_max_allowed_space_usage(
                self.0.inner.as_ptr(),
                max_allowed_space,
            );
        }
    }

    /// Sets the amount of buffer room each compaction should be able to leave. In other
    /// words, at its maximum disk space consumption, the compaction should still leave
    /// `compaction_buffer_size` available on the disk so that other background functions
    /// may continue, such as logging and flushing.
    pub fn set_compaction_buffer_size(&self, compaction_buffer_size: u64) {
        unsafe {
            ffi::rocksdb_sst_file_manager_set_compaction_buffer_size(
                self.0.inner.as_ptr(),
                compaction_buffer_size,
            );
        }
    }

    /// Returns `true` if the total size of SST and blob files exceeded the maximum allowed
    /// space usage.
    pub fn is_max_allowed_space_reached(&self) -> bool {
        unsafe { ffi::rocksdb_sst_file_manager_is_max_allowed_space_reached(self.0.inner.as_ptr()) }
    }

    /// Returns `true` if the total size of SST and blob files as well as the estimated size
    /// of ongoing compactions exceeds the maximum allowed space usage.
    pub fn is_max_allowed_space_reached_including_compactions(&self) -> bool {
        unsafe {
            ffi::rocksdb_sst_file_manager_is_max_allowed_space_reached_including_compactions(
                self.0.inner.as_ptr(),
            )
        }
    }

    /// Returns the total size of all tracked files.
    pub fn total_size(&self) -> u64 {
        unsafe { ffi::rocksdb_sst_file_manager_get_total_size(self.0.inner.as_ptr()) }
    }

    /// Returns the delete rate limit in bytes per second.
    pub fn delete_rate_bytes_per_second(&self) -> i64 {
        unsafe {
            ffi::rocksdb_sst_file_manager_get_delete_rate_bytes_per_second(self.0.inner.as_ptr())
        }
    }

    /// Updates the delete rate limit in bytes per second. Zero means disable delete rate
    /// limiting and delete files immediately.
    ///
    /// If the rate is set to 1024 (1 KiB/s) and a file of 4 KiB is deleted in one second,
    /// RocksDB waits another three seconds before deleting other files. This also affects the
    /// delete rate of the DB's WAL files.
    pub fn set_delete_rate_bytes_per_second(&self, delete_rate: i64) {
        let _guard = self
            .0
            .delete_rate_lock
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        unsafe {
            ffi::rocksdb_sst_file_manager_set_delete_rate_bytes_per_second(
                self.0.inner.as_ptr(),
                delete_rate,
            );
        }
    }

    /// Returns the trash/DB size ratio where new files will be deleted immediately.
    pub fn max_trash_db_ratio(&self) -> f64 {
        unsafe { ffi::rocksdb_sst_file_manager_get_max_trash_db_ratio(self.0.inner.as_ptr()) }
    }

    /// Updates the trash/DB size ratio where new files will be deleted immediately: if the
    /// trash size constitutes more than this fraction of the total DB size, new files passed
    /// to the delete scheduler are deleted immediately.
    ///
    /// `ratio` must not be negative: a negative or NaN `ratio` is clamped to `0`.
    pub fn set_max_trash_db_ratio(&self, ratio: f64) {
        let ratio = ratio.max(0.0);
        unsafe {
            ffi::rocksdb_sst_file_manager_set_max_trash_db_ratio(self.0.inner.as_ptr(), ratio);
        }
    }

    /// Returns the total size of trash files.
    pub fn total_trash_size(&self) -> u64 {
        unsafe { ffi::rocksdb_sst_file_manager_get_total_trash_size(self.0.inner.as_ptr()) }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{Env, SstFileManager};

    #[test]
    fn the_manager_keeps_its_env_alive() {
        let env = Env::new().unwrap();
        let manager = SstFileManager::new(&env);
        assert_eq!(Arc::strong_count(&env.0), 2);
        drop(manager);
        assert_eq!(Arc::strong_count(&env.0), 1);
    }
}
