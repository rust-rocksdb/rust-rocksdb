mod util;

use rocksdb::{CompactOptions, Options, ReadOptions, DB};
use std::cmp::Ordering;
use std::iter::FromIterator;
use util::{U64Comparator, U64Timestamp};

/// This function is for ensuring test of backwards compatibility
pub fn rocks_old_compare(one: &[u8], two: &[u8]) -> Ordering {
    one.cmp(two)
}

type CompareFn = dyn Fn(&[u8], &[u8]) -> Ordering;

/// create database add some values, and iterate over these
pub fn write_to_db_with_comparator(compare_fn: Box<CompareFn>) -> Vec<String> {
    let mut result_vec = Vec::new();

    let tempdir = tempfile::Builder::new()
        .prefix("_path_for_rocksdb_storage")
        .tempdir()
        .expect("Failed to create temporary path for the _path_for_rocksdb_storage");
    let path = tempdir.path();
    {
        let mut db_opts = Options::default();

        db_opts.create_missing_column_families(true);
        db_opts.create_if_missing(true);
        db_opts.set_comparator("cname", compare_fn);
        let db = DB::open(&db_opts, path).unwrap();
        db.put(b"a-key", b"a-value").unwrap();
        db.put(b"b-key", b"b-value").unwrap();
        let mut iter = db.raw_iterator();
        iter.seek_to_first();
        while iter.valid() {
            let key = iter.key().unwrap();
            // maybe not best way to copy?
            let key_str = key.iter().map(|b| *b as char).collect::<Vec<_>>();
            result_vec.push(String::from_iter(key_str));
            iter.next();
        }
    }
    let _ = DB::destroy(&Options::default(), path);
    result_vec
}

#[test]
/// First verify that using a function as a comparator works as expected
/// This should verify backwards compatibility
/// Then run a test with a clojure where an x-variable is passed
/// Keep in mind that this variable must be moved to the clojure
/// Then run a test with a reverse sorting clojure and make sure the order is reverted
fn test_comparator() {
    let local_compare = move |one: &[u8], two: &[u8]| one.cmp(two);
    let x = 0;
    let local_compare_reverse = move |one: &[u8], two: &[u8]| {
        println!(
            "Use the x value from the closure scope to do something smart: {:?}",
            x
        );
        match one.cmp(two) {
            Ordering::Less => Ordering::Greater,
            Ordering::Equal => Ordering::Equal,
            Ordering::Greater => Ordering::Less,
        }
    };

    let old_res = write_to_db_with_comparator(Box::new(rocks_old_compare));
    println!("Keys in normal sort order, no closure: {:?}", old_res);
    assert_eq!(vec!["a-key", "b-key"], old_res);
    let res_closure = write_to_db_with_comparator(Box::new(local_compare));
    println!("Keys in normal sort order, closure: {:?}", res_closure);
    assert_eq!(res_closure, old_res);
    let res_closure_reverse = write_to_db_with_comparator(Box::new(local_compare_reverse));
    println!(
        "Keys in reverse sort order, closure: {:?}",
        res_closure_reverse
    );
    assert_eq!(vec!["b-key", "a-key"], res_closure_reverse);
}

#[test]
fn test_comparator_with_ts() {
    let tempdir = tempfile::Builder::new()
        .prefix("_path_for_rocksdb_storage_with_ts")
        .tempdir()
        .expect("Failed to create temporary path for the _path_for_rocksdb_storage_with_ts.");
    let path = tempdir.path();
    let _ = DB::destroy(&Options::default(), path);

    {
        let mut db_opts = Options::default();
        db_opts.create_missing_column_families(true);
        db_opts.create_if_missing(true);
        db_opts.set_comparator_with_ts(
            U64Comparator::NAME,
            U64Timestamp::SIZE,
            Box::new(U64Comparator::compare),
            Box::new(U64Comparator::compare_ts),
            Box::new(U64Comparator::compare_without_ts),
        );
        let db = DB::open(&db_opts, path).unwrap();

        let key = b"hello";
        let val1 = b"world0";
        let val2 = b"world1";

        let ts = U64Timestamp::new(1);
        let ts2 = U64Timestamp::new(2);
        let ts3 = U64Timestamp::new(3);

        let mut opts = ReadOptions::default();
        opts.set_timestamp(ts);

        // basic put and get
        db.put_with_ts(key, ts, val1).unwrap();
        let value = db.get_opt(key, &opts).unwrap();
        assert_eq!(value.unwrap().as_slice(), val1);

        // update
        db.put_with_ts(key, ts2, val2).unwrap();
        opts.set_timestamp(ts2);
        let value = db.get_opt(key, &opts).unwrap();
        assert_eq!(value.unwrap().as_slice(), val2);

        // delete
        db.delete_with_ts(key, ts3).unwrap();
        opts.set_timestamp(ts3);
        let value = db.get_opt(key, &opts).unwrap();
        assert!(value.is_none());

        // ts2 should read deleted data
        opts.set_timestamp(ts2);
        let value = db.get_opt(key, &opts).unwrap();
        assert_eq!(value.unwrap().as_slice(), val2);

        // ts1 should read old data
        opts.set_timestamp(ts);
        let value = db.get_opt(key, &opts).unwrap();
        assert_eq!(value.unwrap().as_slice(), val1);

        // test iterator with ts
        opts.set_timestamp(ts2);
        let mut iter = db.raw_iterator_opt(opts);
        iter.seek_to_first();
        let mut result_vec = Vec::new();
        while iter.valid() {
            let key = iter.key().unwrap();
            // maybe not best way to copy?
            let key_str = key.iter().map(|b| *b as char).collect::<Vec<_>>();
            result_vec.push(String::from_iter(key_str));
            iter.next();
        }
        assert_eq!(result_vec, ["hello"]);

        // test full_history_ts_low works
        let mut compact_opts = CompactOptions::default();
        compact_opts.set_full_history_ts_low(ts2);
        db.compact_range_opt(None::<&[u8]>, None::<&[u8]>, &compact_opts);
        db.flush().unwrap();

        let mut opts = ReadOptions::default();
        opts.set_timestamp(ts3);
        let value = db.get_opt(key, &opts).unwrap();
        assert_eq!(value, None);
        // cannot read with timestamp older than full_history_ts_low
        opts.set_timestamp(ts);
        assert!(db.get_opt(key, &opts).is_err());
    }

    let _ = DB::destroy(&Options::default(), path);
}

