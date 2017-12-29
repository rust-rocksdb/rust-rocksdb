extern crate cc;

pub fn link(name: &str, bundled: bool) {
    use std::env::var;
    let target = var("TARGET").unwrap();
    let target: Vec<_> = target.split('-').collect();
    if target.get(2) == Some(&"windows") {
        println!("cargo:rustc-link-lib=dylib={}", name);
        if bundled && target.get(3) == Some(&"gnu") {
            let dir = var("CARGO_MANIFEST_DIR").unwrap();
            println!("cargo:rustc-link-search=native={}/{}", dir, target[0]);
        }
    }
}

fn main() {
	let mut config = cc::Build::new();
	config.include("rocksdb/include/");
	config.include("rocksdb/");
	config.include("snappy/");
	config.include(".");

	config.define("NDEBUG", Some("1"));
	config.define("SNAPPY", Some("1"));

	if cfg!(target_os = "macos") {
		config.define("OS_MACOSX", Some("1"));

	}
	if cfg!(target_os = "linux") {
		config.define("OS_LINUX", Some("1"));
        //COMMON_FLAGS="$COMMON_FLAGS -fno-builtin-memcmp"
	}
	if cfg!(target_os = "freebsd") {
		config.define("OS_FREEBSD", Some("1"));
	}

	if cfg!(windows) {
		link("rpcrt4", false);
		config.define("OS_WIN", Some("1"));
		config.file("rocksdb/port/win/env_default.cc");
		config.file("rocksdb/port/win/env_win.cc");
		config.file("rocksdb/port/win/io_win.cc");
		config.file("rocksdb/port/win/port_win.cc");
		config.file("rocksdb/port/win/win_logger.cc");
		config.file("rocksdb/port/win/win_thread.cc");
	} else {
		config.define("ROCKSDB_PLATFORM_POSIX", Some("1"));
		config.define("ROCKSDB_LIB_IO_POSIX", Some("1"));
		config.file("rocksdb/env/env_posix.cc");
		config.file("rocksdb/env/io_posix.cc");
		config.file("rocksdb/port/port_posix.cc");
	}

	if !cfg!(target_env = "msvc") {
		config.flag("-std=c++11");
	} else {
		config.flag("-EHsc");
	}

	config.file("rocksdb/cache/clock_cache.cc");
	config.file("rocksdb/cache/lru_cache.cc");
	config.file("rocksdb/cache/sharded_cache.cc");

	config.file("rocksdb/db/builder.cc");
	config.file("rocksdb/db/c.cc");
	config.file("rocksdb/db/column_family.cc");
	config.file("rocksdb/db/compacted_db_impl.cc");
	config.file("rocksdb/db/compaction.cc");
	config.file("rocksdb/db/compaction_iterator.cc");
	config.file("rocksdb/db/compaction_job.cc");
	config.file("rocksdb/db/compaction_picker.cc");
	config.file("rocksdb/db/compaction_picker_universal.cc");
	config.file("rocksdb/db/convenience.cc");
	config.file("rocksdb/db/db_filesnapshot.cc");
	config.file("rocksdb/db/db_impl.cc");
	config.file("rocksdb/db/db_impl_compaction_flush.cc");
	config.file("rocksdb/db/db_impl_experimental.cc");
	config.file("rocksdb/db/db_impl_files.cc");
	config.file("rocksdb/db/db_impl_open.cc");
	config.file("rocksdb/db/db_impl_readonly.cc");
	config.file("rocksdb/db/db_impl_write.cc");
	config.file("rocksdb/db/db_info_dumper.cc");
	config.file("rocksdb/db/db_iter.cc");
	config.file("rocksdb/db/dbformat.cc");
	config.file("rocksdb/db/event_helpers.cc");
	config.file("rocksdb/db/experimental.cc");
	config.file("rocksdb/db/external_sst_file_ingestion_job.cc");
	config.file("rocksdb/db/file_indexer.cc");
	config.file("rocksdb/db/flush_job.cc");
	config.file("rocksdb/db/flush_scheduler.cc");
	config.file("rocksdb/db/forward_iterator.cc");
	config.file("rocksdb/db/internal_stats.cc");
	config.file("rocksdb/db/log_reader.cc");
	config.file("rocksdb/db/log_writer.cc");
	config.file("rocksdb/db/malloc_stats.cc");
	config.file("rocksdb/db/managed_iterator.cc");
	config.file("rocksdb/db/memtable.cc");
	config.file("rocksdb/db/memtable_list.cc");
	config.file("rocksdb/db/merge_helper.cc");
	config.file("rocksdb/db/merge_operator.cc");
	config.file("rocksdb/db/range_del_aggregator.cc");
	config.file("rocksdb/db/repair.cc");
	config.file("rocksdb/db/snapshot_impl.cc");
	config.file("rocksdb/db/table_cache.cc");
	config.file("rocksdb/db/table_properties_collector.cc");
	config.file("rocksdb/db/transaction_log_impl.cc");
	config.file("rocksdb/db/version_builder.cc");
	config.file("rocksdb/db/version_edit.cc");
	config.file("rocksdb/db/version_set.cc");
	config.file("rocksdb/db/wal_manager.cc");
	config.file("rocksdb/db/write_batch.cc");
	config.file("rocksdb/db/write_batch_base.cc");
	config.file("rocksdb/db/write_controller.cc");
	config.file("rocksdb/db/write_thread.cc");

	config.file("rocksdb/env/env.cc");
	config.file("rocksdb/env/env_chroot.cc");
	config.file("rocksdb/env/env_encryption.cc");
	config.file("rocksdb/env/env_hdfs.cc");
	config.file("rocksdb/env/mock_env.cc");

	config.file("rocksdb/memtable/alloc_tracker.cc");
	config.file("rocksdb/memtable/hash_cuckoo_rep.cc");
	config.file("rocksdb/memtable/hash_linklist_rep.cc");
	config.file("rocksdb/memtable/hash_skiplist_rep.cc");
	config.file("rocksdb/memtable/skiplistrep.cc");
	config.file("rocksdb/memtable/vectorrep.cc");
	config.file("rocksdb/memtable/write_buffer_manager.cc");

	config.file("rocksdb/monitoring/histogram.cc");
	config.file("rocksdb/monitoring/histogram_windowing.cc");
	config.file("rocksdb/monitoring/instrumented_mutex.cc");
	config.file("rocksdb/monitoring/iostats_context.cc");
	config.file("rocksdb/monitoring/perf_context.cc");
	config.file("rocksdb/monitoring/perf_level.cc");
	config.file("rocksdb/monitoring/statistics.cc");
	config.file("rocksdb/monitoring/thread_status_impl.cc");
	config.file("rocksdb/monitoring/thread_status_updater.cc");
	config.file("rocksdb/monitoring/thread_status_util.cc");

	config.file("rocksdb/options/cf_options.cc");
	config.file("rocksdb/options/db_options.cc");
	config.file("rocksdb/options/options.cc");
	config.file("rocksdb/options/options_helper.cc");
	config.file("rocksdb/options/options_parser.cc");
	config.file("rocksdb/options/options_sanity_check.cc");

	config.file("rocksdb/port/stack_trace.cc");

	config.file("rocksdb/table/adaptive_table_factory.cc");
	config.file("rocksdb/table/block.cc");
	config.file("rocksdb/table/block_based_filter_block.cc");
	config.file("rocksdb/table/block_based_table_builder.cc");
	config.file("rocksdb/table/block_based_table_factory.cc");
	config.file("rocksdb/table/block_based_table_reader.cc");
	config.file("rocksdb/table/block_builder.cc");
	config.file("rocksdb/table/block_prefix_index.cc");
	config.file("rocksdb/table/bloom_block.cc");
	config.file("rocksdb/table/cuckoo_table_builder.cc");
	config.file("rocksdb/table/cuckoo_table_factory.cc");
	config.file("rocksdb/table/cuckoo_table_reader.cc");
	config.file("rocksdb/table/flush_block_policy.cc");
	config.file("rocksdb/table/format.cc");
	config.file("rocksdb/table/full_filter_block.cc");
	config.file("rocksdb/table/get_context.cc");
	config.file("rocksdb/table/index_builder.cc");
	config.file("rocksdb/table/iterator.cc");
	config.file("rocksdb/table/merging_iterator.cc");
	config.file("rocksdb/table/meta_blocks.cc");
	config.file("rocksdb/table/partitioned_filter_block.cc");
	config.file("rocksdb/table/persistent_cache_helper.cc");
	config.file("rocksdb/table/plain_table_builder.cc");
	config.file("rocksdb/table/plain_table_factory.cc");
	config.file("rocksdb/table/plain_table_index.cc");
	config.file("rocksdb/table/plain_table_key_coding.cc");
	config.file("rocksdb/table/plain_table_reader.cc");
	config.file("rocksdb/table/sst_file_writer.cc");
	config.file("rocksdb/table/table_properties.cc");
	config.file("rocksdb/table/two_level_iterator.cc");

	config.file("rocksdb/util/arena.cc");
	config.file("rocksdb/util/auto_roll_logger.cc");
	config.file("rocksdb/util/bloom.cc");
	config.file("rocksdb/util/coding.cc");
	config.file("rocksdb/util/compaction_job_stats_impl.cc");
	config.file("rocksdb/util/comparator.cc");
	config.file("rocksdb/util/concurrent_arena.cc");
	config.file("rocksdb/util/crc32c.cc");
	config.file("rocksdb/util/delete_scheduler.cc");
	config.file("rocksdb/util/dynamic_bloom.cc");
	config.file("rocksdb/util/event_logger.cc");
	config.file("rocksdb/util/file_reader_writer.cc");
	config.file("rocksdb/util/file_util.cc");
	config.file("rocksdb/util/filename.cc");
	config.file("rocksdb/util/filter_policy.cc");
	config.file("rocksdb/util/hash.cc");
	config.file("rocksdb/util/log_buffer.cc");
	config.file("rocksdb/util/murmurhash.cc");
	config.file("rocksdb/util/random.cc");
	config.file("rocksdb/util/rate_limiter.cc");
	config.file("rocksdb/util/slice.cc");
	config.file("rocksdb/util/sst_file_manager_impl.cc");
	config.file("rocksdb/util/status.cc");
	config.file("rocksdb/util/status_message.cc");
	config.file("rocksdb/util/string_util.cc");
	config.file("rocksdb/util/sync_point.cc");
	config.file("rocksdb/util/thread_local.cc");
	config.file("rocksdb/util/threadpool_imp.cc");
	config.file("rocksdb/util/xxhash.cc");

	config.file("rocksdb/utilities/backupable/backupable_db.cc");
	config.file("rocksdb/utilities/blob_db/blob_db.cc");
	config.file("rocksdb/utilities/blob_db/blob_db_impl.cc");
	config.file("rocksdb/utilities/blob_db/blob_dump_tool.cc");
	config.file("rocksdb/utilities/blob_db/blob_file.cc");
	config.file("rocksdb/utilities/blob_db/blob_log_format.cc");
	config.file("rocksdb/utilities/blob_db/blob_log_reader.cc");
	config.file("rocksdb/utilities/blob_db/blob_log_writer.cc");
	config.file("rocksdb/utilities/blob_db/ttl_extractor.cc");
	config.file("rocksdb/utilities/checkpoint/checkpoint_impl.cc");
	config.file("rocksdb/utilities/col_buf_decoder.cc");
	config.file("rocksdb/utilities/col_buf_encoder.cc");
	config.file("rocksdb/utilities/column_aware_encoding_util.cc");
	config.file("rocksdb/utilities/compaction_filters/remove_emptyvalue_compactionfilter.cc");
	config.file("rocksdb/utilities/convenience/info_log_finder.cc");
	config.file("rocksdb/utilities/date_tiered/date_tiered_db_impl.cc");
	config.file("rocksdb/utilities/debug.cc");
	config.file("rocksdb/utilities/document/document_db.cc");
	config.file("rocksdb/utilities/document/json_document.cc");
	config.file("rocksdb/utilities/document/json_document_builder.cc");
	config.file("rocksdb/utilities/env_mirror.cc");
	config.file("rocksdb/utilities/env_timed.cc");
	config.file("rocksdb/utilities/geodb/geodb_impl.cc");
	config.file("rocksdb/utilities/leveldb_options/leveldb_options.cc");
	config.file("rocksdb/utilities/lua/rocks_lua_compaction_filter.cc");
	config.file("rocksdb/utilities/memory/memory_util.cc");
	config.file("rocksdb/utilities/merge_operators/max.cc");
	config.file("rocksdb/utilities/merge_operators/put.cc");
	config.file("rocksdb/utilities/merge_operators/string_append/stringappend.cc");
	config.file("rocksdb/utilities/merge_operators/string_append/stringappend2.cc");
	config.file("rocksdb/utilities/merge_operators/uint64add.cc");
	config.file("rocksdb/utilities/option_change_migration/option_change_migration.cc");
	config.file("rocksdb/utilities/options/options_util.cc");
	config.file("rocksdb/utilities/persistent_cache/block_cache_tier.cc");
	config.file("rocksdb/utilities/persistent_cache/block_cache_tier_file.cc");
	config.file("rocksdb/utilities/persistent_cache/block_cache_tier_metadata.cc");
	config.file("rocksdb/utilities/persistent_cache/persistent_cache_tier.cc");
	config.file("rocksdb/utilities/persistent_cache/volatile_tier_impl.cc");
	config.file("rocksdb/utilities/redis/redis_lists.cc");
	config.file("rocksdb/utilities/simulator_cache/sim_cache.cc");
	config.file("rocksdb/utilities/spatialdb/spatial_db.cc");
	config.file("rocksdb/utilities/table_properties_collectors/compact_on_deletion_collector.cc");
	config.file("rocksdb/utilities/transactions/optimistic_transaction.cc");
	config.file("rocksdb/utilities/transactions/optimistic_transaction_db_impl.cc");
	config.file("rocksdb/utilities/transactions/pessimistic_transaction.cc");
	config.file("rocksdb/utilities/transactions/pessimistic_transaction_db.cc");
	config.file("rocksdb/utilities/transactions/transaction_base.cc");
	config.file("rocksdb/utilities/transactions/transaction_db_mutex_impl.cc");
	config.file("rocksdb/utilities/transactions/transaction_lock_mgr.cc");
	config.file("rocksdb/utilities/transactions/transaction_util.cc");
	config.file("rocksdb/utilities/transactions/write_prepared_txn.cc");
	config.file("rocksdb/utilities/ttl/db_ttl_impl.cc");
	config.file("rocksdb/utilities/write_batch_with_index/write_batch_with_index.cc");
	config.file("rocksdb/utilities/write_batch_with_index/write_batch_with_index_internal.cc");

	config.file("build_version.cc");

	config.cpp(true);
	config.compile("librocksdb.a");

	println!("cargo:rustc-link-lib=static=snappy");
}
