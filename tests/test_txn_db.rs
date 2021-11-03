mod util;

use pretty_assertions::assert_eq;

use rocksdb::{
    CuckooTableOptions, Error, IteratorMode, Options, ReadOptions, SliceTransform, TxnDB,
    TxnDBOptions, WriteBatch,
};
use util::DBPath;

#[test]
fn open_default() {
    let path = DBPath::new("_rust_rocksdb_txndb_open_default");

    {
        let db = TxnDB::open_default(&path).unwrap();

        assert!(db.put(b"k1", b"v1111").is_ok());

        let r: Result<Option<Vec<u8>>, Error> = db.get(b"k1");

        assert_eq!(r.unwrap().unwrap(), b"v1111");
        assert!(db.delete(b"k1").is_ok());
        assert!(db.get(b"k1").unwrap().is_some());
        assert!(db.get(b"k1").unwrap().is_none());
    }
}

#[test]
fn put_get() {
    let path = DBPath::new("_rust_rocksdb_txndb_put_get");
    {
        let db = TxnDB::open_default(&path).unwrap();
        assert!(db.put(b"k1", b"v1111").is_ok());
        assert!(db.put(b"k2", b"v22222222").is_ok());

        let v1 = db.get(b"k1").unwrap().unwrap();
        let v2 = db.get(b"k2").unwrap().unwrap();
        assert_eq!(v1.as_slice(), b"v1111");
        assert_eq!(v2.as_slice(), b"v22222222");
    }
}

#[test]
fn destroy_on_open() {
    let path = DBPath::new("_rust_rocksdb_txndb_destroy_on_open");
    let _db = TxnDB::open_default(&path).unwrap();
    let opts = Options::default();
    // The TxnDB will still be open when we try to destroy it and the lock should fail.
    match TxnDB::destroy(&opts, &path) {
        Err(s) => {
            let message = s.to_string();
            assert!(message.contains("IO error:"));
            assert!(message.contains("_rust_rocksdb_txndb_destroy_on_open"));
            assert!(message.contains("/LOCK:"));
        }
        Ok(_) => panic!("should fail"),
    }
}

#[test]
fn writebatch() {
    let path = DBPath::new("_rust_rocksdb_txndb_writebatch");
    {
        let db = TxnDB::open_default(&path).unwrap();
        {
            // test put
            let mut batch = WriteBatch::default();
            assert!(db.get(b"k1").unwrap().is_none());
            assert_eq!(batch.len(), 0);
            assert!(batch.is_empty());
            batch.put(b"k1", b"v1111");
            batch.put(b"k2", b"v2222");
            batch.put(b"k3", b"v3333");
            assert_eq!(batch.len(), 3);
            assert!(!batch.is_empty());
            assert!(db.get(b"k1").unwrap().is_none());
            let p = db.write(batch);
            assert!(p.is_ok());
            let r: Result<Option<Vec<u8>>, Error> = db.get(b"k1");
            assert_eq!(r.unwrap().unwrap(), b"v1111");
        }
        {
            // test delete
            let mut batch = WriteBatch::default();
            batch.delete(b"k1");
            assert_eq!(batch.len(), 1);
            assert!(!batch.is_empty());
            let p = db.write(batch);
            assert!(p.is_ok());
            assert!(db.get(b"k1").unwrap().is_none());
        }
        {
            // test size_in_bytes
            let mut batch = WriteBatch::default();
            let before = batch.size_in_bytes();
            batch.put(b"k1", b"v1234567890");
            let after = batch.size_in_bytes();
            assert!(before + 10 <= after);
        }
    }
}

#[test]
fn iterator_test() {
    let path = DBPath::new("_rust_rocksdb_txndb_iteratortest");
    {
        let data = [(b"k1", b"v1111"), (b"k2", b"v2222"), (b"k3", b"v3333")];
        let db = TxnDB::open_default(&path).unwrap();

        for (key, value) in &data {
            assert!(db.put(key, value).is_ok());
        }

        let iter = db.iterator(IteratorMode::Start);

        for (idx, (db_key, db_value)) in iter.enumerate() {
            let (key, value) = data[idx];
            assert_eq!((&key[..], &value[..]), (db_key.as_ref(), db_value.as_ref()));
        }
    }
}

#[test]
fn snapshot_test() {
    let path = DBPath::new("_rust_rocksdb_txndb_snapshottest");
    {
        let db = TxnDB::open_default(&path).unwrap();

        assert!(db.put(b"k1", b"v1111").is_ok());

        let snap = db.snapshot();
        assert_eq!(snap.get(b"k1").unwrap().unwrap(), b"v1111");

        assert!(db.put(b"k2", b"v2222").is_ok());

        assert!(db.get(b"k2").unwrap().is_some());
        assert!(snap.get(b"k2").unwrap().is_none());
    }
}

#[test]
fn prefix_extract_and_iterate_test() {
    let path = DBPath::new("_rust_rocksdb_txndb_prefix_extract_and_iterate");
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        opts.set_prefix_extractor(SliceTransform::create_fixed_prefix(2));
        let txn_db_opts = TxnDBOptions::default();

        let db = TxnDB::open(&opts, &txn_db_opts, &path).unwrap();
        db.put(b"p1_k1", b"v1").unwrap();
        db.put(b"p2_k2", b"v2").unwrap();
        db.put(b"p1_k3", b"v3").unwrap();
        db.put(b"p1_k4", b"v4").unwrap();
        db.put(b"p2_k5", b"v5").unwrap();

        let mut readopts = ReadOptions::default();
        readopts.set_prefix_same_as_start(true);
        readopts.set_iterate_lower_bound(b"p1".to_vec());
        readopts.set_pin_data(true);

        let iter = db.iterator_opt(IteratorMode::Start, readopts);
        let expected: Vec<_> = vec![(b"p1_k1", b"v1"), (b"p1_k3", b"v3"), (b"p1_k4", b"v4")]
            .into_iter()
            .map(|(k, v)| (k.to_vec().into_boxed_slice(), v.to_vec().into_boxed_slice()))
            .collect();
        assert_eq!(expected, iter.collect::<Vec<_>>());
    }
}

#[test]
fn cuckoo() {
    let path = DBPath::new("_rust_rocksdb_txndb_cuckoo");

    {
        let mut opts = Options::default();
        let txn_db_opts = TxnDBOptions::default();
        let mut factory_opts = CuckooTableOptions::default();
        factory_opts.set_hash_ratio(0.8);
        factory_opts.set_max_search_depth(20);
        factory_opts.set_cuckoo_block_size(10);
        factory_opts.set_identity_as_first_hash(true);
        factory_opts.set_use_module_hash(false);

        opts.set_cuckoo_table_factory(&factory_opts);
        opts.create_if_missing(true);

        let db = TxnDB::open(&opts, &txn_db_opts, &path).unwrap();
        db.put(b"k1", b"v1").unwrap();
        db.put(b"k2", b"v2").unwrap();
        let r: Result<Option<Vec<u8>>, Error> = db.get(b"k1");

        assert_eq!(r.unwrap().unwrap(), b"v1");
        let r: Result<Option<Vec<u8>>, Error> = db.get(b"k2");

        assert_eq!(r.unwrap().unwrap(), b"v2");
        assert!(db.delete(b"k1").is_ok());
        assert!(db.get(b"k1").unwrap().is_none());
    }
}
