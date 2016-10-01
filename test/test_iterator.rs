use rocksdb::*;
use tempdir::TempDir;

fn prev_collect<'a>(iter: &mut DBIterator<'a>) -> Vec<Kv> {
    let mut buf = vec![];
    while iter.valid() {
        buf.push(iter.kv().unwrap());
        iter.prev();
    }
    buf
}

#[test]
pub fn test_iterator() {
    let path = TempDir::new("_rust_rocksdb_iteratortest").expect("");

    let k1 = b"k1";
    let k2 = b"k2";
    let k3 = b"k3";
    let k4 = b"k4";
    let v1 = b"v1111";
    let v2 = b"v2222";
    let v3 = b"v3333";
    let v4 = b"v4444";
    let db = DB::open_default(path.path().to_str().unwrap()).unwrap();
    let p = db.put(k1, v1);
    assert!(p.is_ok());
    let p = db.put(k2, v2);
    assert!(p.is_ok());
    let p = db.put(k3, v3);
    assert!(p.is_ok());
    let expected =
        vec![(k1.to_vec(), v1.to_vec()), (k2.to_vec(), v2.to_vec()), (k3.to_vec(), v3.to_vec())];

    let mut iter = db.iter();

    iter.seek(SeekKey::Start);
    assert_eq!(iter.collect::<Vec<_>>(), expected);

    // Test that it's idempotent
    iter.seek(SeekKey::Start);
    assert_eq!(iter.collect::<Vec<_>>(), expected);

    // Test it in reverse a few times
    iter.seek(SeekKey::End);
    let mut tmp_vec = prev_collect(&mut iter);
    tmp_vec.reverse();
    assert_eq!(tmp_vec, expected);

    iter.seek(SeekKey::End);
    let mut tmp_vec = prev_collect(&mut iter);
    tmp_vec.reverse();
    assert_eq!(tmp_vec, expected);

    // Try it forward again
    iter.seek(SeekKey::Start);
    assert_eq!(iter.collect::<Vec<_>>(), expected);

    iter.seek(SeekKey::Start);
    assert_eq!(iter.collect::<Vec<_>>(), expected);

    let mut old_iterator = db.iter();
    old_iterator.seek(SeekKey::Start);
    let p = db.put(&*k4, &*v4);
    assert!(p.is_ok());
    let expected2 = vec![(k1.to_vec(), v1.to_vec()),
                         (k2.to_vec(), v2.to_vec()),
                         (k3.to_vec(), v3.to_vec()),
                         (k4.to_vec(), v4.to_vec())];
    assert_eq!(old_iterator.collect::<Vec<_>>(), expected);

    iter = db.iter();
    iter.seek(SeekKey::Start);
    assert_eq!(iter.collect::<Vec<_>>(), expected2);

    iter.seek(SeekKey::Key(k2));
    let expected =
        vec![(k2.to_vec(), v2.to_vec()), (k3.to_vec(), v3.to_vec()), (k4.to_vec(), v4.to_vec())];
    assert_eq!(iter.collect::<Vec<_>>(), expected);

    iter.seek(SeekKey::Key(k2));
    let expected = vec![(k2.to_vec(), v2.to_vec()), (k1.to_vec(), v1.to_vec())];
    assert_eq!(prev_collect(&mut iter), expected);

    iter.seek(SeekKey::Key(b"k0"));
    assert!(iter.valid());
    iter.seek(SeekKey::Key(b"k1"));
    assert!(iter.valid());
    iter.seek(SeekKey::Key(b"k11"));
    assert!(iter.valid());
    iter.seek(SeekKey::Key(b"k5"));
    assert!(!iter.valid());
    iter.seek(SeekKey::Key(b"k0"));
    assert!(iter.valid());
    iter.seek(SeekKey::Key(b"k1"));
    assert!(iter.valid());
    iter.seek(SeekKey::Key(b"k11"));
    assert!(iter.valid());
    iter.seek(SeekKey::Key(b"k5"));
    assert!(!iter.valid());

    iter.seek(SeekKey::Key(b"k4"));
    assert!(iter.valid());
    iter.prev();
    assert!(iter.valid());
    iter.next();
    assert!(iter.valid());
    iter.next();
    assert!(!iter.valid());
    // Once iterator is invalid, it can't be reverted.
    iter.prev();
    assert!(!iter.valid());
}

#[test]
fn read_with_upper_bound() {
    let path = TempDir::new("_rust_rocksdb_read_with_upper_bound_test").expect("");
    let mut opts = Options::new();
    opts.create_if_missing(true);
    {
        let db = DB::open(&opts, path.path().to_str().unwrap()).unwrap();
        let writeopts = WriteOptions::new();
        db.put_opt(b"k1-0", b"a", &writeopts).unwrap();
        db.put_opt(b"k1-1", b"b", &writeopts).unwrap();
        db.put_opt(b"k2-0", b"c", &writeopts).unwrap();

        let mut readopts = ReadOptions::new();
        readopts.set_iterate_upper_bound(b"k2");
        let mut iter = db.iter_opt(readopts);
        iter.seek(SeekKey::Start);
        let mut count = 0;
        while iter.valid() {
            count += 1;
            if !iter.next() {
                break;
            }
        }
        assert_eq!(count, 2);
    }
}
