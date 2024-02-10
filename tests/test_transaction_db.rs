// Copyright 2021 Yiyuan Liu
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

mod util;

use pretty_assertions::assert_eq;

use rocksdb::{
    CuckooTableOptions, DBAccess, Direction, Error, ErrorKind, IteratorMode, Options, ReadOptions,
    SliceTransform, TransactionDB, TransactionDBOptions, TransactionOptions,
    WriteBatchWithTransaction, WriteOptions, DB,
};
use util::DBPath;

#[test]
fn open_default() {
    let path = DBPath::new("_rust_rocksdb_transaction_db_open_default");

    {
        let db: TransactionDB = TransactionDB::open_default(&path).unwrap();

        assert!(db.put(b"k1", b"v1111").is_ok());

        let r: Result<Option<Vec<u8>>, Error> = db.get(b"k1");

        assert_eq!(r.unwrap().unwrap(), b"v1111");
        assert!(db.delete(b"k1").is_ok());
        assert!(db.get(b"k1").unwrap().is_none());
    }
}

#[test]
fn open_cf() {
    let path = DBPath::new("_rust_rocksdb_transaction_db_open_cf");
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        let db: TransactionDB = TransactionDB::open_cf(
            &opts,
            &TransactionDBOptions::default(),
            &path,
            ["cf1", "cf2"],
        )
        .unwrap();

        let cf1 = db.cf_handle("cf1").unwrap();
        let cf2 = db.cf_handle("cf2").unwrap();

        db.put(b"k0", b"v0").unwrap();
        db.put_cf(&cf1, b"k1", b"v1").unwrap();
        db.put_cf(&cf2, b"k2", b"v2").unwrap();

        assert_eq!(db.get(b"k0").unwrap().unwrap(), b"v0");
        assert!(db.get(b"k1").unwrap().is_none());
        assert!(db.get(b"k2").unwrap().is_none());

        assert!(db.get_cf(&cf1, b"k0").unwrap().is_none());
        assert_eq!(db.get_cf(&cf1, b"k1").unwrap().unwrap(), b"v1");
        assert!(db.get_cf(&cf1, b"k2").unwrap().is_none());

        assert!(db.get_cf(&cf2, b"k0").unwrap().is_none());
        assert!(db.get_cf(&cf2, b"k1").unwrap().is_none());
        assert_eq!(db.get_cf(&cf2, b"k2").unwrap().unwrap(), b"v2");
    }
}

#[test]
fn put_get() {
    let path = DBPath::new("_rust_rocksdb_transaction_db_put_get");
    {
        let db: TransactionDB = TransactionDB::open_default(&path).unwrap();
        assert!(db.put(b"k1", b"v1111").is_ok());
        assert!(db.put(b"k2", b"v22222222").is_ok());

        let v1 = db.get(b"k1").unwrap().unwrap();
        let v2 = db.get(b"k2").unwrap().unwrap();
        assert_eq!(v1.as_slice(), b"v1111");
        assert_eq!(v2.as_slice(), b"v22222222");
    }
}

#[test]
fn multi_get() {
    let path = DBPath::new("_rust_rocksdb_multi_get");

    {
        let db: TransactionDB = TransactionDB::open_default(&path).unwrap();
        let initial_snap = db.snapshot();
        db.put(b"k1", b"v1").unwrap();
        let k1_snap = db.snapshot();
        db.put(b"k2", b"v2").unwrap();

        let _ = db.multi_get([b"k0"; 40]);

        let assert_values = |values: Vec<_>| {
            assert_eq!(3, values.len());
            assert_eq!(values[0], None);
            assert_eq!(values[1], Some(b"v1".to_vec()));
            assert_eq!(values[2], Some(b"v2".to_vec()));
        };

        let values = db
            .multi_get([b"k0", b"k1", b"k2"])
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>();

        assert_values(values);

        let values = DBAccess::multi_get_opt(&db, [b"k0", b"k1", b"k2"], &Default::default())
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>();

        assert_values(values);

        let values = db
            .snapshot()
            .multi_get([b"k0", b"k1", b"k2"])
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>();

        assert_values(values);

        let none_values = initial_snap
            .multi_get([b"k0", b"k1", b"k2"])
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>();

        assert_eq!(none_values, vec![None; 3]);

        let k1_only = k1_snap
            .multi_get([b"k0", b"k1", b"k2"])
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>();

        assert_eq!(k1_only, vec![None, Some(b"v1".to_vec()), None]);

        let txn = db.transaction();
        let values = txn
            .multi_get([b"k0", b"k1", b"k2"])
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>();

        assert_values(values);
    }
}

