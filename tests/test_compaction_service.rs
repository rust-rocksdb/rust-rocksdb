use rocksdb::compaction_service::{CompactionServiceJobStatus, CompactionServiceScheduleResponse};

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
    let success = CompactionServiceScheduleResponse::with_status(CompactionServiceJobStatus::Success)
        .expect("Failed to create success response");
    assert_eq!(success.status(), CompactionServiceJobStatus::Success);

    let failure = CompactionServiceScheduleResponse::with_status(CompactionServiceJobStatus::Failure)
        .expect("Failed to create failure response");
    assert_eq!(failure.status(), CompactionServiceJobStatus::Failure);

    let aborted = CompactionServiceScheduleResponse::with_status(CompactionServiceJobStatus::Aborted)
        .expect("Failed to create aborted response");
    assert_eq!(aborted.status(), CompactionServiceJobStatus::Aborted);

    let use_local = CompactionServiceScheduleResponse::with_status(CompactionServiceJobStatus::UseLocal)
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

