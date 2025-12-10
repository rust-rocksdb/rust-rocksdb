use std::ffi::CStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use rocksdb::compaction_service::{
    CompactionService, CompactionServiceJobInfo, CompactionServiceJobStatus,
    CompactionServiceScheduleResponse,
};
use rocksdb::{Error, Options, DB};

mod util;
use util::DBPath;

// ============================================================================
// Test CompactionService Implementation
// ============================================================================

/// A test compaction service that tracks all callback invocations
struct TestCompactionService {
    name: &'static CStr,
    schedule_count: Arc<AtomicUsize>,
    wait_count: Arc<AtomicUsize>,
    cancel_count: Arc<AtomicUsize>,
    on_installation_count: Arc<AtomicUsize>,
}

impl TestCompactionService {
    fn new(
        schedule_count: Arc<AtomicUsize>,
        wait_count: Arc<AtomicUsize>,
        cancel_count: Arc<AtomicUsize>,
        on_installation_count: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            name: c"TestCompactionService",
            schedule_count,
            wait_count,
            cancel_count,
            on_installation_count,
        }
    }
}

impl CompactionService for TestCompactionService {
    fn schedule(
        &self,
        job_info: &CompactionServiceJobInfo,
        _compaction_service_input: &[u8],
    ) -> Result<CompactionServiceScheduleResponse, Error> {
        self.schedule_count.fetch_add(1, Ordering::SeqCst);

        // Extract job info to verify it's accessible
        let _db_name = job_info.db_name()?;
        let _cf_name = job_info.cf_name()?;
        let job_id = job_info.job_id();
        let _reason = job_info.compaction_reason();

        let scheduled_job_id = format!("test-job-{}", job_id);

        // Return UseLocal to let RocksDB perform the compaction locally
        // This allows the test to complete without implementing remote compaction
        CompactionServiceScheduleResponse::new(&scheduled_job_id, CompactionServiceJobStatus::UseLocal)
    }

    fn wait(&self, _scheduled_job_id: &str) -> Result<CompactionServiceJobStatus, Error> {
        self.wait_count.fetch_add(1, Ordering::SeqCst);
        Ok(CompactionServiceJobStatus::Success)
    }

    fn cancel_awaiting_jobs(&self) {
        self.cancel_count.fetch_add(1, Ordering::SeqCst);
    }

    fn on_installation(&self, _scheduled_job_id: &str, _status: CompactionServiceJobStatus) {
        self.on_installation_count.fetch_add(1, Ordering::SeqCst);
    }

    fn name(&self) -> &CStr {
        self.name
    }
}

// ============================================================================
// CompactionServiceScheduleResponse Tests
// ============================================================================

#[test]
fn test_schedule_response_create_with_job_id() {
    let response = CompactionServiceScheduleResponse::new(
        "test-job-123",
        CompactionServiceJobStatus::Success,
    )
    .expect("Failed to create schedule response");

    assert_eq!(response.status(), CompactionServiceJobStatus::Success);
    assert_eq!(response.scheduled_job_id().unwrap(), "test-job-123");
}

#[test]
fn test_schedule_response_create_with_status_only() {
    let response =
        CompactionServiceScheduleResponse::with_status(CompactionServiceJobStatus::Failure)
            .expect("Failed to create schedule response");

    assert_eq!(response.status(), CompactionServiceJobStatus::Failure);
}

#[test]
fn test_schedule_response_different_statuses() {
    let success =
        CompactionServiceScheduleResponse::with_status(CompactionServiceJobStatus::Success)
            .expect("Failed to create success response");
    assert_eq!(success.status(), CompactionServiceJobStatus::Success);

    let failure =
        CompactionServiceScheduleResponse::with_status(CompactionServiceJobStatus::Failure)
            .expect("Failed to create failure response");
    assert_eq!(failure.status(), CompactionServiceJobStatus::Failure);

    let aborted =
        CompactionServiceScheduleResponse::with_status(CompactionServiceJobStatus::Aborted)
            .expect("Failed to create aborted response");
    assert_eq!(aborted.status(), CompactionServiceJobStatus::Aborted);

    let use_local =
        CompactionServiceScheduleResponse::with_status(CompactionServiceJobStatus::UseLocal)
            .expect("Failed to create use_local response");
    assert_eq!(use_local.status(), CompactionServiceJobStatus::UseLocal);
}