#[test]
fn multi_get_cf() {
    let path = DBPath::new("_rust_rocksdb_multi_get_cf");

    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        let db: TransactionDB = TransactionDB::open_cf(
            &opts,
            &TransactionDBOptions::default(),
            &path,
            ["cf0", "cf1", "cf2"],
        )
        .unwrap();

        let cf0 = db.cf_handle("cf0").unwrap();

        let cf1 = db.cf_handle("cf1").unwrap();
        db.put_cf(&cf1, b"k1", b"v1").unwrap();

        let cf2 = db.cf_handle("cf2").unwrap();
        db.put_cf(&cf2, b"k2", b"v2").unwrap();

        let values = db
            .multi_get_cf(vec![(&cf0, b"k0"), (&cf1, b"k1"), (&cf2, b"k2")])
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        assert_eq!(3, values.len());
        assert_eq!(values[0], None);
        assert_eq!(values[1], Some(b"v1".to_vec()));
        assert_eq!(values[2], Some(b"v2".to_vec()));

        let txn = db.transaction();
        let values = txn
            .multi_get_cf(vec![(&cf0, b"k0"), (&cf1, b"k1"), (&cf2, b"k2")])
            .into_iter()
            .map(Result::unwrap)
            .collect::<Vec<_>>();

        assert_eq!(3, values.len());
        assert_eq!(values[0], None);
        assert_eq!(values[1], Some(b"v1".to_vec()));
        assert_eq!(values[2], Some(b"v2".to_vec()));
    }
}

#[test]
fn destroy_on_open() {
    let path = DBPath::new("_rust_rocksdb_transaction_db_destroy_on_open");
    let _db: TransactionDB = TransactionDB::open_default(&path).unwrap();
    let opts = Options::default();
    // The TransactionDB will still be open when we try to destroy it and the lock should fail.
    match DB::destroy(&opts, &path) {
        Err(s) => {
            let message = s.to_string();
            assert_eq!(s.kind(), ErrorKind::IOError);
            assert!(message.contains("_rust_rocksdb_transaction_db_destroy_on_open"));
            assert!(message.contains("/LOCK:"));
        }
        Ok(_) => panic!("should fail"),
    }
}

#[test]
fn writebatch() {
    let path = DBPath::new("_rust_rocksdb_transaction_db_writebatch");
    {
        let db: TransactionDB = TransactionDB::open_default(&path).unwrap();
        {
            // test put
            let mut batch = WriteBatchWithTransaction::<true>::default();
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
            let mut batch = WriteBatchWithTransaction::<true>::default();
            batch.delete(b"k1");
            assert_eq!(batch.len(), 1);
            assert!(!batch.is_empty());
            let p = db.write(batch);
            assert!(p.is_ok());
            assert!(db.get(b"k1").unwrap().is_none());
        }
        {
            // test size_in_bytes
            let mut batch = WriteBatchWithTransaction::<true>::default();
            let before = batch.size_in_bytes();
            batch.put(b"k1", b"v1234567890");
            let after = batch.size_in_bytes();
            assert!(before + 10 <= after);
        }
    }
}

