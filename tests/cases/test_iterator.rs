// Copyright 2017 PingCAP, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// See the License for the specific language governing permissions and
// limitations under the License.

use std::ops::Deref;
use std::sync::*;
use std::thread;

use rocksdb::rocksdb::Snapshot;
use rocksdb::*;

use super::tempdir_with_prefix;

struct FixedPrefixTransform {
    pub prefix_len: usize,
}

impl SliceTransform for FixedPrefixTransform {
    fn transform<'a>(&mut self, key: &'a [u8]) -> &'a [u8] {
        &key[..self.prefix_len]
    }

    fn in_domain(&mut self, key: &[u8]) -> bool {
        key.len() >= self.prefix_len
    }
}

struct FixedSuffixTransform {
    pub suffix_len: usize,
}

impl SliceTransform for FixedSuffixTransform {
    fn transform<'a>(&mut self, key: &'a [u8]) -> &'a [u8] {
        &key[..(key.len() - self.suffix_len)]
    }

    fn in_domain(&mut self, key: &[u8]) -> bool {
        key.len() >= self.suffix_len
    }
}

fn prev_collect<D: Deref<Target = DB>>(iter: &mut DBIterator<D>) -> Vec<Kv> {
    let mut buf = vec![];
    while iter.valid() {
        buf.push(iter.kv().unwrap());
        iter.prev();
    }
    buf
}

fn next_collect<D: Deref<Target = DB>>(iter: &mut DBIterator<D>) -> Vec<Kv> {
    let mut buf = vec![];
    while iter.valid() {
        buf.push(iter.kv().unwrap());
        iter.next();
    }
    buf
}

#[test]
pub fn test_iterator() {
    let path = tempdir_with_prefix("_rust_rocksdb_iteratortest");

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
    let expected = vec![
        (k1.to_vec(), v1.to_vec()),
        (k2.to_vec(), v2.to_vec()),
        (k3.to_vec(), v3.to_vec()),
    ];

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
    let expected2 = vec![
        (k1.to_vec(), v1.to_vec()),
        (k2.to_vec(), v2.to_vec()),
        (k3.to_vec(), v3.to_vec()),
        (k4.to_vec(), v4.to_vec()),
    ];
    assert_eq!(old_iterator.collect::<Vec<_>>(), expected);

    iter = db.iter();
    iter.seek(SeekKey::Start);
    assert_eq!(iter.collect::<Vec<_>>(), expected2);

    iter.seek(SeekKey::Key(k2));
    let expected = vec![
        (k2.to_vec(), v2.to_vec()),
        (k3.to_vec(), v3.to_vec()),
        (k4.to_vec(), v4.to_vec()),
    ];
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
    //iter.prev();
    //assert!(!iter.valid());
}

#[test]
fn test_send_iterator() {
    let path = tempdir_with_prefix("_rust_rocksdb_iteratortest_send");

    let db = Arc::new(DB::open_default(path.path().to_str().unwrap()).unwrap());
    db.put(b"k1", b"v1").unwrap();

    let opt = ReadOptions::new();
    let iter = DBIterator::new(db.clone(), opt);

    let make_checker = |mut iter: DBIterator<Arc<DB>>| {
        let (tx, rx) = mpsc::channel();
        let j = thread::spawn(move || {
            rx.recv().unwrap();
            iter.seek(SeekKey::Start);
            assert_eq!(iter.key(), b"k1");
            assert_eq!(iter.value(), b"v1");
        });
        (tx, j)
    };

    let (tx, handle) = make_checker(iter);
    drop(db);
    tx.send(()).unwrap();
    handle.join().unwrap();

    let db = Arc::new(DB::open_default(path.path().to_str().unwrap()).unwrap());
    db.flush(true).unwrap();

    let snap = Snapshot::new(db.clone());
    let iter = snap.iter_opt_clone(ReadOptions::new());
    db.put(b"k1", b"v2").unwrap();
    db.flush(true).unwrap();
    db.compact_range(None, None);

    let (tx, handle) = make_checker(iter);
    // iterator still holds the sst file, so it should be able to read the old value.
    drop(snap);
    drop(db);
    tx.send(()).unwrap();
    handle.join().unwrap();
}

