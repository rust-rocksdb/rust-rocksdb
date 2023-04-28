use rocksdb::{Options, DB};
use std::cmp::Ordering;
use std::iter::FromIterator;

/// This function is for ensuring test of backwards compatibility
pub fn rocks_old_compare(one: &[u8], two: &[u8]) -> Ordering {
    one.cmp(two)
}

/// create database add some values, and iterate over these
pub fn write_to_db_with_comparator(
    compare_fn: Box<dyn Fn(&[u8], &[u8]) -> Ordering>,
) -> Vec<String> {
    let mut result_vec = Vec::new();

    let path = "_path_for_rocksdb_storage";
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
/// This should verify backwards compatablity
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
