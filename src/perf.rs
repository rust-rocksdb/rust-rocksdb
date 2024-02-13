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

use libc::{c_int, c_uchar, c_void};

use crate::{db::DBInner, ffi, ffi_util::from_cstr, Cache, Error};
use crate::{DBCommon, ThreadMode, DB};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(i32)]
pub enum PerfStatsLevel {
    /// Unknown settings
    Uninitialized = 0,
    /// Disable perf stats
    Disable,
    /// Enables only count stats
    EnableCount,
    /// Count stats and enable time stats except for mutexes
    EnableTimeExceptForMutex,
    /// Other than time, also measure CPU time counters. Still don't measure
    /// time (neither wall time nor CPU time) for mutexes
    EnableTimeAndCPUTimeExceptForMutex,
    /// Enables count and time stats
    EnableTime,
    /// N.B must always be the last value!
    OutOfBound,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(i32)]
pub enum PerfMetric {
    UserKeyComparisonCount = 0,
    BlockCacheHitCount = 1,
    BlockReadCount = 2,
    BlockReadByte = 3,
    BlockReadTime = 4,
    BlockChecksumTime = 5,
    BlockDecompressTime = 6,
    GetReadBytes = 7,
    MultigetReadBytes = 8,
    IterReadBytes = 9,
    InternalKeySkippedCount = 10,
    InternalDeleteSkippedCount = 11,
    InternalRecentSkippedCount = 12,
    InternalMergeCount = 13,
    GetSnapshotTime = 14,
    GetFromMemtableTime = 15,
    GetFromMemtableCount = 16,
    GetPostProcessTime = 17,
    GetFromOutputFilesTime = 18,
    SeekOnMemtableTime = 19,
    SeekOnMemtableCount = 20,
    NextOnMemtableCount = 21,
    PrevOnMemtableCount = 22,
    SeekChildSeekTime = 23,
    SeekChildSeekCount = 24,
    SeekMinHeapTime = 25,
    SeekMaxHeapTime = 26,
    SeekInternalSeekTime = 27,
    FindNextUserEntryTime = 28,
    WriteWalTime = 29,
    WriteMemtableTime = 30,
    WriteDelayTime = 31,
    WritePreAndPostProcessTime = 32,
    DbMutexLockNanos = 33,
    DbConditionWaitNanos = 34,
    MergeOperatorTimeNanos = 35,
    ReadIndexBlockNanos = 36,
    ReadFilterBlockNanos = 37,
    NewTableBlockIterNanos = 38,
    NewTableIteratorNanos = 39,
    BlockSeekNanos = 40,
    FindTableNanos = 41,
    BloomMemtableHitCount = 42,
    BloomMemtableMissCount = 43,
    BloomSstHitCount = 44,
    BloomSstMissCount = 45,
    KeyLockWaitTime = 46,
    KeyLockWaitCount = 47,
    EnvNewSequentialFileNanos = 48,
    EnvNewRandomAccessFileNanos = 49,
    EnvNewWritableFileNanos = 50,
    EnvReuseWritableFileNanos = 51,
    EnvNewRandomRwFileNanos = 52,
    EnvNewDirectoryNanos = 53,
    EnvFileExistsNanos = 54,
    EnvGetChildrenNanos = 55,
    EnvGetChildrenFileAttributesNanos = 56,
    EnvDeleteFileNanos = 57,
    EnvCreateDirNanos = 58,
    EnvCreateDirIfMissingNanos = 59,
    EnvDeleteDirNanos = 60,
    EnvGetFileSizeNanos = 61,
    EnvGetFileModificationTimeNanos = 62,
    EnvRenameFileNanos = 63,
    EnvLinkFileNanos = 64,
    EnvLockFileNanos = 65,
    EnvUnlockFileNanos = 66,
    EnvNewLoggerNanos = 67,
    TotalMetricCount = 68,
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
    fn default() -> Self {
        let ctx = unsafe { ffi::rocksdb_perfcontext_create() };
        assert!(!ctx.is_null(), "Could not create Perf Context");

        Self { inner: ctx }
    }
}

