use rocksdb::{ColumnFamilyOptions, DBOptions, Writable, DB};

use super::tempdir_with_prefix;

// This is testing that the reference counting prevents use after frees,
// and can be verified by running under valgrind.
#[test]
fn test_lifetimes() {
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let cf_opts = ColumnFamilyOptions::new();
    let path = tempdir_with_prefix("table_properties_rc");
    let path = path.path().to_str().unwrap();
    let db = DB::open_cf(opts, path, vec![("default", cf_opts)]).unwrap();

    let samples = vec![
        (b"key1".to_vec(), b"value1".to_vec()),
        (b"key2".to_vec(), b"value2".to_vec()),
        (b"key3".to_vec(), b"value3".to_vec()),
        (b"key4".to_vec(), b"value4".to_vec()),
    ];

    // Put 4 keys.
    for &(ref k, ref v) in &samples {
        db.put(k, v).unwrap();
        assert_eq!(v.as_slice(), &*db.get(k).unwrap().unwrap());
    }
    db.flush(true).unwrap();

    let collection = db.get_properties_of_all_tables_rc().unwrap();

    assert_eq!(collection.len(), 1);

    let mut iter = collection.iter();

    drop(collection);

    let (_key, prop) = iter.next().unwrap();

    drop(iter);

    let _ucp = prop.user_collected_properties();

    drop(prop);
}