#[test]
fn test_schedule_response_drop() {
    // Test that Drop is properly implemented (no memory leak or crash)
    {
        let _response = CompactionServiceScheduleResponse::new(
            "ephemeral-job",
            CompactionServiceJobStatus::Success,
        )
        .expect("Failed to create schedule response");
        // response goes out of scope and should be dropped
    }
    // If we get here without crashing, the test passes
}

// ============================================================================
// CompactionService Integration Tests
// ============================================================================

#[test]
fn test_compaction_service_cancel_called_on_db_close() {
    let path = DBPath::new("_rust_rocksdb_compaction_service_cancel_test");

    let schedule_count = Arc::new(AtomicUsize::new(0));
    let wait_count = Arc::new(AtomicUsize::new(0));
    let cancel_count = Arc::new(AtomicUsize::new(0));
    let on_installation_count = Arc::new(AtomicUsize::new(0));

    let cancel_count_clone = cancel_count.clone();

    {
        let service = TestCompactionService::new(
            schedule_count.clone(),
            wait_count.clone(),
            cancel_count.clone(),
            on_installation_count.clone(),
        );

        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_compaction_service(service);

        let db = DB::open(&opts, &path).expect("Failed to open DB");

        // Write some data
        db.put(b"key1", b"value1").expect("Failed to put");

        // DB will be dropped here, which should trigger cancel_awaiting_jobs
    }

    // Verify cancel_awaiting_jobs was called when DB closed
    assert!(
        cancel_count_clone.load(Ordering::SeqCst) >= 1,
        "cancel_awaiting_jobs should be called when DB closes"
    );
}

