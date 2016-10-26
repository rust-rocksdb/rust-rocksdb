use rocksdb::{DB, Options};


#[test]
fn test_set_num_levels() {
	let path = "_rust_rocksdb_test_set_num_levels";
	let mut opts = Options::default();
	opts.create_if_missing(true);
	opts.set_num_levels(2);
	let db = DB::open(&opts, path).unwrap();
	drop(db);
}