impl Drop for PerfContext {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_perfcontext_destroy(self.inner);
        }
    }
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
            let ptr =
                ffi::rocksdb_perfcontext_report(self.inner, c_uchar::from(exclude_zero_counters));
            let report = from_cstr(ptr);
            ffi::rocksdb_free(ptr as *mut c_void);
            report
        }
    }

    /// Returns value of a metric
    pub fn metric(&self, id: PerfMetric) -> u64 {
        unsafe { ffi::rocksdb_perfcontext_metric(self.inner, id as c_int) }
    }
}

/// Memory usage stats
pub struct MemoryUsageStats {
    /// Approximate memory usage of all the mem-tables
    pub mem_table_total: u64,
    /// Approximate memory usage of un-flushed mem-tables
    pub mem_table_unflushed: u64,
    /// Approximate memory usage of all the table readers
    pub mem_table_readers_total: u64,
    /// Approximate memory usage by cache
    pub cache_total: u64,
}

/// Wrap over memory_usage_t. Hold current memory usage of the specified DB instances and caches
pub struct MemoryUsage {
    inner: *mut ffi::rocksdb_memory_usage_t,
}

impl Drop for MemoryUsage {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_approximate_memory_usage_destroy(self.inner);
        }
    }
}

impl MemoryUsage {
    /// Approximate memory usage of all the mem-tables
    pub fn approximate_mem_table_total(&self) -> u64 {
        unsafe { ffi::rocksdb_approximate_memory_usage_get_mem_table_total(self.inner) }
    }

    /// Approximate memory usage of un-flushed mem-tables
    pub fn approximate_mem_table_unflushed(&self) -> u64 {
        unsafe { ffi::rocksdb_approximate_memory_usage_get_mem_table_unflushed(self.inner) }
    }

    /// Approximate memory usage of all the table readers
    pub fn approximate_mem_table_readers_total(&self) -> u64 {
        unsafe { ffi::rocksdb_approximate_memory_usage_get_mem_table_readers_total(self.inner) }
    }

    /// Approximate memory usage by cache
    pub fn approximate_cache_total(&self) -> u64 {
        unsafe { ffi::rocksdb_approximate_memory_usage_get_cache_total(self.inner) }
    }
}

/// Builder for MemoryUsage
pub struct MemoryUsageBuilder {
    inner: *mut ffi::rocksdb_memory_consumers_t,
}

impl Drop for MemoryUsageBuilder {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_memory_consumers_destroy(self.inner);
        }
    }
}

impl MemoryUsageBuilder {
    /// Create new instance
    pub fn new() -> Result<Self, Error> {
        let mc = unsafe { ffi::rocksdb_memory_consumers_create() };
        if mc.is_null() {
            Err(Error::new(
                "Could not create MemoryUsage builder".to_owned(),
            ))
        } else {
            Ok(Self { inner: mc })
        }
    }

    /// Add a DB instance to collect memory usage from it and add up in total stats
    pub fn add_db<T: ThreadMode, D: DBInner>(&mut self, db: &DBCommon<T, D>) {
        unsafe {
            ffi::rocksdb_memory_consumers_add_db(self.inner, db.inner.inner());
        }
    }

    /// Add a cache to collect memory usage from it and add up in total stats
    pub fn add_cache(&mut self, cache: &Cache) {
        unsafe {
            ffi::rocksdb_memory_consumers_add_cache(self.inner, cache.0.inner.as_ptr());
        }
    }

    /// Build up MemoryUsage
    pub fn build(&self) -> Result<MemoryUsage, Error> {
        unsafe {
            let mu = ffi_try!(ffi::rocksdb_approximate_memory_usage_create(self.inner));
            Ok(MemoryUsage { inner: mu })
        }
    }
}

/// Get memory usage stats from DB instances and Cache instances
pub fn get_memory_usage_stats(
    dbs: Option<&[&DB]>,
    caches: Option<&[&Cache]>,
) -> Result<MemoryUsageStats, Error> {
    let mut builder = MemoryUsageBuilder::new()?;
    if let Some(dbs_) = dbs {
        dbs_.iter().for_each(|db| builder.add_db(db));
    }
    if let Some(caches_) = caches {
        caches_.iter().for_each(|cache| builder.add_cache(cache));
    }

    let mu = builder.build()?;
    Ok(MemoryUsageStats {
        mem_table_total: mu.approximate_mem_table_total(),
        mem_table_unflushed: mu.approximate_mem_table_unflushed(),
        mem_table_readers_total: mu.approximate_mem_table_readers_total(),
        cache_total: mu.approximate_cache_total(),
    })
}