#[test]
fn iterator_test() {
    let path = DBPath::new("_rust_rocksdb_transaction_db_iteratortest");
    {
        let db: TransactionDB = TransactionDB::open_default(&path).unwrap();

        let k1: Box<[u8]> = b"k1".to_vec().into_boxed_slice();
        let k2: Box<[u8]> = b"k2".to_vec().into_boxed_slice();
        let k3: Box<[u8]> = b"k3".to_vec().into_boxed_slice();
        let k4: Box<[u8]> = b"k4".to_vec().into_boxed_slice();
        let v1: Box<[u8]> = b"v1111".to_vec().into_boxed_slice();
        let v2: Box<[u8]> = b"v2222".to_vec().into_boxed_slice();
        let v3: Box<[u8]> = b"v3333".to_vec().into_boxed_slice();
        let v4: Box<[u8]> = b"v4444".to_vec().into_boxed_slice();

        db.put(&*k1, &*v1).unwrap();
        db.put(&*k2, &*v2).unwrap();
        db.put(&*k3, &*v3).unwrap();
        let expected = vec![
            (k1.clone(), v1.clone()),
            (k2.clone(), v2.clone()),
            (k3.clone(), v3.clone()),
        ];

        let iter = db.iterator(IteratorMode::Start);
        assert_eq!(iter.map(Result::unwrap).collect::<Vec<_>>(), expected);

        // Test that it's idempotent
        let iter = db.iterator(IteratorMode::Start);
        assert_eq!(iter.map(Result::unwrap).collect::<Vec<_>>(), expected);
        let iter = db.iterator(IteratorMode::Start);
        assert_eq!(iter.map(Result::unwrap).collect::<Vec<_>>(), expected);

        // Test in reverse
        let iter = db.iterator(IteratorMode::End);
        let mut tmp_vec = iter.map(Result::unwrap).collect::<Vec<_>>();
        tmp_vec.reverse();

        let old_iter = db.iterator(IteratorMode::Start);
        db.put(&*k4, &*v4).unwrap();
        let expected2 = vec![
            (k1, v1),
            (k2, v2),
            (k3.clone(), v3.clone()),
            (k4.clone(), v4.clone()),
        ];
        assert_eq!(old_iter.map(Result::unwrap).collect::<Vec<_>>(), expected);

        let iter = db.iterator(IteratorMode::Start);
        assert_eq!(iter.map(Result::unwrap).collect::<Vec<_>>(), expected2);

        let iter = db.iterator(IteratorMode::From(b"k3", Direction::Forward));
        assert_eq!(
            iter.map(Result::unwrap).collect::<Vec<_>>(),
            vec![(k3, v3), (k4, v4)]
        );
    }
}

#[test]
fn snapshot_test() {
    let path = DBPath::new("_rust_rocksdb_transaction_db_snapshottest");
    {
        let db: TransactionDB = TransactionDB::open_default(&path).unwrap();

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
    let path = DBPath::new("_rust_rocksdb_transaction_db_prefix_extract_and_iterate");
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        opts.set_prefix_extractor(SliceTransform::create_fixed_prefix(2));
        let txn_db_opts = TransactionDBOptions::default();

        let db: TransactionDB = TransactionDB::open(&opts, &txn_db_opts, &path).unwrap();
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
        assert_eq!(expected, iter.map(Result::unwrap).collect::<Vec<_>>());
    }
}

