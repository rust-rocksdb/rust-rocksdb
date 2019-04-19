extern crate rocksdb;

use rocksdb::{
    prelude::*, Options, TemporaryDBPath, TransactionDB, TransactionOptions, WriteOptions,
};

#[test]
pub fn test_transaction() {
    let n = TemporaryDBPath::new();
    {
        let db = TransactionDB::open_default(&n).unwrap();

        let trans = db.transaction_default();

        trans.put(b"k1", b"v1").unwrap();
        trans.put(b"k2", b"v2").unwrap();
        trans.put(b"k3", b"v3").unwrap();
        trans.put(b"k4", b"v4").unwrap();

        trans.commit().unwrap();

        let trans2 = db.transaction_default();

        let mut iter = trans2.raw_iterator();

        iter.seek_to_first();

        assert_eq!(iter.valid(), true);
        assert_eq!(iter.key(), Some(b"k1".to_vec()));
        assert_eq!(iter.value(), Some(b"v1".to_vec()));

        iter.next();

        assert_eq!(iter.valid(), true);
        assert_eq!(iter.key(), Some(b"k2".to_vec()));
        assert_eq!(iter.value(), Some(b"v2".to_vec()));

        iter.next(); // k3
        iter.next(); // k4
        iter.next(); // invalid!

        assert_eq!(iter.valid(), false);
        assert_eq!(iter.key(), None);
        assert_eq!(iter.value(), None);

        let trans3 = db.transaction_default();

        trans2.put(b"k2", b"v5").unwrap();
        trans3.put(b"k2", b"v6").unwrap_err();

        trans3.commit().unwrap();

        trans2.commit().unwrap();
    }
}

#[test]
pub fn test_transaction_rollback_savepoint() {
    let path = TemporaryDBPath::new();
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);

        let db = TransactionDB::open(&opts, &path).unwrap();
        let write_options = WriteOptions::default();
        let transaction_options = TransactionOptions::new();

        let trans1 = db.transaction(&write_options, &transaction_options);
        let trans2 = db.transaction(&write_options, &transaction_options);

        trans1.put(b"k1", b"v1").unwrap();

        let k1_2 = trans2.get(b"k1").unwrap();
        assert!(k1_2.is_none());

        trans1.commit().unwrap();

        let trans3 = db.transaction(&write_options, &transaction_options);

        let k1_2 = trans2.get(b"k1").unwrap().unwrap();
        assert_eq!(&*k1_2, b"v1");

        trans3.delete(b"k1").unwrap();

        let k1_2 = trans2.get(b"k1").unwrap().unwrap();
        assert_eq!(&*k1_2, b"v1");

        trans3.rollback().unwrap();

        let k1_2 = trans2.get(b"k1").unwrap().unwrap();
        assert_eq!(&*k1_2, b"v1");

        let trans4 = db.transaction(&write_options, &transaction_options);

        trans4.delete(b"k1").unwrap();
        trans4.set_savepoint();
        trans4.put(b"k2", b"v2").unwrap();
        trans4.rollback_to_savepoint().unwrap();
        trans4.commit().unwrap();

        let k1_2 = trans2.get(b"k1").unwrap();
        assert!(k1_2.is_none());

        let k2_2 = trans2.get(b"k2").unwrap();
        assert!(k2_2.is_none());

        trans2.commit().unwrap();
    }
}

#[test]
pub fn test_transaction_snapshot() {
    let path = TemporaryDBPath::new();
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = TransactionDB::open(&opts, &path).unwrap();

        let write_options = WriteOptions::default();
        let transaction_options = TransactionOptions::new();
        let trans1 = db.transaction(&write_options, &transaction_options);

        let mut transaction_options_snapshot = TransactionOptions::new();
        transaction_options_snapshot.set_snapshot(true);
        // create transaction with snapshot
        let trans2 = db.transaction(&write_options, &transaction_options_snapshot);

        trans1.put(b"k1", b"v1").unwrap();

        let k1_2 = trans2.get(b"k1").unwrap();
        assert!(k1_2.is_none());

        trans1.commit().unwrap();
        drop(trans1);

        trans2.commit().unwrap();
        drop(trans2);

        let trans3 = db.transaction(&write_options, &transaction_options_snapshot);

        let trans4 = db.transaction(&write_options, &transaction_options);
        trans4.delete(b"k1").unwrap();
        trans4.commit().unwrap();
        drop(trans4);

        assert!(trans3.get(b"k1").unwrap().is_none());

        let k1_3 = trans3.snapshot().get(b"k1").unwrap().unwrap();
        assert_eq!(&*k1_3, b"v1");

        trans3.commit().unwrap();
        drop(trans3);

        let trans5 = db.transaction(&write_options, &transaction_options_snapshot);

        let k1_5 = trans5.snapshot().get(b"k1").unwrap();
        assert!(k1_5.is_none());

        trans5.commit().unwrap();
    }
}
