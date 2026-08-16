use rocksdb::{
    perf::{get_memory_usage_stats, MemoryUsageBuilder},
    DB,
};

#[test]
fn test_memory_usage_builder() {
    let tempdir = tempfile::tempdir().unwrap();

    let db = DB::open_default(tempdir.path()).unwrap();
    let mut builder = MemoryUsageBuilder::new().unwrap();
    builder.add_db(&db);
    let memory_usage = builder.build().unwrap();
    assert!(memory_usage.approximate_mem_table_total() > 0);

    // alternative non-builder approach
    let memory_usage = get_memory_usage_stats(Some(&[&db]), None).unwrap();
    assert!(memory_usage.mem_table_total > 0);
}

#[test]
fn test_memory_usage_builder_outlive_db() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail/memory_usage_builder_outlive_db.rs");
}