#[test]
fn cuckoo() {
    let path = DBPath::new("_rust_rocksdb_transaction_db_cuckoo");

    {
        let mut opts = Options::default();
        let txn_db_opts = TransactionDBOptions::default();
        let mut factory_opts = CuckooTableOptions::default();
        factory_opts.set_hash_ratio(0.8);
        factory_opts.set_max_search_depth(20);
        factory_opts.set_cuckoo_block_size(10);
        factory_opts.set_identity_as_first_hash(true);
        factory_opts.set_use_module_hash(false);

        opts.set_cuckoo_table_factory(&factory_opts);
        opts.create_if_missing(true);

        let db: TransactionDB = TransactionDB::open(&opts, &txn_db_opts, &path).unwrap();
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

#[test]
fn transaction() {
    let path = DBPath::new("_rust_rocksdb_transaction_db_transaction");
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let mut txn_db_opts = TransactionDBOptions::default();
        txn_db_opts.set_txn_lock_timeout(10);

        let db: TransactionDB = TransactionDB::open(&opts, &txn_db_opts, &path).unwrap();

        // put outside of transaction
        db.put(b"k1", b"v1").unwrap();
        assert_eq!(db.get(b"k1").unwrap().unwrap(), b"v1");

        let txn1 = db.transaction();
        txn1.put(b"k1", b"v2").unwrap();

        // get outside of transaction
        assert_eq!(db.get(b"k1").unwrap().unwrap().as_slice(), b"v1");

        // modify same key in another transaction, should get TimedOut
        let txn2 = db.transaction();
        let err = txn2.put(b"k1", b"v3").unwrap_err();
        assert_eq!(err.kind(), ErrorKind::TimedOut);

        // modify same key directly, should also get TimedOut
        let err = db.put(b"k1", b"v4").unwrap_err();
        assert_eq!(err.kind(), ErrorKind::TimedOut);

        txn1.commit().unwrap();
        assert_eq!(db.get(b"k1").unwrap().unwrap().as_slice(), b"v2");
    }
}

#[test]
fn transaction_iterator() {
    let path = DBPath::new("_rust_rocksdb_transaction_db_transaction_iterator");
    {
        let db: TransactionDB = TransactionDB::open_default(&path).unwrap();

        let k1: Box<[u8]> = b"k1".to_vec().into_boxed_slice();
        let k2: Box<[u8]> = b"k2".to_vec().into_boxed_slice();
        let k3: Box<[u8]> = b"k3".to_vec().into_boxed_slice();
        let k4: Box<[u8]> = b"k4".to_vec().into_boxed_slice();
        let v1: Box<[u8]> = b"v1111".to_vec().into_boxed_slice();
        let v2: Box<[u8]> = b"v2222".to_vec().into_boxed_slice();
        let v3: Box<[u8]> = b"v3333".to_vec().into_boxed_slice();
        let v4: Box<[u8]> = b"v4444".to_vec().into_boxed_slice();

        db.put(&*k1, &*v1).unwrap();
        db.put(&*k2, &*v2).unwrap();
        db.put(&*k3, &*v3).unwrap();
        let expected = vec![
            (k1.clone(), v1.clone()),
            (k2.clone(), v2.clone()),
            (k3.clone(), v3.clone()),
        ];

        let txn = db.transaction();

        let iter = txn.iterator(IteratorMode::Start);
        assert_eq!(iter.map(Result::unwrap).collect::<Vec<_>>(), expected);

        // Test that it's idempotent
        let iter = txn.iterator(IteratorMode::Start);
        assert_eq!(iter.map(Result::unwrap).collect::<Vec<_>>(), expected);
        let iter = txn.iterator(IteratorMode::Start);
        assert_eq!(iter.map(Result::unwrap).collect::<Vec<_>>(), expected);

        // Test in reverse
        let iter = txn.iterator(IteratorMode::End);
        let mut tmp_vec = iter.map(Result::unwrap).collect::<Vec<_>>();
        tmp_vec.reverse();

        let old_iter = txn.iterator(IteratorMode::Start);
        txn.put(&*k4, &*v4).unwrap();
        let expected2 = vec![
            (k1, v1),
            (k2, v2),
            (k3.clone(), v3.clone()),
            (k4.clone(), v4.clone()),
        ];
        assert_eq!(old_iter.map(Result::unwrap).collect::<Vec<_>>(), expected);

        let iter = txn.iterator(IteratorMode::Start);
        assert_eq!(iter.map(Result::unwrap).collect::<Vec<_>>(), expected2);

        let iter = txn.iterator(IteratorMode::From(b"k3", Direction::Forward));
        assert_eq!(
            iter.map(Result::unwrap).collect::<Vec<_>>(),
            vec![(k3, v3), (k4, v4)]
        );
    }
}

#[test]
fn transaction_rollback() {
    let path = DBPath::new("_rust_rocksdb_transaction_db_transaction_rollback");
    {
        let db: TransactionDB = TransactionDB::open_default(&path).unwrap();
        let txn = db.transaction();

        txn.rollback().unwrap();

        txn.put(b"k1", b"v1").unwrap();
        txn.set_savepoint();
        txn.put(b"k2", b"v2").unwrap();

        assert_eq!(txn.get(b"k1").unwrap().unwrap(), b"v1");
        assert_eq!(txn.get(b"k2").unwrap().unwrap(), b"v2");

        txn.rollback_to_savepoint().unwrap();
        assert_eq!(txn.get(b"k1").unwrap().unwrap(), b"v1");
        assert!(txn.get(b"k2").unwrap().is_none());

        txn.rollback().unwrap();
        assert!(txn.get(b"k1").unwrap().is_none());

        txn.commit().unwrap();

        assert!(db.get(b"k2").unwrap().is_none());
    }
}

#[test]
fn transaction_cf() {
    let path = DBPath::new("_rust_rocksdb_transaction_db_transaction_cf");
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        let db: TransactionDB = TransactionDB::open_cf(
            &opts,
            &TransactionDBOptions::default(),
            &path,
            ["cf1", "cf2"],
        )
        .unwrap();

        let cf1 = db.cf_handle("cf1").unwrap();
        let cf2 = db.cf_handle("cf2").unwrap();

        let txn = db.transaction();
        txn.put(b"k0", b"v0").unwrap();
        txn.put_cf(&cf1, b"k1", b"v1").unwrap();
        txn.put_cf(&cf2, b"k2", b"v2").unwrap();

        assert_eq!(txn.get(b"k0").unwrap().unwrap(), b"v0");
        assert!(txn.get(b"k1").unwrap().is_none());
        assert!(txn.get(b"k2").unwrap().is_none());

        assert!(txn.get_cf(&cf1, b"k0").unwrap().is_none());
        assert_eq!(txn.get_cf(&cf1, b"k1").unwrap().unwrap(), b"v1");
        assert!(txn.get_cf(&cf1, b"k2").unwrap().is_none());

        assert!(txn.get_cf(&cf2, b"k0").unwrap().is_none());
        assert!(txn.get_cf(&cf2, b"k1").unwrap().is_none());
        assert_eq!(txn.get_cf(&cf2, b"k2").unwrap().unwrap(), b"v2");

        txn.commit().unwrap();
    }
}