#[test]
fn test_seek_for_prev() {
    let path = tempdir_with_prefix("_rust_rocksdb_seek_for_prev");
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    {
        let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
        let writeopts = WriteOptions::new();
        db.put_opt(b"k1-0", b"a", &writeopts).unwrap();
        db.put_opt(b"k1-1", b"b", &writeopts).unwrap();
        db.put_opt(b"k1-3", b"d", &writeopts).unwrap();

        let mut iter = db.iter();
        iter.seek_for_prev(SeekKey::Key(b"k1-2"));
        assert!(iter.valid());
        assert_eq!(iter.key(), b"k1-1");
        assert_eq!(iter.value(), b"b");

        let mut iter = db.iter();
        iter.seek_for_prev(SeekKey::Key(b"k1-3"));
        assert!(iter.valid());
        assert_eq!(iter.key(), b"k1-3");
        assert_eq!(iter.value(), b"d");

        let mut iter = db.iter();
        iter.seek_for_prev(SeekKey::Start);
        assert!(iter.valid());
        assert_eq!(iter.key(), b"k1-0");
        assert_eq!(iter.value(), b"a");

        let mut iter = db.iter();
        iter.seek_for_prev(SeekKey::End);
        assert!(iter.valid());
        assert_eq!(iter.key(), b"k1-3");
        assert_eq!(iter.value(), b"d");

        let mut iter = db.iter();
        iter.seek_for_prev(SeekKey::Key(b"k0-0"));
        assert!(!iter.valid());

        let mut iter = db.iter();
        iter.seek_for_prev(SeekKey::Key(b"k2-0"));
        assert!(iter.valid());
        assert_eq!(iter.key(), b"k1-3");
        assert_eq!(iter.value(), b"d");
    }
}

#[test]
fn read_with_upper_bound() {
    let path = tempdir_with_prefix("_rust_rocksdb_read_with_upper_bound_test");
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    {
        let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
        let writeopts = WriteOptions::new();
        db.put_opt(b"k1-0", b"a", &writeopts).unwrap();
        db.put_opt(b"k1-1", b"b", &writeopts).unwrap();
        db.put_opt(b"k2-0", b"c", &writeopts).unwrap();

        let mut readopts = ReadOptions::new();
        let upper_bound = b"k2".to_vec();
        readopts.set_iterate_upper_bound(upper_bound);
        assert_eq!(readopts.iterate_upper_bound(), b"k2");
        let mut iter = db.iter_opt(readopts);
        iter.seek(SeekKey::Start);
        let vec = next_collect(&mut iter);
        assert_eq!(vec.len(), 2);
    }
}

