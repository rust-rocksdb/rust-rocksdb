use tempdir::TempDir;
use rocksdb::{DB, Options, Range, Writable};


#[test]
fn test_compact_range() {
	let path = TempDir::new("_rust_rocksdb_test_compact_range").expect("");
	let mut opts = Options::new();
	opts.create_if_missing(true);
	let db = DB::open(&opts, path.path().to_str().unwrap()).unwrap();
	let samples = vec![
        (b"k1".to_vec(), b"value--------1".to_vec()),
        (b"k2".to_vec(), b"value--------2".to_vec()),
        (b"k3".to_vec(), b"value--------3".to_vec()),
        (b"k4".to_vec(), b"value--------4".to_vec()),
        (b"k5".to_vec(), b"value--------5".to_vec()),
    ];
	for &(ref k, ref v) in &samples {
		db.put(k, v).unwrap();
		assert_eq!(v.as_slice(), &*db.get(k).unwrap().unwrap());
	}

	// flush memtable to sst file
	db.flush(true).unwrap();
	let old_size = db.get_approximate_sizes(&[Range::new(b"k0", b"k6")])[0];

	// delete all and compact whole range
	for &(ref k, _) in &samples {
		db.delete(k).unwrap()
	}
	db.compact_range(b"", b"");
	let new_size = db.get_approximate_sizes(&[Range::new(b"k0", b"k6")])[0];
	assert!(old_size > new_size);
}
