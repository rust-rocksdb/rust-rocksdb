use rocksdb::{DB, Direction, IteratorMode, Writable};
use tempdir::TempDir;

fn cba(input: &Box<[u8]>) -> Box<[u8]> {
    input.iter().cloned().collect::<Vec<_>>().into_boxed_slice()
}

#[test]
pub fn test_iterator() {
    let path = TempDir::new("_rust_rocksdb_iteratortest").expect("");

    let k1: Box<[u8]> = b"k1".to_vec().into_boxed_slice();
    let k2: Box<[u8]> = b"k2".to_vec().into_boxed_slice();
    let k3: Box<[u8]> = b"k3".to_vec().into_boxed_slice();
    let k4: Box<[u8]> = b"k4".to_vec().into_boxed_slice();
    let v1: Box<[u8]> = b"v1111".to_vec().into_boxed_slice();
    let v2: Box<[u8]> = b"v2222".to_vec().into_boxed_slice();
    let v3: Box<[u8]> = b"v3333".to_vec().into_boxed_slice();
    let v4: Box<[u8]> = b"v4444".to_vec().into_boxed_slice();
    let db = DB::open_default(path.path().to_str().unwrap()).unwrap();
    let p = db.put(&*k1, &*v1);
    assert!(p.is_ok());
    let p = db.put(&*k2, &*v2);
    assert!(p.is_ok());
    let p = db.put(&*k3, &*v3);
    assert!(p.is_ok());
    let expected = vec![(cba(&k1), cba(&v1)),
                        (cba(&k2), cba(&v2)),
                        (cba(&k3), cba(&v3))];

    let iterator1 = db.iterator(IteratorMode::Start);
    assert_eq!(iterator1.collect::<Vec<_>>(), expected);

    // Test that it's idempotent
    let iterator1 = db.iterator(IteratorMode::Start);
    assert_eq!(iterator1.collect::<Vec<_>>(), expected);

    let iterator1 = db.iterator(IteratorMode::Start);
    assert_eq!(iterator1.collect::<Vec<_>>(), expected);

    let iterator1 = db.iterator(IteratorMode::Start);
    assert_eq!(iterator1.collect::<Vec<_>>(), expected);

    // Test it in reverse a few times
    let iterator1 = db.iterator(IteratorMode::End);
    let mut tmp_vec = iterator1.collect::<Vec<_>>();
    tmp_vec.reverse();
    assert_eq!(tmp_vec, expected);

    let iterator1 = db.iterator(IteratorMode::End);
    let mut tmp_vec = iterator1.collect::<Vec<_>>();
    tmp_vec.reverse();
    assert_eq!(tmp_vec, expected);

    let iterator1 = db.iterator(IteratorMode::End);
    let mut tmp_vec = iterator1.collect::<Vec<_>>();
    tmp_vec.reverse();
    assert_eq!(tmp_vec, expected);

    let iterator1 = db.iterator(IteratorMode::End);
    let mut tmp_vec = iterator1.collect::<Vec<_>>();
    tmp_vec.reverse();
    assert_eq!(tmp_vec, expected);

    let iterator1 = db.iterator(IteratorMode::End);
    let mut tmp_vec = iterator1.collect::<Vec<_>>();
    tmp_vec.reverse();
    assert_eq!(tmp_vec, expected);

    // Try it forward again
    let iterator1 = db.iterator(IteratorMode::Start);
    assert_eq!(iterator1.collect::<Vec<_>>(), expected);

    let iterator1 = db.iterator(IteratorMode::Start);
    assert_eq!(iterator1.collect::<Vec<_>>(), expected);

    let old_iterator = db.iterator(IteratorMode::Start);
    let p = db.put(&*k4, &*v4);
    assert!(p.is_ok());
    let expected2 = vec![(cba(&k1), cba(&v1)),
                         (cba(&k2), cba(&v2)),
                         (cba(&k3), cba(&v3)),
                         (cba(&k4), cba(&v4))];
    assert_eq!(old_iterator.collect::<Vec<_>>(), expected);

    let iterator1 = db.iterator(IteratorMode::Start);
    assert_eq!(iterator1.collect::<Vec<_>>(), expected2);

    let iterator1 = db.iterator(IteratorMode::From(b"k2",
                                                   Direction::Forward));
    let expected = vec![(cba(&k2), cba(&v2)),
                        (cba(&k3), cba(&v3)),
                        (cba(&k4), cba(&v4))];
    assert_eq!(iterator1.collect::<Vec<_>>(), expected);

    let iterator1 = db.iterator(IteratorMode::From(b"k2",
                                                   Direction::Reverse));
    let expected = vec![(cba(&k2), cba(&v2)), (cba(&k1), cba(&v1))];
    assert_eq!(iterator1.collect::<Vec<_>>(), expected);
}
