//! Primary Service: A gRPC service that opens RocksDB with remote compaction support.
//!
//! This service demonstrates how to:
//! 1. Implement the `CompactionService` trait for custom remote compaction handling
//! 2. Configure RocksDB to use the custom compaction service
//! 3. Call the remote-worker service to execute compaction via OpenAndCompact

use std::collections::HashMap;
use std::ffi::CStr;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};

use rocksdb::{
    compaction_service::{
        CompactionService, CompactionServiceJobInfo, CompactionServiceJobStatus,
        CompactionServiceScheduleResponse,
    },
    Options, DB,
};

use tonic::transport::Channel;
use uuid::Uuid;

/// Calculate the total size of a directory in bytes
fn get_dir_size(path: &std::path::Path) -> u64 {
    let mut size = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                size += std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            } else if path.is_dir() {
                size += get_dir_size(&path);
            }
        }
    }
    size
}

/// Format bytes as human-readable string
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

/// Print database size and file count
fn print_db_size(db_path: &std::path::Path, label: &str) {
    let size = get_dir_size(db_path);
    let file_count = std::fs::read_dir(db_path)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "sst")
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0);

    println!(
        "[Primary] {} DB size: {} ({} SST files)",
        label,
        format_size(size),
        file_count
    );
}

/// Print RocksDB compaction stats
fn print_compaction_stats(db: &DB, label: &str) {
    println!("\n[Primary] === {} ===", label);

    // Files per level
    println!("[Primary] Files per level:");
    for level in 0..7 {
        let prop = format!("rocksdb.num-files-at-level{}", level);
        if let Ok(Some(count)) = db.property_value(&prop) {
            if count != "0" {
                println!("[Primary]   L{}: {} files", level, count);
            }
        }
    }

    // Compaction stats
    if let Ok(Some(stats)) = db.property_value("rocksdb.stats") {
        // Extract just the compaction summary lines
        for line in stats.lines() {
            if line.contains("Compaction") || line.contains("compaction") {
                if line.len() < 200 {
                    // Skip very long lines
                    println!("[Primary] {}", line.trim());
                }
            }
        }
    }

    // Number of running compactions
    if let Ok(Some(val)) = db.property_value("rocksdb.num-running-compactions") {
        println!("[Primary] Running compactions: {}", val);
    }

    // Pending compaction bytes
    if let Ok(Some(val)) = db.property_value("rocksdb.estimate-pending-compaction-bytes") {
        let bytes: u64 = val.parse().unwrap_or(0);
        println!("[Primary] Pending compaction: {}", format_size(bytes));
    }

    println!();
}

// Include the generated protobuf code
pub mod worker {
    tonic::include_proto!("worker");
}

use worker::compaction_worker_client::CompactionWorkerClient;
use worker::CompactionRequest;

/// Represents a pending compaction job
struct PendingJob {
    /// Serialized compaction input from RocksDB
    input: Vec<u8>,
    /// Database path for this job
    db_path: PathBuf,
    /// Condition variable to signal job completion
    completed: Arc<(Mutex<Option<CompactionResult>>, Condvar)>,
}

/// Result of a completed compaction job
struct CompactionResult {
    status: CompactionServiceJobStatus,
    output: Vec<u8>,
}

/// Shared state between the CompactionService implementation and async runtime
struct SharedState {
    /// Pending jobs waiting to be processed
    pending_jobs: HashMap<String, PendingJob>,
    /// Database path for compaction workers
    db_path: PathBuf,
    /// Whether we should cancel all pending jobs
    cancelled: bool,
    /// Tokio runtime handle for spawning async tasks
    runtime: Option<tokio::runtime::Handle>,
    /// gRPC client to the remote worker
    worker_client: Option<CompactionWorkerClient<Channel>>,
    /// Compaction statistics
    scheduled_count: u32,
    completed_success: u32,
    completed_failure: u32,
    completed_local: u32,
}

/// Our custom CompactionService implementation that calls the remote worker
pub struct RemoteCompactionService {
    /// The service name (must be static for FFI)
    name: &'static CStr,
    /// Shared state
    state: Arc<Mutex<SharedState>>,
}

impl RemoteCompactionService {
    fn new(state: Arc<Mutex<SharedState>>) -> Self {
        Self {
            name: c"RemoteCompactionService",
            state,
        }
    }
}

