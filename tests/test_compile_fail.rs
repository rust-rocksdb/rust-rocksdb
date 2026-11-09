//! Checks that the borrow checker rejects API misuse, such as a snapshot,
//! iterator, checkpoint or transaction outliving its database.

#[test]
fn compile_fail() {
    // trybuild checks the cases in a separate project with its own target
    // directory (`target/tests/trybuild`), so librocksdb-sys' build script would
    // compile RocksDB from scratch a second time. A project that only has
    // compile-fail cases is type-checked with `cargo check` and never linked,
    // so pointing the build script at a "prebuilt" library directory skips the
    // C++ build entirely. The directory does not need to contain the libraries.
    //
    // This test is the only one in its binary, so changing the environment of
    // the process cannot race with other tests.
    let lib_dir = env!("CARGO_TARGET_TMPDIR");
    std::env::set_var("ROCKSDB_LIB_DIR", lib_dir);
    std::env::set_var("SNAPPY_LIB_DIR", lib_dir);

    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail/*.rs");
}
