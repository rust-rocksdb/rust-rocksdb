use rocksdb::{DBOptions, Writable, DB};

use super::tempdir_with_prefix;

macro_rules! check_kv {
    ($db:expr, $key:expr, $val:expr) => {
        assert_eq!($db.get($key).unwrap().unwrap(), $val);
    };
    ($db:expr, $cf:expr, $key:expr, $val:expr) => {
        assert_eq!($db.get_cf($cf, $key).unwrap().unwrap(), $val);
    };
}

#[test]
fn test_open_for_read_only() {
    let temp = tempdir_with_prefix("_rust_rocksdb_test_open_for_read_only");
    let path = temp.path().to_str().unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);

    let rw = DB::open_default(path).unwrap();
    rw.put(b"k1", b"v1").unwrap();
    rw.put(b"k2", b"v2").unwrap();
    rw.put(b"k3", b"v3").unwrap();
    check_kv!(rw, b"k1", b"v1");
    check_kv!(rw, b"k2", b"v2");
    check_kv!(rw, b"k3", b"v3");

    let r1 = DB::open_for_read_only(opts.clone(), path, false).unwrap();
    check_kv!(r1, b"k1", b"v1");
    check_kv!(r1, b"k2", b"v2");
    check_kv!(r1, b"k3", b"v3");

    let r2 = DB::open_for_read_only(opts.clone(), path, false).unwrap();
    check_kv!(r2, b"k1", b"v1");
    check_kv!(r2, b"k2", b"v2");
    check_kv!(r2, b"k3", b"v3");

    let r3 = DB::open_for_read_only(opts.clone(), path, false).unwrap();
    check_kv!(r3, b"k1", b"v1");
    check_kv!(r3, b"k2", b"v2");
    check_kv!(r3, b"k3", b"v3");

    drop(rw);
    drop(r1);
    drop(r2);
    drop(r3);
}

#[test]
fn test_open_cf_for_read_only() {
    let temp = tempdir_with_prefix("_rust_rocksdb_test_open_cf_for_read_only");
    let path = temp.path().to_str().unwrap();

    {
        let mut rw = DB::open_default(path).unwrap();
        let _ = rw.create_cf("cf1").unwrap();
        let _ = rw.create_cf("cf2").unwrap();
    }

    {
        let rw = DB::open_cf(DBOptions::new(), path, vec!["cf1", "cf2"]).unwrap();
        let cf1 = rw.cf_handle("cf1").unwrap();
        rw.put_cf(cf1, b"cf1_k1", b"cf1_v1").unwrap();
        rw.put_cf(cf1, b"cf1_k2", b"cf1_v2").unwrap();
        rw.put_cf(cf1, b"cf1_k3", b"cf1_v3").unwrap();
        check_kv!(rw, cf1, b"cf1_k1", b"cf1_v1");
        check_kv!(rw, cf1, b"cf1_k2", b"cf1_v2");
        check_kv!(rw, cf1, b"cf1_k3", b"cf1_v3");
        let cf2 = rw.cf_handle("cf2").unwrap();
        rw.put_cf(cf2, b"cf2_k1", b"cf2_v1").unwrap();
        rw.put_cf(cf2, b"cf2_k2", b"cf2_v2").unwrap();
        rw.put_cf(cf2, b"cf2_k3", b"cf2_v3").unwrap();
        check_kv!(rw, cf2, b"cf2_k1", b"cf2_v1");
        check_kv!(rw, cf2, b"cf2_k2", b"cf2_v2");
        check_kv!(rw, cf2, b"cf2_k3", b"cf2_v3");
    }

    {
        let r1 = DB::open_cf_for_read_only(DBOptions::new(), path, vec!["cf1"], false).unwrap();
        let cf1 = r1.cf_handle("cf1").unwrap();
        check_kv!(r1, cf1, b"cf1_k1", b"cf1_v1");
        check_kv!(r1, cf1, b"cf1_k2", b"cf1_v2");
        check_kv!(r1, cf1, b"cf1_k3", b"cf1_v3");

        let r2 = DB::open_cf_for_read_only(DBOptions::new(), path, vec!["cf2"], false).unwrap();
        let cf2 = r2.cf_handle("cf2").unwrap();
        check_kv!(r2, cf2, b"cf2_k1", b"cf2_v1");
        check_kv!(r2, cf2, b"cf2_k2", b"cf2_v2");
        check_kv!(r2, cf2, b"cf2_k3", b"cf2_v3");
    }
}
