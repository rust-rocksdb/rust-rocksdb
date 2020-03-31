mod test_column_family;
mod test_compact_range;
mod test_compaction_filter;
mod test_compression;
mod test_delete_files_in_range;
mod test_delete_range;
mod test_encryption;
mod test_event_listener;
mod test_ingest_external_file;
mod test_iterator;
mod test_logger;
mod test_metadata;
mod test_multithreaded;
mod test_prefix_extractor;
mod test_rate_limiter;
mod test_read_only;
mod test_rocksdb_options;
mod test_slice_transform;
mod test_statistics;
mod test_table_properties;
mod test_table_properties_rc;
mod test_titan;
mod test_ttl;

fn tempdir_with_prefix(prefix: &str) -> tempfile::TempDir {
    tempfile::Builder::new().prefix(prefix).tempdir().expect("")
}
