use rocksdb::{ColumnFamilyOptions, DBOptions, Writable, DB};

use super::tempdir_with_prefix;

#[test]
pub fn test_ttl() {
    let path = tempdir_with_prefix("_rust_rocksdb_ttl_test");
    let path_str = path.path().to_str().unwrap();

    // should be able to open db with ttl
    {
        let mut opts = DBOptions::new();
        let cf_opts = ColumnFamilyOptions::new();
        let ttl = 10;
        opts.create_if_missing(true);

        let mut db = match DB::open_cf_with_ttl(
            opts,
            path.path().to_str().unwrap(),
            vec![("default", cf_opts)],
            &[ttl],
        ) {
            Ok(db) => {
                println!("successfully opened db with ttl");
                db
            }
            Err(e) => panic!("failed to open db with ttl: {}", e),
        };

        match db.create_cf("cf1") {
            Ok(_) => println!("cf1 created successfully"),
            Err(e) => {
                panic!("could not create column family: {}", e);
            }
        }
        assert_eq!(db.cf_names(), vec!["default", "cf1"]);

        match db.create_cf("cf2") {
            Ok(_) => println!("cf2 created successfully"),
            Err(e) => {
                panic!("could not create column family: {}", e);
            }
        }
        assert_eq!(db.cf_names(), vec!["default", "cf1", "cf2"]);
        drop(db);
    }

    // should be able to write, read over a cf with the length of ttls equals to that of cfs
    {
        let db = match DB::open_cf_with_ttl(
            DBOptions::new(),
            path_str,
            vec![
                ("cf1", ColumnFamilyOptions::new()),
                ("cf2", ColumnFamilyOptions::new()),
                ("default", ColumnFamilyOptions::new()),
            ],
            &[10, 10, 10],
        ) {
            Ok(db) => {
                println!("successfully opened cf with ttl");
                db
            }
            Err(e) => panic!("failed to open cf with ttl: {}", e),
        };
        let cf1 = db.cf_handle("cf1").unwrap();
        assert!(db.put_cf(cf1, b"k1", b"v1").is_ok());
        assert!(db.get_cf(cf1, b"k1").unwrap().unwrap().to_utf8().unwrap() == "v1");
        let p = db.put_cf(cf1, b"k1", b"a");
        assert!(p.is_ok());
    }

    // should be able to write, read over a cf with the length of ttls equals to that of cfs.
    // default cf could be with ttl 0 if it is not in cfds
    {
        let db = match DB::open_cf_with_ttl(
            DBOptions::new(),
            path_str,
            vec![
                ("cf1", ColumnFamilyOptions::new()),
                ("cf2", ColumnFamilyOptions::new()),
            ],
            &[10, 10],
        ) {
            Ok(db) => {
                println!("successfully opened cf with ttl");
                db
            }
            Err(e) => panic!("failed to open cf with ttl: {}", e),
        };
        let cf1 = db.cf_handle("cf1").unwrap();
        assert!(db.put_cf(cf1, b"k1", b"v1").is_ok());
        assert!(db.get_cf(cf1, b"k1").unwrap().unwrap().to_utf8().unwrap() == "v1");
        let p = db.put_cf(cf1, b"k1", b"a");
        assert!(p.is_ok());
    }

    // should fail to open cf with ttl when the length of ttls not equal to that of cfs
    {
        let _db = match DB::open_cf_with_ttl(
            DBOptions::new(),
            path_str,
            vec![
                ("cf1", ColumnFamilyOptions::new()),
                ("cf2", ColumnFamilyOptions::new()),
            ],
            &[10],
        ) {
            Ok(_) => panic!(
                "should not have opened DB successfully with ttl \
                 when the length of ttl not equal to that of cfs"
            ),
            Err(e) => assert!(e.starts_with("the length of ttls not equal to length of cfs")),
        };
    }

    // should fail to open cf with ttl when the length of ttls not equal to that of cfs
    // when default is in cfds, it's ttl must be supplied
    {
        let _db = match DB::open_cf_with_ttl(
            DBOptions::new(),
            path_str,
            vec![
                ("cf1", ColumnFamilyOptions::new()),
                ("cf2", ColumnFamilyOptions::new()),
                ("default", ColumnFamilyOptions::new()),
            ],
            &[10, 10],
        ) {
            Ok(_) => panic!(
                "should not have opened DB successfully with ttl \
                 when the length of ttl not equal to that of cfs"
            ),
            Err(e) => assert!(e.starts_with("the length of ttls not equal to length of cfs")),
        };
    }
}
