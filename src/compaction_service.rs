use std::ffi::{CStr, CString};

use crate::ffi;
use crate::Error;
use crate::ffi_util;
use crate::env::Priority;

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionServiceJobStatus {
    Success = 0,
    Failure = 1,
    Aborted = 2,
    UseLocal = 3,
}

/// Reason for a compaction to be triggered.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionReason {
    Unknown = 0,
    /// [Level] number of L0 files > level0_file_num_compaction_trigger
    LevelL0FilesNum = 1,
    /// [Level] total size of level > MaxBytesForLevel()
    LevelMaxLevelSize = 2,
    /// [Universal] Compacting for size amplification
    UniversalSizeAmplification = 3,
    /// [Universal] Compacting for size ratio
    UniversalSizeRatio = 4,
    /// [Universal] number of sorted runs > level0_file_num_compaction_trigger
    UniversalSortedRunNum = 5,
    /// [FIFO] total size > max_table_files_size
    FIFOMaxSize = 6,
    /// [FIFO] reduce number of files
    FIFOReduceNumFiles = 7,
    /// [FIFO] files with creation time < (current_time - interval)
    FIFOTtl = 8,
    /// Manual compaction
    ManualCompaction = 9,
    /// DB::SuggestCompactRange() marked files for compaction
    FilesMarkedForCompaction = 10,
    /// [Level] Automatic compaction within bottommost level to cleanup duplicate
    /// versions of same user key, usually due to a released snapshot.
    BottommostFiles = 11,
    /// Compaction based on TTL
    Ttl = 12,
    /// According to the comments in flush_job.cc, RocksDB treats flush as
    /// a level 0 compaction in internal stats.
    Flush = 13,
    /// [InternalOnly] External sst file ingestion treated as a compaction
    /// with placeholder input level L0 as file ingestion
    /// technically does not have an input level like other compactions.
    /// Used only for internal stats and conflict checking with other compactions.
    ExternalSstIngestion = 14,
    /// Compaction due to SST file being too old
    PeriodicCompaction = 15,
    /// Compaction in order to move files to temperature
    ChangeTemperature = 16,
    /// Compaction scheduled to force garbage collection of blob files
    ForcedBlobGC = 17,
    /// A special TTL compaction for RoundRobin policy, which basically the same as
    /// kLevelMaxLevelSize, but the goal is to compact TTLed files.
    RoundRobinTtl = 18,
    /// [InternalOnly] DBImpl::ReFitLevel treated as a compaction,
    /// Used only for internal conflict checking with other compactions.
    RefitLevel = 19,
    /// Total number of compaction reasons (not a valid reason).
    NumOfReasons = 20,
}

pub struct CompactionServiceJobInfo {
    inner: *const ffi::rocksdb_compactionservice_jobinfo_t,
}

impl CompactionServiceJobInfo {
    fn ptr_to_string(ptr: *const i8, len: usize, error_msg: &str) -> Result<String, Error> {
        unsafe {
            if ptr.is_null() {
                return Err(Error::new(error_msg.to_string()));
            }
            let slice = std::slice::from_raw_parts(ptr as *const u8, len);
            Ok(String::from_utf8_lossy(slice).to_string())
        }
    }

    fn get_string_from_ffi<F>(inner: *const ffi::rocksdb_compactionservice_jobinfo_t, ffi_fn: F, error_msg: &str) -> Result<String, Error>
    where
        F: Fn(*const ffi::rocksdb_compactionservice_jobinfo_t, &mut usize) -> *const i8,
    {
        unsafe {
            let mut len = 0;
            let ptr = ffi_fn(inner, &mut len);
            Self::ptr_to_string(ptr, len, error_msg)
        }
    }

    pub fn db_name(&self) -> Result<String, Error> {
        Self::get_string_from_ffi(
            self.inner,
            ffi::rocksdb_compactionservice_jobinfo_t_get_db_name,
            "Failed to get db_name",
        )
    }

    pub fn db_id(&self) -> Result<String, Error> {
        Self::get_string_from_ffi(
            self.inner,
            ffi::rocksdb_compactionservice_jobinfo_t_get_db_id,
            "Failed to get db_id",
        )
    }

    pub fn db_session_id(&self) -> Result<String, Error> {
        unsafe {
            Self::get_string_from_ffi(
                self.inner, 
                ffi::rocksdb_compactionservice_jobinfo_t_get_db_session_id,
                "Failed to get db_session_id"
            )
     
        }
    }

    pub fn cf_name(&self) -> Result<String, Error> {
        unsafe {
            Self::get_string_from_ffi(
                self.inner, 
                ffi::rocksdb_compactionservice_jobinfo_t_get_cf_name
                "Failed to get cf_name"
            )
     
        }
    }

    pub fn cf_id(&self) -> u32 {
        unsafe {
            ffi::rocksdb_compactionservice_jobinfo_t_get_cf_id
        }
    }

    pub fn job_id(&self) -> u64 {
        unsafe {
            ffi::rocksdb_compactionservice_jobinfo_t_get_job_id(self.inner);
        }
    }

    pub fn priority(&self) -> Priority {
        unsafe {
            let job_priority = ffi::rocksdb_compactionservice_jobinfo_t_get_priority(self.inner);
            std::mem::transmute(job_priority)
        }
    }

    pub fn compaction_reason(&self) -> CompactionReason {
        unsafe {
            let reason = ffi::rocksdb_compactionservice_jobinfo_t_get_compaction_reason(self.inner);
            std::mem::transmute(reason)
        }
    }

    pub fn base_input_level(&self) -> u64 {
        unsafe {
            ffi::rocksdb_compactionservice_jobinfo_t_get_base_input_level(self.inner);
        }
    }

    pub fn output_level(&self) -> u64 {
        unsafe {
            ffi::rocksdb_compactionservice_jobinfo_t_get_output_level(self.inner);
        }
    }

    pub fn is_full_compaction(&self) -> bool {
        unsafe {
            ffi::rocksdb_compactionservice_jobinfo_t_is_full_compaction(self.inner) != 0
        }
    }

    pub fn is_manual_compaction(&self) -> bool {
        unsafe {
            ffi::rocksdb_compactionservice_jobinfo_t_is_manual_compaction(self.inner) != 0
        }
    }

    pub fn is_bottommost_level(&self) -> bool {
        unsafe {
            ffi::rocksdb_compactionservice_jobinfo_t_is_bottommost_level(self.inner) != 0
        }
    }

}