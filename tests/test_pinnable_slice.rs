extern crate rocksdb;
mod util;

use rocksdb::{Options, DB};
use util::DBPath;

#[test]
fn test_pinnable_slice() {
    let path = DBPath::new("_rust_rocksdb_pinnable_slice_test");

    let mut opts = Options::default();
    opts.create_if_missing(true);
    let db = DB::open(&opts, &path).unwrap();

    db.put(b"k1", b"value12345").unwrap();

    let result = db.get_pinned(b"k1");
    assert!(result.is_ok());

    let value = result.unwrap();
    assert!(value.is_some());

    let pinnable_slice = value.unwrap();

    assert_eq!(b"12345", &pinnable_slice[5..10]);
}