impl CompactionService for RemoteCompactionService {
    fn schedule(
        &self,
        job_info: &CompactionServiceJobInfo,
        compaction_service_input: &[u8],
    ) -> Result<CompactionServiceScheduleResponse, rocksdb::Error> {
        let job_id = Uuid::new_v4().to_string();

        let db_name = job_info.db_name().unwrap_or_else(|_| "unknown".to_string());
        let cf_name = job_info.cf_name().unwrap_or_else(|_| "unknown".to_string());

        println!("\n[CompactionService] ========================================");
        println!("[CompactionService] Schedule called - job_id={}", job_id);
        println!("[CompactionService]   db={}, cf={}", db_name, cf_name);
        println!(
            "[CompactionService]   input_level={}, output_level={}",
            job_info.base_input_level(),
            job_info.output_level()
        );
        println!(
            "[CompactionService]   input_size={} bytes",
            compaction_service_input.len()
        );
        println!("[CompactionService] ========================================\n");

        let mut state = self.state.lock().unwrap();

        // Check if cancelled
        if state.cancelled {
            println!("[CompactionService] Service cancelled, using local compaction");
            return CompactionServiceScheduleResponse::with_status(
                CompactionServiceJobStatus::UseLocal,
            );
        }

        // Store the job for the wait() call to process
        let pending_job = PendingJob {
            input: compaction_service_input.to_vec(),
            db_path: state.db_path.clone(),
            completed: Arc::new((Mutex::new(None), Condvar::new())),
        };

        state.pending_jobs.insert(job_id.clone(), pending_job);
        state.scheduled_count += 1;

        println!(
            "[CompactionService] Job {} scheduled (total scheduled: {})",
            job_id, state.scheduled_count
        );

        CompactionServiceScheduleResponse::new(&job_id, CompactionServiceJobStatus::Success)
    }

    fn wait(
        &self,
        scheduled_job_id: &str,
        result: &mut Vec<u8>,
    ) -> Result<CompactionServiceJobStatus, rocksdb::Error> {
        println!(
            "\n[CompactionService] Wait called for job: {}",
            scheduled_job_id
        );

        // Get job info and runtime handle
        let (input, db_path, runtime, worker_client) = {
            let state = self.state.lock().unwrap();

            let job = match state.pending_jobs.get(scheduled_job_id) {
                Some(job) => job,
                None => {
                    println!(
                        "[CompactionService] Job {} not found, using local",
                        scheduled_job_id
                    );
                    return Ok(CompactionServiceJobStatus::UseLocal);
                }
            };

            let runtime = match &state.runtime {
                Some(rt) => rt.clone(),
                None => {
                    println!("[CompactionService] No runtime available, using local");
                    return Ok(CompactionServiceJobStatus::UseLocal);
                }
            };

            let client = match &state.worker_client {
                Some(c) => c.clone(),
                None => {
                    println!("[CompactionService] No worker client available, using local");
                    return Ok(CompactionServiceJobStatus::UseLocal);
                }
            };

            (job.input.clone(), job.db_path.clone(), runtime, client)
        };

        println!(
            "[CompactionService] Calling remote worker for job: {}",
            scheduled_job_id
        );
        println!("[CompactionService]   DB path: {:?}", db_path);
        println!("[CompactionService]   Input size: {} bytes", input.len());

        // Call the remote worker using the tokio runtime
        let job_id = scheduled_job_id.to_string();
        let db_path_str = db_path.to_string_lossy().to_string();

        let worker_result = runtime.block_on(async {
            let mut client = worker_client;

            let request = tonic::Request::new(CompactionRequest {
                job_id: job_id.clone(),
                compaction_input: input,
                db_path: db_path_str,
            });

            println!("[CompactionService] Sending request to remote worker...");

            match client.execute_compaction(request).await {
                Ok(response) => {
                    let resp = response.into_inner();
                    println!("[CompactionService] Remote worker responded:");
                    println!("[CompactionService]   success={}", resp.success);
                    println!(
                        "[CompactionService]   output_size={} bytes",
                        resp.compaction_output.len()
                    );
                    if !resp.error_message.is_empty() {
                        println!("[CompactionService]   error={}", resp.error_message);
                    }
                    Ok(resp)
                }
                Err(e) => {
                    println!("[CompactionService] Failed to call remote worker: {}", e);
                    Err(e)
                }
            }
        });

        // Process the result
        match worker_result {
            Ok(response) if response.success => {
                *result = response.compaction_output;
                println!(
                    "[CompactionService] Job {} completed successfully via remote worker",
                    scheduled_job_id
                );
                Ok(CompactionServiceJobStatus::Success)
            }
            Ok(response) => {
                println!(
                    "[CompactionService] Remote worker failed: {}, falling back to local",
                    response.error_message
                );
                Ok(CompactionServiceJobStatus::UseLocal)
            }
            Err(_) => {
                println!("[CompactionService] Remote worker call failed, falling back to local");
                Ok(CompactionServiceJobStatus::UseLocal)
            }
        }
    }

    fn cancel_awaiting_jobs(&self) {
        println!("[CompactionService] cancel_awaiting_jobs called");

        let mut state = self.state.lock().unwrap();
        state.cancelled = true;

        // Wake up all waiting jobs
        for (job_id, job) in state.pending_jobs.iter() {
            println!("[CompactionService] Cancelling job: {}", job_id);
            let (lock, cvar) = &*job.completed;
            let mut result = lock.lock().unwrap();
            *result = Some(CompactionResult {
                status: CompactionServiceJobStatus::Aborted,
                output: Vec::new(),
            });
            cvar.notify_all();
        }
    }

