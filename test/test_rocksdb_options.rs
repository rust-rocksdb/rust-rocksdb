
use rocksdb::{DB, Options};use tempdir::TempDir;


#[test]
fn test_set_num_levels() {
    let path = TempDir::new("_rust_rocksdb_test_set_num_levels").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);
    opts.set_num_levels(2);
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    drop(db);
}

#[test]
fn test_log_file_opt() {
    let path = TempDir::new("_rust_rocksdb_log_file_opt").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);
    opts.set_max_log_file_size(100 * 1024 * 1024);
    opts.set_keep_log_file_num(10);
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
    drop(db);
}

