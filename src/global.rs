use libc::c_int;

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