#[test]
fn test_total_order_seek() {
    let path = tempdir_with_prefix("_rust_rocksdb_total_order_seek");
    let mut bbto = BlockBasedOptions::new();
    bbto.set_bloom_filter(10, false);
    bbto.set_whole_key_filtering(false);
    let mut cf_opts = ColumnFamilyOptions::new();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    cf_opts.set_block_based_table_factory(&bbto);
    cf_opts
        .set_prefix_extractor(
            "FixedPrefixTransform",
            Box::new(FixedPrefixTransform { prefix_len: 2 }),
        )
        .unwrap();
    // also create prefix bloom for memtable
    cf_opts.set_memtable_prefix_bloom_size_ratio(0.1 as f64);

    let keys = vec![
        b"k1-1", b"k1-2", b"k1-3", b"k2-1", b"k2-2", b"k2-3", b"k3-1", b"k3-2", b"k3-3",
    ];
    let db = DB::open_cf(
        opts,
        path.path().to_str().unwrap(),
        vec![("default", cf_opts)],
    )
    .unwrap();
    let wopts = WriteOptions::new();

    // sst1
    db.put_opt(b"k1-1", b"a", &wopts).unwrap();
    db.put_opt(b"k1-2", b"b", &wopts).unwrap();
    db.put_opt(b"k1-3", b"c", &wopts).unwrap();
    db.put_opt(b"k2-1", b"a", &wopts).unwrap();
    db.flush(true /* sync */).unwrap(); // flush memtable to sst file.

    // sst2
    db.put_opt(b"k2-2", b"b", &wopts).unwrap();
    db.put_opt(b"k2-3", b"c", &wopts).unwrap();
    db.flush(true /* sync */).unwrap(); // flush memtable to sst file.

    // memtable
    db.put_opt(b"k3-1", b"a", &wopts).unwrap();
    db.put_opt(b"k3-2", b"b", &wopts).unwrap();
    db.put_opt(b"k3-3", b"c", &wopts).unwrap();

    let mut ropts = ReadOptions::new();
    ropts.set_prefix_same_as_start(true);
    let mut iter = db.iter_opt(ropts);
    // only iterate sst files and memtables that contain keys with the same prefix as b"k1"
    // and the keys is iterated as valid when prefixed as b"k1"
    iter.seek(SeekKey::Key(b"k1-0"));
    let mut key_count = 0;
    while iter.valid() {
        assert_eq!(keys[key_count], iter.key());
        key_count = key_count + 1;
        iter.next();
    }
    assert!(key_count == 3);

    let mut iter = db.iter();
    // only iterate sst files and memtables that contain keys with the same prefix as b"k1"
    // but it still can next/prev to the keys which is not prefixed as b"k1" with
    // prefix_same_as_start
    iter.seek(SeekKey::Key(b"k1-0"));
    let mut key_count = 0;
    while iter.valid() {
        assert_eq!(keys[key_count], iter.key());
        key_count = key_count + 1;
        iter.next();
    }
    assert!(key_count == 4);

    let mut ropts = ReadOptions::new();
    ropts.set_total_order_seek(true);
    let mut iter = db.iter_opt(ropts);
    iter.seek(SeekKey::Key(b"k1-0"));
    let mut key_count = 0;
    while iter.valid() {
        // iterator all sst files and memtables
        assert_eq!(keys[key_count], iter.key());
        key_count = key_count + 1;
        iter.next();
    }
    assert!(key_count == 9);
}

#[test]
fn test_fixed_suffix_seek() {
    let path = tempdir_with_prefix("_rust_rocksdb_fixed_suffix_seek");
    let mut bbto = BlockBasedOptions::new();
    bbto.set_bloom_filter(10, false);
    bbto.set_whole_key_filtering(false);
    let mut opts = DBOptions::new();
    let mut cf_opts = ColumnFamilyOptions::new();
    opts.create_if_missing(true);
    cf_opts.set_block_based_table_factory(&bbto);
    cf_opts
        .set_prefix_extractor(
            "FixedSuffixTransform",
            Box::new(FixedSuffixTransform { suffix_len: 2 }),
        )
        .unwrap();

    let db = DB::open_cf(
        opts,
        path.path().to_str().unwrap(),
        vec![("default", cf_opts)],
    )
    .unwrap();
    db.put(b"k-eghe-5", b"a").unwrap();
    db.put(b"k-24yfae-6", b"a").unwrap();
    db.put(b"k-h1fwd-7", b"a").unwrap();
    db.flush(true).unwrap();

    let mut iter = db.iter();
    iter.seek(SeekKey::Key(b"k-24yfae-8"));
    let vec = prev_collect(&mut iter);
    assert!(vec.len() == 2);

    let mut iter = db.iter();
    iter.seek(SeekKey::Key(b"k-24yfa-9"));
    let vec = prev_collect(&mut iter);
    assert!(vec.len() == 0);
}
