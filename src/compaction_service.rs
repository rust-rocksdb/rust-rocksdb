use std::ffi::CString;

use core::ffi::{c_char, c_int, c_void};

use libc::size_t;
use std::ffi::CStr;

use crate::db_options::BlockBasedOptions;
use crate::env::Priority;
use crate::ffi;
use crate::ffi_util;
use crate::Error;

use std::path::Path;

use crate::Env;
use std::sync::Arc;

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
        if ptr.is_null() {
            return Err(Error::new(error_msg.to_string()));
        }
        unsafe {
            let slice = std::slice::from_raw_parts(ptr as *const u8, len);
            Ok(String::from_utf8_lossy(slice).to_string())
        }
    }

    pub fn db_name(&self) -> Result<String, Error> {
        unsafe {
            let mut len = 0;
            let ptr = ffi::rocksdb_compactionservice_jobinfo_t_get_db_name(self.inner, &mut len);
            Self::ptr_to_string(ptr, len, "Failed to get db_name")
        }
    }

    pub fn db_id(&self) -> Result<String, Error> {
        unsafe {
            let mut len = 0;
            let ptr = ffi::rocksdb_compactionservice_jobinfo_t_get_db_id(self.inner, &mut len);
            Self::ptr_to_string(ptr, len, "Failed to get db_id")
        }
    }

    pub fn db_session_id(&self) -> Result<String, Error> {
        unsafe {
            let mut len = 0;
            let ptr =
                ffi::rocksdb_compactionservice_jobinfo_t_get_db_session_id(self.inner, &mut len);
            Self::ptr_to_string(ptr, len, "Failed to get db_session_id")
        }
    }

    pub fn cf_name(&self) -> Result<String, Error> {
        unsafe {
            let mut len = 0;
            let ptr = ffi::rocksdb_compactionservice_jobinfo_t_get_cf_name(self.inner, &mut len);
            Self::ptr_to_string(ptr, len, "Failed to get cf_name")
        }
    }

    pub fn cf_id(&self) -> u32 {
        unsafe { ffi::rocksdb_compactionservice_jobinfo_t_get_cf_id(self.inner) }
    }

    pub fn job_id(&self) -> u64 {
        unsafe { ffi::rocksdb_compactionservice_jobinfo_t_get_job_id(self.inner) }
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

    pub fn base_input_level(&self) -> i32 {
        unsafe { ffi::rocksdb_compactionservice_jobinfo_t_get_base_input_level(self.inner) }
    }

    pub fn output_level(&self) -> i32 {
        unsafe { ffi::rocksdb_compactionservice_jobinfo_t_get_output_level(self.inner) }
    }

    pub fn is_full_compaction(&self) -> bool {
        unsafe { ffi::rocksdb_compactionservice_jobinfo_t_is_full_compaction(self.inner) != 0 }
    }

    pub fn is_manual_compaction(&self) -> bool {
        unsafe { ffi::rocksdb_compactionservice_jobinfo_t_is_manual_compaction(self.inner) != 0 }
    }

    pub fn is_bottommost_level(&self) -> bool {
        unsafe { ffi::rocksdb_compactionservice_jobinfo_t_is_bottommost_level(self.inner) != 0 }
    }
}

pub struct CompactionServiceScheduleResponse {
    inner: *mut ffi::rocksdb_compactionservice_scheduleresponse_t,
}

impl CompactionServiceScheduleResponse {
    pub fn new(
        scheduled_job_id: &str,
        status: CompactionServiceJobStatus,
    ) -> Result<CompactionServiceScheduleResponse, Error> {
        let job_id_cstr = CString::new(scheduled_job_id)
            .map_err(|e| Error::new(format!("Invalid job_id: {}", e)))?;

        let mut error: *mut libc::c_char = std::ptr::null_mut();
        let inner = unsafe {
            ffi::rocksdb_compactionservice_scheduleresponse_create(
                job_id_cstr.as_ptr(),
                status as i32,
                &mut error,
            )
        };
        if !error.is_null() {
            return Err(Error::new(ffi_util::error_message(error)));
        }

        Ok(CompactionServiceScheduleResponse { inner })
    }

    pub fn with_status(
        status: CompactionServiceJobStatus,
    ) -> Result<CompactionServiceScheduleResponse, Error> {
        let mut error: *mut libc::c_char = std::ptr::null_mut();
        let inner = unsafe {
            ffi::rocksdb_compactionservice_scheduleresponse_create_with_status(
                status as i32,
                &mut error,
            )
        };
        if !error.is_null() {
            return Err(Error::new(ffi_util::error_message(error)));
        }

        Ok(CompactionServiceScheduleResponse { inner })
    }

