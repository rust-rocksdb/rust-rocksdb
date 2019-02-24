extern crate rocksdb;

use rocksdb::{Options, TemporaryDBPath, DB};

#[test]
fn test_pinnable_slice() {
    let path = TemporaryDBPath::new();

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