#[test]
fn test_comparator_with_column_family_with_ts() {
    let tempdir = tempfile::Builder::new()
        .prefix("_path_for_rocksdb_storage_with_column_family_with_ts")
        .tempdir()
        .expect("Failed to create temporary path for the _path_for_rocksdb_storage_with_column_family_with_ts.");
    let path = tempdir.path();
    let _ = DB::destroy(&Options::default(), path);

    {
        let mut db_opts = Options::default();
        db_opts.create_missing_column_families(true);
        db_opts.create_if_missing(true);

        let mut cf_opts = Options::default();
        cf_opts.set_comparator_with_ts(
            U64Comparator::NAME,
            U64Timestamp::SIZE,
            Box::new(U64Comparator::compare),
            Box::new(U64Comparator::compare_ts),
            Box::new(U64Comparator::compare_without_ts),
        );

        let cfs = vec![("cf", cf_opts)];

        let db = DB::open_cf_with_opts(&db_opts, path, cfs).unwrap();
        let cf = db.cf_handle("cf").unwrap();

        let key = b"hello";
        let val1 = b"world0";
        let val2 = b"world1";

        let ts = U64Timestamp::new(1);
        let ts2 = U64Timestamp::new(2);
        let ts3 = U64Timestamp::new(3);

        let mut opts = ReadOptions::default();
        opts.set_timestamp(ts);

        // basic put and get
        db.put_cf_with_ts(&cf, key, ts, val1).unwrap();
        let value = db.get_cf_opt(&cf, key, &opts).unwrap();
        assert_eq!(value.unwrap().as_slice(), val1);

        // update
        db.put_cf_with_ts(&cf, key, ts2, val2).unwrap();
        opts.set_timestamp(ts2);
        let value = db.get_cf_opt(&cf, key, &opts).unwrap();
        assert_eq!(value.unwrap().as_slice(), val2);

        // delete
        db.delete_cf_with_ts(&cf, key, ts3).unwrap();
        opts.set_timestamp(ts3);
        let value = db.get_cf_opt(&cf, key, &opts).unwrap();
        assert!(value.is_none());

        // ts2 should read deleted data
        opts.set_timestamp(ts2);
        let value = db.get_cf_opt(&cf, key, &opts).unwrap();
        assert_eq!(value.unwrap().as_slice(), val2);

        // ts1 should read old data
        opts.set_timestamp(ts);
        let value = db.get_cf_opt(&cf, key, &opts).unwrap();
        assert_eq!(value.unwrap().as_slice(), val1);

        // test iterator with ts
        opts.set_timestamp(ts2);
        let mut iter = db.raw_iterator_cf_opt(&cf, opts);
        iter.seek_to_first();
        let mut result_vec = Vec::new();
        while iter.valid() {
            let key = iter.key().unwrap();
            // maybe not best way to copy?
            let key_str = key.iter().map(|b| *b as char).collect::<Vec<_>>();
            result_vec.push(String::from_iter(key_str));
            iter.next();
        }
        assert_eq!(result_vec, ["hello"]);

        // test full_history_ts_low works
        let mut compact_opts = CompactOptions::default();
        compact_opts.set_full_history_ts_low(ts2);
        db.compact_range_cf_opt(&cf, None::<&[u8]>, None::<&[u8]>, &compact_opts);
        db.flush().unwrap();

        // Attempt to read `full_history_ts_low`.
        // It should match the value we set earlier (`ts2`).
        let full_history_ts_low = db.get_full_history_ts_low(&cf).unwrap();
        assert_eq!(U64Timestamp::from(full_history_ts_low.as_slice()), ts2);

        let mut opts = ReadOptions::default();
        opts.set_timestamp(ts3);
        let value = db.get_cf_opt(&cf, key, &opts).unwrap();
        assert_eq!(value, None);
        // cannot read with timestamp older than full_history_ts_low
        opts.set_timestamp(ts);
        assert!(db.get_cf_opt(&cf, key, &opts).is_err());
    }

    let _ = DB::destroy(&Options::default(), path);
}