    pub fn status(&self) -> CompactionServiceJobStatus {
        unsafe {
            let status = ffi::rocksdb_compactionservice_scheduleresponse_getstatus(self.inner);
            match status {
                0 => CompactionServiceJobStatus::Success,
                1 => CompactionServiceJobStatus::Failure,
                2 => CompactionServiceJobStatus::Aborted,
                3 => CompactionServiceJobStatus::UseLocal,
                _ => CompactionServiceJobStatus::Failure,
            }
        }
    }

    pub fn scheduled_job_id(&self) -> Result<String, Error> {
        unsafe {
            let mut len = 0;
            let ptr = ffi::rocksdb_compactionservice_scheduleresponse_get_scheduled_job_id(
                self.inner, &mut len,
            );
            if ptr.is_null() {
                return Err(Error::new("No scheduled job ID".to_string()));
            }
            let slice = std::slice::from_raw_parts(ptr as *const u8, len);
            Ok(String::from_utf8_lossy(slice).to_string())
        }
    }
}

impl Drop for CompactionServiceScheduleResponse {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_compactionservice_scheduleresponse_t_destroy(self.inner);
        }
    }
}

pub trait CompactionService: Send + Sync + 'static {
    fn schedule(
        &self,
        job_info: &CompactionServiceJobInfo,
        compaction_service_input: &[u8],
    ) -> Result<CompactionServiceScheduleResponse, Error>;

    /// Wait for a scheduled compaction job to complete.
    ///
    /// Populates `result` with the serialized compaction output from open_and_compact.
    /// Returns the job completion status.
    fn wait(
        &self,
        scheduled_job_id: &str,
        result: &mut Vec<u8>,
    ) -> Result<CompactionServiceJobStatus, Error>;

    fn cancel_awaiting_jobs(&self) {}

    fn on_installation(&self, _scheduled_job_id: &str, _status: CompactionServiceJobStatus) {}

    fn name(&self) -> &CStr;
}

// Compaction Service Destructor
pub unsafe extern "C" fn destructor_callback<S>(raw_cb: *mut c_void)
where
    S: CompactionService,
{
    drop(Box::from_raw(raw_cb as *mut S));
}

// Name
pub unsafe extern "C" fn name_callback<S>(raw_cb: *mut c_void) -> *const c_char
where
    S: CompactionService,
{
    let service = &*(raw_cb as *mut S);
    service.name().as_ptr()
}

// Schedule Function
pub unsafe extern "C" fn schedule_callback<S>(
    raw_cb: *mut c_void,
    info: *const ffi::rocksdb_compactionservice_jobinfo_t,
    input: *const c_char,
    input_len: size_t,
) -> *mut ffi::rocksdb_compactionservice_scheduleresponse_t
where
    S: CompactionService,
{
    let service = &*(raw_cb as *mut S);
    let job_info = CompactionServiceJobInfo { inner: info };
    let input_slice = std::slice::from_raw_parts(input as *const u8, input_len);

    match service.schedule(&job_info, input_slice) {
        Ok(response) => {
            let ptr = response.inner;
            std::mem::forget(response); // letting c take up the ownership since this response is useful only for rocksdb c code
            ptr
        }
        Err(_) => {
            let mut err: *mut libc::c_char = std::ptr::null_mut();
            let response = ffi::rocksdb_compactionservice_scheduleresponse_create_with_status(
                CompactionServiceJobStatus::Failure as i32,
                &mut err,
            );
            response
        }
    }
}

// Wait
pub unsafe extern "C" fn wait_callback<S>(
    raw_cb: *mut c_void,
    scheduled_job_id: *const c_char,
    result: *mut *mut c_char,
    result_len: *mut size_t,
) -> c_int
where
    S: CompactionService,
{
    let service = &*(raw_cb as *mut S);
    let job_id = CStr::from_ptr(scheduled_job_id).to_str().unwrap_or("");

    // Create output buffer for the compaction result
    let mut result_bytes: Vec<u8> = Vec::new();

    match service.wait(job_id, &mut result_bytes) {
        Ok(status) => {
            // Allocate C memory and copy the result bytes
            let len = result_bytes.len();
            if len > 0 && !result.is_null() && !result_len.is_null() {
                let ptr = libc::malloc(len) as *mut c_char;
                if !ptr.is_null() {
                    std::ptr::copy_nonoverlapping(result_bytes.as_ptr() as *const c_char, ptr, len);
                    *result = ptr;
                    *result_len = len;
                }
            } else if !result.is_null() && !result_len.is_null() {
                *result = std::ptr::null_mut();
                *result_len = 0;
            }
            status as c_int
        }
        Err(_) => {
            // Ensure result pointers are set even on error
            if !result.is_null() && !result_len.is_null() {
                *result = std::ptr::null_mut();
                *result_len = 0;
            }
            CompactionServiceJobStatus::Failure as c_int
        }
    }
}

