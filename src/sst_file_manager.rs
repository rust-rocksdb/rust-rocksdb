// Copyright 2020 Tyler Neely
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

use crate::{ffi, Env};
use std::ptr::NonNull;
use std::sync::Arc;

pub(crate) struct SstFileManagerWrapper {
    pub(crate) inner: NonNull<ffi::rocksdb_sst_file_manager_t>,
}

impl Drop for SstFileManagerWrapper {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_sst_file_manager_destroy(self.inner.as_ptr());
        }
    }
}

unsafe impl Send for SstFileManagerWrapper {}
unsafe impl Sync for SstFileManagerWrapper {}

/// SstFileManager is used to track SST files in the database and control their deletion rate.
///
/// All SstFileManager public functions are thread-safe.
#[derive(Clone)]
pub struct SstFileManager(pub(crate) Arc<SstFileManagerWrapper>);

impl SstFileManager {
    /// Create a new SstFileManager with the default environment.
    ///
    /// # Examples
    ///
    /// ```
    /// use rocksdb::{SstFileManager, Env};
    ///
    /// let env = Env::new().unwrap();
    /// let sst_file_manager = SstFileManager::new(&env);
    /// ```
    pub fn new(env: &Env) -> Self {
        let inner =
            unsafe { NonNull::new_unchecked(ffi::rocksdb_sst_file_manager_create(env.0.inner)) };
        SstFileManager(Arc::new(SstFileManagerWrapper { inner }))
    }

    /// Set the maximum allowed space usage in bytes.
    ///
    /// This will track total size of SST files and will prevent new flushes/compactions
    /// if the total size of SST files exceeds the provided limit.
    ///
    /// # Arguments
    ///
    /// * `max_allowed_space` - Maximum allowed space usage in bytes
    ///
    /// # Examples
    ///
    /// ```
    /// use rocksdb::{SstFileManager, Env};
    ///
    /// let env = Env::new().unwrap();
    /// let sst_file_manager = SstFileManager::new(&env);
    /// // Set maximum allowed space to 1GB
    /// sst_file_manager.set_max_allowed_space_usage(1024 * 1024 * 1024);
    /// ```
    pub fn set_max_allowed_space_usage(&self, max_allowed_space: u64) {
        unsafe {
            ffi::rocksdb_sst_file_manager_set_max_allowed_space_usage(
                self.0.inner.as_ptr(),
                max_allowed_space,
            );
        }
    }

    /// Set the compaction buffer size in bytes.
    ///
    /// This is used to reserve space for compaction operations.
    ///
    /// # Arguments
    ///
    /// * `compaction_buffer_size` - Compaction buffer size in bytes
    pub fn set_compaction_buffer_size(&self, compaction_buffer_size: u64) {
        unsafe {
            ffi::rocksdb_sst_file_manager_set_compaction_buffer_size(
                self.0.inner.as_ptr(),
                compaction_buffer_size,
            );
        }
    }

    /// Returns true if the total size of SST files exceeded the maximum allowed space usage.
    ///
    /// # Examples
    ///
    /// ```
    /// use rocksdb::{SstFileManager, Env};
    ///
    /// let env = Env::new().unwrap();
    /// let sst_file_manager = SstFileManager::new(&env);
    /// sst_file_manager.set_max_allowed_space_usage(1024 * 1024 * 1024);
    ///
    /// if sst_file_manager.is_max_allowed_space_reached() {
    ///     println!("Maximum space reached!");
    /// }
    /// ```
    pub fn is_max_allowed_space_reached(&self) -> bool {
        unsafe { ffi::rocksdb_sst_file_manager_is_max_allowed_space_reached(self.0.inner.as_ptr()) }
    }

    /// Returns true if the total size of SST files as well as estimated size of ongoing
    /// compactions exceeds the maximum allowed space usage.
    pub fn is_max_allowed_space_reached_including_compactions(&self) -> bool {
        unsafe {
            ffi::rocksdb_sst_file_manager_is_max_allowed_space_reached_including_compactions(
                self.0.inner.as_ptr(),
            )
        }
    }

    /// Returns the total size of all tracked SST files in bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use rocksdb::{SstFileManager, Env};
    ///
    /// let env = Env::new().unwrap();
    /// let sst_file_manager = SstFileManager::new(&env);
    /// let total_size = sst_file_manager.get_total_size();
    /// println!("Total SST files size: {} bytes", total_size);
    /// ```
    pub fn get_total_size(&self) -> u64 {
        unsafe { ffi::rocksdb_sst_file_manager_get_total_size(self.0.inner.as_ptr()) }
    }

    /// Returns the current delete rate in bytes per second.
    pub fn get_delete_rate_bytes_per_second(&self) -> i64 {
        unsafe {
            ffi::rocksdb_sst_file_manager_get_delete_rate_bytes_per_second(self.0.inner.as_ptr())
        }
    }

    /// Set the delete rate limit in bytes per second.
    ///
    /// This controls how fast obsolete SST files are deleted.
    ///
    /// # Arguments
    ///
    /// * `delete_rate` - Delete rate in bytes per second
    pub fn set_delete_rate_bytes_per_second(&self, delete_rate: i64) {
        unsafe {
            ffi::rocksdb_sst_file_manager_set_delete_rate_bytes_per_second(
                self.0.inner.as_ptr(),
                delete_rate,
            );
        }
    }

    /// Returns the maximum trash to DB size ratio.
    pub fn get_max_trash_db_ratio(&self) -> f64 {
        unsafe { ffi::rocksdb_sst_file_manager_get_max_trash_db_ratio(self.0.inner.as_ptr()) }
    }

    /// Set the maximum trash to DB size ratio.
    ///
    /// # Arguments
    ///
    /// * `ratio` - Maximum trash to DB size ratio
    pub fn set_max_trash_db_ratio(&self, ratio: f64) {
        unsafe {
            ffi::rocksdb_sst_file_manager_set_max_trash_db_ratio(self.0.inner.as_ptr(), ratio);
        }
    }

    /// Returns the total size of trash files in bytes.
    pub fn get_total_trash_size(&self) -> u64 {
        unsafe { ffi::rocksdb_sst_file_manager_get_total_trash_size(self.0.inner.as_ptr()) }
    }
}
