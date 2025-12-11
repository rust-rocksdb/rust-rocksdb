fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/compaction.proto")?;
    tonic_build::compile_protos("proto/worker.proto")?;
    Ok(())
}