pub unsafe extern "C" fn cancel_awaiting_jobs_callback<S>(raw_cb: *mut c_void)
where
    S: CompactionService,
{
    let service = &*(raw_cb as *mut S);
    service.cancel_awaiting_jobs();
}

pub unsafe extern "C" fn on_installation_callback<S>(
    raw_cb: *mut c_void,
    scheduled_job_id: *const c_char,
    status: c_int,
) where
    S: CompactionService,
{
    let service = &*(raw_cb as *mut S);
    let job_id = CStr::from_ptr(scheduled_job_id).to_str().unwrap_or("");
    let status = match status {
        0 => CompactionServiceJobStatus::Success,
        1 => CompactionServiceJobStatus::Failure,
        2 => CompactionServiceJobStatus::Aborted,
        3 => CompactionServiceJobStatus::UseLocal,
        _ => CompactionServiceJobStatus::Failure,
    };
    service.on_installation(job_id, status);
}

pub struct CompactionServiceOptionsOverride {
    pub(crate) inner: *mut ffi::rocksdb_compaction_service_options_override_t,
    // Hold references to prevent dropping
    _env: Option<Env>,
}

impl Default for CompactionServiceOptionsOverride {
    fn default() -> Self {
        Self::new()
    }
}

impl CompactionServiceOptionsOverride {
    pub fn new() -> Self {
        let inner = unsafe { ffi::rocksdb_compaction_service_options_override_create() };
        Self { inner, _env: None }
    }

    /// Creates a new `CompactionServiceOptionsOverride` from existing `Options`.
    ///
    /// This copies relevant settings from the provided options, which is useful
    /// for ensuring the remote compaction worker uses the same configuration
    /// as the primary database (e.g., table factory, compression settings).
    pub fn from_options(options: &crate::Options) -> Self {
        let inner = unsafe {
            ffi::rocksdb_compaction_service_options_override_create_from_options(options.inner)
        };
        Self { inner, _env: None }
    }
}

impl Drop for CompactionServiceOptionsOverride {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_compaction_service_options_override_destroy(self.inner);
        }
    }
}

unsafe impl Send for CompactionServiceOptionsOverride {}
unsafe impl Sync for CompactionServiceOptionsOverride {}

/// Thread-safe cancellation token for OpenAndCompact operations.
/// Can be shared across threads to cancel an in-progress compaction.
pub struct OpenAndCompactCanceled {
    inner: *mut libc::c_uchar,
}

impl OpenAndCompactCanceled {
    pub fn new() -> Self {
        let inner = unsafe { ffi::rocksdb_open_and_compact_canceled_create() };
        Self { inner }
    }

    /// Sets the cancellation flag.
    /// When set to true, the ongoing OpenAndCompact operation will be cancelled.
    pub fn set(&self, canceled: bool) {
        unsafe {
            ffi::rocksdb_open_and_compact_canceled_set(self.inner, canceled as libc::c_uchar);
        }
    }

    pub(crate) fn as_ptr(&self) -> *mut libc::c_uchar {
        self.inner
    }
}

impl Default for OpenAndCompactCanceled {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for OpenAndCompactCanceled {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_open_and_compact_canceled_destroy(self.inner);
        }
    }
}

unsafe impl Send for OpenAndCompactCanceled {}
unsafe impl Sync for OpenAndCompactCanceled {}

/// Options for OpenAndCompact operations.
pub struct OpenAndCompactOptions {
    pub(crate) inner: *mut ffi::rocksdb_open_and_compact_options_t,
    // Hold reference to prevent dropping
    _canceled: Option<Arc<OpenAndCompactCanceled>>,
}

impl Default for OpenAndCompactOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenAndCompactOptions {
    pub fn new() -> Self {
        let inner = unsafe { ffi::rocksdb_open_and_compact_options_create() };
        Self {
            inner,
            _canceled: None,
        }
    }

    /// Sets the cancellation token.
    /// The compaction can be cancelled by calling `canceled.set(true)`.
    pub fn set_canceled(&mut self, canceled: Arc<OpenAndCompactCanceled>) {
        unsafe {
            ffi::rocksdb_open_and_compact_options_set_canceled(self.inner, canceled.as_ptr());
        }
        self._canceled = Some(canceled);
    }

