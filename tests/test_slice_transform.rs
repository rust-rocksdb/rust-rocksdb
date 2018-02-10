extern crate rocksdb;

use rocksdb::{DB, Options, SliceTransform};

#[test]
pub fn test_slice_transform() {

    let path = "_rust_rocksdb_slicetransform_test";
    let a1: Box<[u8]> = key(b"aaa1");
    let a2: Box<[u8]> = key(b"aaa2");
    let b1: Box<[u8]> = key(b"bbb1");
    let b2: Box<[u8]> = key(b"bbb2");

    fn first_three(k: &[u8]) -> Vec<u8> {
        k.iter().take(3).cloned().collect()
    }

    let prefix_extractor = SliceTransform::create("first_three", first_three, None);

    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.set_prefix_extractor(prefix_extractor);

    let db = DB::open(&opts, path).unwrap();

    assert!(db.put(&*a1, &*a1).is_ok());
    assert!(db.put(&*a2, &*a2).is_ok());
    assert!(db.put(&*b1, &*b1).is_ok());
    assert!(db.put(&*b2, &*b2).is_ok());

    fn cba(input: &Box<[u8]>) -> Box<[u8]> {
        input.iter().cloned().collect::<Vec<_>>().into_boxed_slice()
    }

    fn key(k: &[u8]) -> Box<[u8]> { k.to_vec().into_boxed_slice() }

    {
        let expected = vec![(cba(&a1), cba(&a1)), (cba(&a2), cba(&a2))];
        let a_iterator = db.prefix_iterator(b"aaa");
        assert_eq!(a_iterator.collect::<Vec<_>>(), expected)
    }

    {
        let expected = vec![(cba(&b1), cba(&b1)), (cba(&b2), cba(&b2))];
        let b_iterator = db.prefix_iterator(b"bbb");
        assert_eq!(b_iterator.collect::<Vec<_>>(), expected)
    }
}