#[test]
fn transaction_snapshot() {
    let path = DBPath::new("_rust_rocksdb_transaction_db_transaction_snapshot");
    {
        let db: TransactionDB = TransactionDB::open_default(&path).unwrap();

        let txn = db.transaction();
        let snapshot = txn.snapshot();
        assert!(snapshot.get(b"k1").unwrap().is_none());
        db.put(b"k1", b"v1").unwrap();
        assert_eq!(snapshot.get(b"k1").unwrap().unwrap(), b"v1");

        let mut opts = TransactionOptions::default();
        opts.set_snapshot(true);
        let txn = db.transaction_opt(&WriteOptions::default(), &opts);
        db.put(b"k2", b"v2").unwrap();
        let snapshot = txn.snapshot();
        assert!(snapshot.get(b"k2").unwrap().is_none());
        assert_eq!(txn.get(b"k2").unwrap().unwrap(), b"v2");
        assert_eq!(
            txn.get_for_update(b"k2", true).unwrap_err().kind(),
            ErrorKind::Busy
        );
    }
}

#[test]
fn two_phase_commit() {
    let path = DBPath::new("_rust_rocksdb_transaction_db_2pc");
    {
        let db: TransactionDB = TransactionDB::open_default(&path).unwrap();

        let txn = db.transaction();
        txn.put(b"k1", b"v1").unwrap();
        txn.set_name(b"txn1").unwrap();
        txn.prepare().unwrap();
        txn.commit().unwrap();

        let txn = db.transaction();
        txn.put(b"k2", b"v2").unwrap();
        let err = txn.prepare().unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidArgument);

        let mut opt = TransactionOptions::new();
        opt.set_skip_prepare(false);
        let txn = db.transaction_opt(&WriteOptions::default(), &opt);
        txn.put(b"k3", b"v3").unwrap();
        let err = txn.prepare().unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidArgument);
    }

    DB::destroy(&Options::default(), &path).unwrap();

    {
        let db: TransactionDB = TransactionDB::open_default(&path).unwrap();

        let txn = db.transaction();
        txn.put(b"k1", b"v1").unwrap();
        txn.set_name(b"t1").unwrap();
        txn.prepare().unwrap();

        let txn2 = db.transaction();
        txn2.put(b"k2", b"v1").unwrap();
        txn2.set_name(b"t2").unwrap();
        txn2.prepare().unwrap();

        let txn3 = db.transaction();
        let err = txn3.set_name(b"t1").unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidArgument);

        // k1 and k2 should locked after we restore prepared transactions.
        let err = db.put(b"k1", b"v2").unwrap_err();
        assert_eq!(err.kind(), ErrorKind::TimedOut);
    }

    {
        // recovery
        let mut opt = TransactionDBOptions::new();
        opt.set_default_lock_timeout(1);
        let db: TransactionDB = TransactionDB::open_default(&path).unwrap();

        // get prepared transactions
        let txns = db.prepared_transactions();
        assert_eq!(txns.len(), 2);

        for txn in txns.into_iter() {
            let name = txn.get_name().unwrap();

            if name == b"t1" {
                txn.commit().unwrap();
            } else if name == b"t2" {
                txn.rollback().unwrap();
            } else {
                unreachable!();
            }
        }

        assert_eq!(db.get(b"k1").unwrap().unwrap(), b"v1");
        assert!(db.get(b"k2").unwrap().is_none());
    }
}

#[test]
fn test_snapshot_outlive_transaction_db() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail/snapshot_outlive_transaction_db.rs");
}

#[test]
fn test_txn_outlive_transaction_db() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail/transaction_outlive_transaction_db.rs");
}

#[test]
fn test_snapshot_outlive_txn() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail/snapshot_outlive_transaction.rs");
}