    /// Enables or disables resumption support.
    /// When enabled, compaction progress is persisted to allow resuming after interruption.
    pub fn set_allow_resumption(&mut self, allow: bool) {
        unsafe {
            ffi::rocksdb_open_and_compact_options_set_allow_resumption(
                self.inner,
                allow as libc::c_uchar,
            );
        }
    }
}

impl Drop for OpenAndCompactOptions {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_open_and_compact_options_destroy(self.inner);
        }
    }
}

unsafe impl Send for OpenAndCompactOptions {}
unsafe impl Sync for OpenAndCompactOptions {}
/// Opens a database as secondary and runs compaction without modifying the original DB.
///
/// This read-only operation outputs compaction results to `output_directory`
/// instead of installing them back to the source database. Designed primarily
/// for use with `CompactionService` to process remote compaction jobs.
///
/// # Arguments
/// * `db_path` - Source database path
/// * `output_directory` - Where compaction output files are written  
/// * `input` - Serialized compaction input (from CompactionService::schedule)
/// * `override_options` - Configuration overrides for the operation
///
/// # Returns
/// Serialized compaction result on success
pub fn open_and_compact<P1: AsRef<Path>, P2: AsRef<Path>>(
    db_path: P1,
    output_directory: P2,
    input: &[u8],
    override_options: &CompactionServiceOptionsOverride,
) -> Result<Vec<u8>, Error> {
    let db_path_cstr = ffi_util::to_cpath(db_path)?;
    let output_dir_cstr = ffi_util::to_cpath(output_directory)?;

    let mut output_len: size_t = 0;
    let mut err: *mut libc::c_char = std::ptr::null_mut();

    let output_ptr = unsafe {
        ffi::rocksdb_open_and_compact(
            db_path_cstr.as_ptr(),
            output_dir_cstr.as_ptr(),
            input.as_ptr() as *const libc::c_char,
            input.len() as size_t,
            &mut output_len,
            override_options.inner,
            &mut err,
        )
    };

    if !err.is_null() {
        return Err(Error::new(ffi_util::error_message(err)));
    }

    if output_ptr.is_null() {
        return Err(Error::new(
            "OpenAndCompact returned null output".to_string(),
        ));
    }

    // Convert to Vec<u8> and free the C memory
    let result = unsafe {
        let slice = std::slice::from_raw_parts(output_ptr as *const u8, output_len);
        let vec = slice.to_vec();
        libc::free(output_ptr as *mut libc::c_void);
        vec
    };

    Ok(result)
}

/// Opens a database as secondary and runs compaction with custom options.
///
/// Similar to `open_and_compact` but allows additional configuration via
/// `OpenAndCompactOptions` (e.g., cancellation, resumption).
pub fn open_and_compact_with_options<P1: AsRef<Path>, P2: AsRef<Path>>(
    options: &OpenAndCompactOptions,
    db_path: P1,
    output_directory: P2,
    input: &[u8],
    override_options: &CompactionServiceOptionsOverride,
) -> Result<Vec<u8>, Error> {
    let db_path_cstr = ffi_util::to_cpath(db_path)?;
    let output_dir_cstr = ffi_util::to_cpath(output_directory)?;

    eprintln!("[DEBUG] open_and_compact_with_options called");
    eprintln!("[DEBUG] options.inner: {:?}", options.inner);
    eprintln!(
        "[DEBUG] override_options.inner: {:?}",
        override_options.inner
    );
    eprintln!("[DEBUG] db_path: {:?}", db_path_cstr);
    eprintln!("[DEBUG] output_dir: {:?}", output_dir_cstr);
    eprintln!("[DEBUG] input len: {}", input.len());

    let mut output_len: size_t = 0;
    let mut err: *mut libc::c_char = std::ptr::null_mut();

    eprintln!("[DEBUG] Calling FFI rocksdb_open_and_compact_with_options...");
    let output_ptr = unsafe {
        ffi::rocksdb_open_and_compact_with_options(
            options.inner,
            db_path_cstr.as_ptr(),
            output_dir_cstr.as_ptr(),
            input.as_ptr() as *const libc::c_char,
            input.len() as size_t,
            &mut output_len,
            override_options.inner,
            &mut err,
        )
    };
    eprintln!("[DEBUG] FFI call returned");

    if !err.is_null() {
        return Err(Error::new(ffi_util::error_message(err)));
    }

    if output_ptr.is_null() {
        return Err(Error::new(
            "OpenAndCompact returned null output".to_string(),
        ));
    }

    // Convert to Vec<u8> and free the C memory
    let result = unsafe {
        let slice = std::slice::from_raw_parts(output_ptr as *const u8, output_len);
        let vec = slice.to_vec();
        libc::free(output_ptr as *mut libc::c_void);
        vec
    };

    Ok(result)
}
