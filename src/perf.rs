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

#[derive(Debug, Copy, Clone, PartialEq)]
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

    /// Returns value of a metric
    pub fn metric(&self, id: PerfMetric) -> u64 {
        unsafe { ffi::rocksdb_perfcontext_metric(self.inner, id as c_int) }
    }
}