#[test]
fn test_compaction_service_schedule_called_on_compaction() {
    let path = DBPath::new("_rust_rocksdb_compaction_service_schedule_test");

    let schedule_count = Arc::new(AtomicUsize::new(0));
    let wait_count = Arc::new(AtomicUsize::new(0));
    let cancel_count = Arc::new(AtomicUsize::new(0));
    let on_installation_count = Arc::new(AtomicUsize::new(0));

    let schedule_count_clone = schedule_count.clone();

    {
        let service = TestCompactionService::new(
            schedule_count.clone(),
            wait_count.clone(),
            cancel_count.clone(),
            on_installation_count.clone(),
        );

        let mut opts = Options::default();
        opts.create_if_missing(true);

        // Set VERY aggressive compaction settings to trigger compaction quickly
        opts.set_write_buffer_size(4 * 1024); // 4KB
        opts.set_target_file_size_base(4 * 1024); // 4KB
        opts.set_max_write_buffer_number(2);
        opts.set_min_write_buffer_number_to_merge(1);
        opts.set_level_zero_file_num_compaction_trigger(2);
        opts.set_level_zero_slowdown_writes_trigger(4);
        opts.set_level_zero_stop_writes_trigger(8);
        opts.set_max_background_jobs(4);

        opts.set_compaction_service(service);

        let db = DB::open(&opts, &path).expect("Failed to open DB");

        // Write lots of data in batches, flushing after each to create L0 files
        for batch in 0..5 {
            for i in 0..1000 {
                let idx = batch * 100 + i;
                let key = format!("key_{:06}", idx);
                let value = format!("value_{:06}_{}", idx, "x".repeat(100));
                db.put(key.as_bytes(), value.as_bytes()).expect("Failed to put");
            }
            db.flush().expect("Failed to flush");
            // Small sleep to allow background compaction to kick in
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        // Wait for background compaction to complete
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    // Verify schedule was called at least once
    let final_schedule_count = schedule_count_clone.load(Ordering::SeqCst);
    assert!(
        final_schedule_count >= 1,
        "schedule should be called during compaction, got {} calls",
        final_schedule_count
    );
}

#[test]
fn test_compaction_service_manual_compaction() {
    let path = DBPath::new("_rust_rocksdb_compaction_service_manual_test");

    let schedule_count = Arc::new(AtomicUsize::new(0));
    let wait_count = Arc::new(AtomicUsize::new(0));
    let cancel_count = Arc::new(AtomicUsize::new(0));
    let on_installation_count = Arc::new(AtomicUsize::new(0));

    let schedule_count_clone = schedule_count.clone();

    {
        let service = TestCompactionService::new(
            schedule_count.clone(),
            wait_count.clone(),
            cancel_count.clone(),
            on_installation_count.clone(),
        );

        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_write_buffer_size(4 * 1024); // 4KB - small to create multiple SST files
        opts.set_target_file_size_base(4 * 1024); // 4KB
        opts.set_disable_auto_compactions(true); // Disable auto, we'll trigger manually

        opts.set_compaction_service(service);

        let db = DB::open(&opts, &path).expect("Failed to open DB");

        // Write data in multiple batches and flush each to create multiple L0 files
        for batch in 0..10 {
            for i in 0..1000 {
                let idx = batch * 100 + i;
                let key = format!("key_{:05}", idx);
                let value = format!("value_{:05}_{}", idx, "x".repeat(100));
                db.put(key.as_bytes(), value.as_bytes()).expect("Failed to put");
            }
            db.flush().expect("Failed to flush");
        }

        // Trigger manual compaction - should trigger compaction service
        db.compact_range::<&[u8], &[u8]>(None, None);
    }

    // Manual compaction should have triggered the service
    let final_schedule_count = schedule_count_clone.load(Ordering::SeqCst);
    assert!(
        final_schedule_count >= 1,
        "schedule should be called during manual compaction, got {} calls",
        final_schedule_count
    );
}

#[test]
fn test_compaction_service_data_integrity() {
    let path = DBPath::new("_rust_rocksdb_compaction_service_integrity_test");

    let schedule_count = Arc::new(AtomicUsize::new(0));
    let wait_count = Arc::new(AtomicUsize::new(0));
    let cancel_count = Arc::new(AtomicUsize::new(0));
    let on_installation_count = Arc::new(AtomicUsize::new(0));

    {
        let service = TestCompactionService::new(
            schedule_count.clone(),
            wait_count.clone(),
            cancel_count.clone(),
            on_installation_count.clone(),
        );

        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_write_buffer_size(4 * 1024);
        opts.set_target_file_size_base(4 * 1024);
        opts.set_level_zero_file_num_compaction_trigger(2);
        opts.set_max_background_jobs(4);

        opts.set_compaction_service(service);

        let db = DB::open(&opts, &path).expect("Failed to open DB");

        // Write data
        let mut expected_data = Vec::new();
        for i in 0..200 {
            let key = format!("key_{:05}", i);
            let value = format!("value_{:05}", i);
            db.put(key.as_bytes(), value.as_bytes()).expect("Failed to put");
            expected_data.push((key, value));

            if i % 20 == 19 {
                db.flush().expect("Failed to flush");
            }
        }

        // Wait for any background compaction
        std::thread::sleep(std::time::Duration::from_millis(500));

        // Verify all data is still readable and correct
        for (key, expected_value) in &expected_data {
            let result = db.get(key.as_bytes()).expect("Failed to get");
            assert!(result.is_some(), "Key {} should exist", key);
            assert_eq!(
                result.unwrap(),
                expected_value.as_bytes(),
                "Value mismatch for key {}",
                key
            );
        }
    }
}
