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

use libc::{self, size_t};

use crate::ffi;

// Cache is a cache used to store data read from data in memory.
pub struct Cache {
    pub(crate) inner: *mut ffi::rocksdb_cache_t,
}

#[allow(dead_code)]
impl Cache {
    /// Create a lru cache with capacity
    pub fn new_lru_cache(capacity: size_t) -> Cache {
        let cache = unsafe { ffi::rocksdb_cache_create_lru(capacity) };
        if cache.is_null() {
            panic!("Could not create Cache");
        }
        Cache { inner: cache }
    }

    /// Returns the Cache memory usage
    pub fn get_usage(&self) -> usize {
        unsafe { ffi::rocksdb_cache_get_usage(self.inner) }
    }

    /// Returns pinned memory usage
    pub fn pinned_usafe(&self) -> usize {
        unsafe { ffi::rocksdb_cache_get_pinned_usage(self.inner) }
    }

    /// Sets cache capacity
    pub fn set_capacity(&mut self, capacity: size_t) {
        unsafe {
            ffi::rocksdb_cache_set_capacity(self.inner, capacity);
        }
    }
}

impl Drop for Cache {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_cache_destroy(self.inner);
        }
    }
}