    fn on_installation(&self, scheduled_job_id: &str, status: CompactionServiceJobStatus) {
        // Clean up the completed job and update stats
        let mut state = self.state.lock().unwrap();
        state.pending_jobs.remove(scheduled_job_id);

        match status {
            CompactionServiceJobStatus::Success => state.completed_success += 1,
            CompactionServiceJobStatus::Failure => state.completed_failure += 1,
            CompactionServiceJobStatus::UseLocal => state.completed_local += 1,
            _ => {}
        }

        println!(
            "[CompactionService] ✓ on_installation - job_id={}, status={:?}",
            scheduled_job_id, status
        );
        println!(
            "[CompactionService]   Stats: scheduled={}, success={}, failure={}, local={}",
            state.scheduled_count,
            state.completed_success,
            state.completed_failure,
            state.completed_local
        );
    }

    fn name(&self) -> &CStr {
        self.name
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Primary Service with Remote Compaction ===\n");

    // Connect to the remote worker
    println!("[Primary] Connecting to remote worker at http://[::1]:50052...");
    let worker_client = match CompactionWorkerClient::connect("http://[::1]:50052").await {
        Ok(client) => {
            println!("[Primary] Connected to remote worker!");
            Some(client)
        }
        Err(e) => {
            println!(
                "[Primary] Warning: Could not connect to remote worker: {}",
                e
            );
            println!("[Primary] Compaction will fall back to local execution.");
            None
        }
    };

    // Create a temporary directory for the database
    let temp_dir = tempfile::Builder::new()
        .prefix("primary_service_db")
        .tempdir()?;

    let db_path = temp_dir.path().to_path_buf();
    println!("[Primary] Database path: {:?}", db_path);

    // Create shared state with the runtime handle and worker client
    let shared_state = Arc::new(Mutex::new(SharedState {
        pending_jobs: HashMap::new(),
        db_path: db_path.clone(),
        cancelled: false,
        runtime: Some(tokio::runtime::Handle::current()),
        worker_client,
        scheduled_count: 0,
        completed_success: 0,
        completed_failure: 0,
        completed_local: 0,
    }));

    // Create the compaction service
    let compaction_service = RemoteCompactionService::new(shared_state.clone());

    // Configure RocksDB options - same as test_remote_compaction.rs
    let mut opts = Options::default();
    opts.create_if_missing(true);

    // Extremely small buffers to trigger compaction quickly
    opts.set_write_buffer_size(4 * 1024); // 4KB
    opts.set_target_file_size_base(4 * 1024); // 4KB

    // Enable compaction writes to go through the compaction service
    opts.set_compaction_service(compaction_service);

    // Open the database
    let db = DB::open(&opts, &db_path)?;
    println!("[Primary] RocksDB opened successfully\n");

    // Print initial DB size
    print_db_size(&db_path, "Initial");

    // Write data in batches to trigger compaction
    println!("\n[Primary] Writing data to trigger compaction...");

    for batch in 0..10 {
        for i in 0..100000 {
            let idx = batch * 10000 + i;
            let key = format!("key_{:06}", idx);
            let value = format!("value_{:06}_{}", idx, "x".repeat(100));
            db.put(key.as_bytes(), value.as_bytes())?;
        }
        db.flush()?;
        print_db_size(&db_path, &format!("After batch {}", batch));
    }

    // Show stats before waiting
    print_compaction_stats(&db, "Before waiting for compaction");

    println!("[Primary] Waiting for background compaction to complete...");
    std::thread::sleep(std::time::Duration::from_secs(5));

    // Show stats after compaction
    print_compaction_stats(&db, "After compaction");
    print_db_size(&db_path, "After compaction");

    // Verify data integrity
    println!("\n[Primary] Verifying data integrity...");
    for i in [0, 1000, 5000, 9999] {
        let key = format!("key_{:06}", i);
        let val = db.get(key.as_bytes())?;
        assert!(val.is_some(), "Key {} should exist", key);
    }
    println!("[Primary] Data verification passed!");

    // Final size
    print_db_size(&db_path, "Final");

    // Print compaction summary
    {
        let state = shared_state.lock().unwrap();
        println!("\n=== Compaction Summary ===");
        println!("  Scheduled:        {}", state.scheduled_count);
        println!("  Completed (remote): {}", state.completed_success);
        println!("  Completed (local):  {}", state.completed_local);
        println!("  Failed:           {}", state.completed_failure);

        if state.completed_success > 0 {
            println!(
                "\n✅ Remote compaction worked! {} jobs completed via remote worker.",
                state.completed_success
            );
        } else if state.scheduled_count > 0 {
            println!("\n⚠️  Compactions were scheduled but none completed via remote worker.");
        } else {
            println!("\n❌ No compactions were triggered.");
        }
    }

    println!("\n=== Primary Service Complete ===\n");

    // Keep db in scope
    drop(db);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compaction_service_name() {
        let state = Arc::new(Mutex::new(SharedState {
            pending_jobs: HashMap::new(),
            db_path: PathBuf::from("/tmp/test"),
            cancelled: false,
            runtime: None,
            worker_client: None,
            scheduled_count: 0,
            completed_success: 0,
            completed_failure: 0,
            completed_local: 0,
        }));

        let service = RemoteCompactionService::new(state);
        assert_eq!(service.name().to_str().unwrap(), "RemoteCompactionService");
    }
}
