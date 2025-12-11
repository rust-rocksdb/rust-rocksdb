//! Remote Worker Service: A gRPC service that executes compaction jobs using OpenAndCompact.
//!
//! This service:
//! 1. Receives compaction requests from the primary service
//! 2. Opens the DB in read-only mode to get options
//! 3. Calls open_and_compact to execute the compaction
//! 4. Returns the serialized compaction result

use std::path::PathBuf;

use rocksdb::{
    open_and_compact, BlockBasedOptions, Cache, CompactionServiceOptionsOverride, Env, Options,
};
// Note: We use Options::load_latest to get the DB's configuration from its OPTIONS file

use tonic::{transport::Server, Request, Response, Status};

// Include the generated protobuf code
pub mod worker {
    tonic::include_proto!("worker");
}

use worker::compaction_worker_server::{CompactionWorker, CompactionWorkerServer};
use worker::{CompactionRequest, CompactionResponse};

/// The gRPC compaction worker service implementation
pub struct CompactionWorkerImpl {
    /// Base directory for compaction output files
    output_base_dir: PathBuf,
    /// Counter for generating unique output directories
    job_counter: std::sync::atomic::AtomicU64,
}

impl CompactionWorkerImpl {
    fn new(output_base_dir: PathBuf) -> Self {
        // Create output base directory
        let _ = std::fs::create_dir_all(&output_base_dir);

        Self {
            output_base_dir,
            job_counter: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Perform the actual compaction using open_and_compact
    fn perform_compaction(
        &self,
        job_id: &str,
        db_path: &str,
        compaction_input: &[u8],
    ) -> Result<Vec<u8>, String> {
        println!("[Worker] Performing compaction for job: {}", job_id);
        println!("[Worker] DB path: {}", db_path);
        println!("[Worker] Input size: {} bytes", compaction_input.len());

        if compaction_input.is_empty() {
            return Err("Empty compaction input".to_string());
        }

        // Create unique output directory for this job
        let job_num = self
            .job_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let output_dir =
            self.output_base_dir
                .join(format!("job_{}_{}", job_num, job_id.replace("/", "_")));
        let _ = std::fs::create_dir_all(&output_dir);

        println!("[Worker] Output directory: {:?}", output_dir);

        // Load options from the DB's OPTIONS file
        // This gets the same configuration as the primary DB
        println!("[Worker] Loading options from DB's OPTIONS file...");

        let env = Env::new().expect("Failed to create Env");
        let cache = Cache::new_lru_cache(8 * 1024 * 1024); // 8MB cache

        let loaded_options = match Options::load_latest(db_path, env, true, cache) {
            Ok((opts, cf_descriptors)) => {
                println!("[Worker] Successfully loaded options from OPTIONS file");
                println!("[Worker] Found {} column families", cf_descriptors.len());
                Some(opts)
            }
            Err(e) => {
                println!("[Worker] Warning: Could not load options: {}", e);
                println!("[Worker] Using default options");
                None
            }
        };

        let mut override_opts = match loaded_options {
            Some(ref opts) => {
                println!("[Worker] Creating override options from loaded options");
                let mut ov = CompactionServiceOptionsOverride::from_options(opts);
                ov
            }
            None => {
                println!("[Worker] Using default override options");
                let opts = Options::default();

                CompactionServiceOptionsOverride::from_options(&opts)
            }
        };


        println!("[Worker] Calling open_and_compact...");

        // Execute the compaction
        let result = open_and_compact(db_path, &output_dir, compaction_input, &override_opts);

        match result {
            Ok(output) => {
                println!("[Worker] open_and_compact succeeded!");
                println!("[Worker] Output size: {} bytes", output.len());

                // List the output files for debugging
                if let Ok(entries) = std::fs::read_dir(&output_dir) {
                    println!("[Worker] Output files:");
                    for entry in entries.flatten() {
                        println!("[Worker]   {:?}", entry.path());
                    }
                }

                // DO NOT clean up! RocksDB needs to rename/move these files
                // during installation. The primary will clean up after on_installation.
                println!("[Worker] Output directory preserved: {:?}", output_dir);

                Ok(output)
            }
            Err(e) => {
                println!("[Worker] open_and_compact failed: {}", e);

                // Clean up on failure only
                let _ = std::fs::remove_dir_all(&output_dir);

                Err(e.to_string())
            }
        }
    }
}

#[tonic::async_trait]
impl CompactionWorker for CompactionWorkerImpl {
    async fn execute_compaction(
        &self,
        request: Request<CompactionRequest>,
    ) -> Result<Response<CompactionResponse>, Status> {
        let req = request.into_inner();

        println!("\n[Worker] ========================================");
        println!("[Worker] Received compaction request");
        println!("[Worker] Job ID: {}", req.job_id);
        println!("[Worker] DB Path: {}", req.db_path);
        println!("[Worker] Input size: {} bytes", req.compaction_input.len());
        println!("[Worker] ========================================\n");

        // Perform the compaction synchronously
        // In production, you might want to use spawn_blocking for this
        let result = self.perform_compaction(&req.job_id, &req.db_path, &req.compaction_input);

        let response = match result {
            Ok(output) => {
                println!("[Worker] Compaction completed successfully");
                CompactionResponse {
                    job_id: req.job_id,
                    success: true,
                    compaction_output: output,
                    error_message: String::new(),
                }
            }
            Err(e) => {
                println!("[Worker] Compaction failed: {}", e);
                CompactionResponse {
                    job_id: req.job_id,
                    success: false,
                    compaction_output: Vec::new(),
                    error_message: e,
                }
            }
        };

        Ok(Response::new(response))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a temporary directory for compaction output
    let output_dir = std::env::temp_dir().join("remote_worker_output");
    println!("[Worker] Output base directory: {:?}", output_dir);

    // Create the worker service
    let worker = CompactionWorkerImpl::new(output_dir);

    // Start the gRPC server on a different port than primary service
    let addr = "[::1]:50052".parse()?;
    println!("[Worker] Starting gRPC server on {}", addr);
    println!("[Worker] Ready to receive compaction requests...\n");

    Server::builder()
        .add_service(CompactionWorkerServer::new(worker))
        .serve(addr)
        .await?;

    Ok(())
}
