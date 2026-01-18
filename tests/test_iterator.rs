// Copyright 2020 Tyler Neely
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

mod util;

use pretty_assertions::assert_eq;

use rocksdb::{Direction, IteratorMode, MemtableFactory, Options, ReadOptions, DB};
use util::{assert_iter, assert_iter_reversed, pair, DBPath, U64Comparator, U64Timestamp};

#[test]
#[allow(clippy::cognitive_complexity)]
fn test_iterator() {
    let n = DBPath::new("_rust_rocksdb_iterator_test");
    {
        const K1: &[u8] = b"k1";
        const K2: &[u8] = b"k2";
        const K3: &[u8] = b"k3";
        const K4: &[u8] = b"k4";
        const V1: &[u8] = b"v1111";
        const V2: &[u8] = b"v2222";
        const V3: &[u8] = b"v3333";
        const V4: &[u8] = b"v4444";

        let db = DB::open_default(&n).unwrap();
        assert!(db.put(K1, V1).is_ok());
        assert!(db.put(K2, V2).is_ok());
        assert!(db.put(K3, V3).is_ok());
        let expected = [pair(K1, V1), pair(K2, V2), pair(K3, V3)];
        assert_iter(db.iterator(IteratorMode::Start), &expected);
        // Test that it's idempotent
        assert_iter(db.iterator(IteratorMode::Start), &expected);
        assert_iter(db.iterator(IteratorMode::Start), &expected);
        assert_iter(db.iterator(IteratorMode::Start), &expected);
        // Test it in reverse a few times
        assert_iter_reversed(db.iterator(IteratorMode::End), &expected);
        assert_iter_reversed(db.iterator(IteratorMode::End), &expected);
        assert_iter_reversed(db.iterator(IteratorMode::End), &expected);
        assert_iter_reversed(db.iterator(IteratorMode::End), &expected);
        // Try it forward again
        assert_iter(db.iterator(IteratorMode::Start), &expected);
        assert_iter(db.iterator(IteratorMode::Start), &expected);

        {
            let old_iterator = db.iterator(IteratorMode::Start);
            assert!(db.put(K4, V4).is_ok());
            assert_iter(old_iterator, &expected);
        }
        let expected2 = [pair(K1, V1), pair(K2, V2), pair(K3, V3), pair(K4, V4)];
        assert_iter(db.iterator(IteratorMode::Start), &expected2);
        assert_iter(
            db.iterator(IteratorMode::From(b"k2", Direction::Forward)),
            &expected2[1..],
        );
        assert_iter_reversed(
            db.iterator(IteratorMode::From(b"k2", Direction::Reverse)),
            &expected[..2],
        );
        assert_iter_reversed(
            db.iterator(IteratorMode::From(b"zz", Direction::Reverse)),
            &expected2,
        );

        {
            let test = |valid, key, dir| {
                let mut it = db.iterator(IteratorMode::From(key, dir));
                let value = it.next();
                if valid {
                    let expect = format!("{value:?}");
                    assert!(matches!(value, Some(Ok(_))), "{:?}", &expect);
                } else {
                    assert_eq!(None, value);
                    assert_eq!(None, it.next()); // Iterator is fused
                }
            };

            test(true, b"k0", Direction::Forward);
            test(true, b"k1", Direction::Forward);
            test(true, b"k11", Direction::Forward);
            test(false, b"k5", Direction::Forward);
            test(false, b"k0", Direction::Reverse);
            test(true, b"k1", Direction::Reverse);
            test(true, b"k11", Direction::Reverse);
            test(true, b"k5", Direction::Reverse);
        }
        {
            let mut iterator1 = db.iterator(IteratorMode::From(b"k4", Direction::Forward));
            iterator1.next().unwrap().unwrap();
            assert_eq!(None, iterator1.next());
            assert_eq!(None, iterator1.next());
        }
        {
            // Check that set_mode resets the iterator
            let mode = IteratorMode::From(K3, Direction::Forward);
            let mut iterator = db.iterator(mode);
            assert_iter(&mut iterator, &expected2[2..]);
            iterator.set_mode(mode);
            assert_iter(&mut iterator, &expected2[2..]);
        }
    }
}

#[test]
fn test_prefix_iterator() {
    let n = DBPath::new("_rust_rocksdb_prefix_iterator_test");
    {
        const A1: &[u8] = b"aaa1";
        const A2: &[u8] = b"aaa2";
        const B1: &[u8] = b"bbb1";
        const B2: &[u8] = b"bbb2";

        let prefix_extractor = rocksdb::SliceTransform::create_fixed_prefix(3);

        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_prefix_extractor(prefix_extractor);

        let db = DB::open(&opts, &n).unwrap();

        assert!(db.put(A1, A1).is_ok());
        assert!(db.put(A2, A2).is_ok());
        assert!(db.put(B1, B1).is_ok());
        assert!(db.put(B2, B2).is_ok());

        assert_iter(db.prefix_iterator(b"aaa"), &[pair(A1, A1), pair(A2, A2)]);
        assert_iter(db.prefix_iterator(b"bbb"), &[pair(B1, B1), pair(B2, B2)]);
        assert_iter(db.prefix_iterator(A2), &[pair(A2, A2)]);
    }
}

#[test]
fn test_prefix_iterator_uses_full_prefix() {
    // Test scenario derived from GitHub issue #221

    // Explanation: `db.prefix_iterator` sets the underlying
    // options to seek to the first key that matches the *entire*
    // `prefix`. From there, the iterator will continue to read pairs
    // as long as the prefix extracted from `key` matches the
    // prefix extracted from `prefix`.

    let path = DBPath::new("_rust_rocksdb_prefix_iterator_uses_full_prefix_test");
    {
        let data = [
            ([0, 0, 0, 0], b"111"),
            ([0, 0, 0, 1], b"222"),
            ([0, 1, 0, 1], b"333"),
            ([0, 1, 1, 1], b"444"),
            ([0, 1, 2, 1], b"555"),
            ([0, 2, 0, 0], b"666"),
            ([2, 0, 0, 0], b"777"),
            ([2, 2, 2, 2], b"888"),
        ];

        let prefix_extractor = rocksdb::SliceTransform::create_fixed_prefix(1);

        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_prefix_extractor(prefix_extractor);

        let db = DB::open(&opts, &path).unwrap();

        for (key, value) in &data {
            assert!(db.put(key, *value).is_ok());
        }

        assert_iter(
            db.prefix_iterator([0, 1, 1]),
            &[
                pair(&[0, 1, 1, 1], b"444"),
                pair(&[0, 1, 2, 1], b"555"),
                pair(&[0, 2, 0, 0], b"666"),
            ],
        );
    }
}

#[test]
fn test_full_iterator() {
    let path = DBPath::new("full_iterator_test");
    {
        const A1: &[u8] = b"aaa1";
        const A2: &[u8] = b"aaa2";
        const B1: &[u8] = b"bbb1";
        const B2: &[u8] = b"bbb2";

        let prefix_extractor = rocksdb::SliceTransform::create_fixed_prefix(3);
        let factory = MemtableFactory::HashSkipList {
            bucket_count: 1_000_000,
            height: 4,
            branching_factor: 4,
        };

        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_prefix_extractor(prefix_extractor);
        opts.set_allow_concurrent_memtable_write(false);
        opts.set_memtable_factory(factory);

        let db = DB::open(&opts, &path).unwrap();

        assert!(db.put(A1, A1).is_ok());
        assert!(db.put(A2, A2).is_ok());
        assert!(db.put(B1, B1).is_ok());
        assert!(db.put(B2, B2).is_ok());

        // A normal iterator won't work here since we're using a HashSkipList for our memory table
        // implementation (which buckets keys based on their prefix):
        let bad_iterator = db.iterator(IteratorMode::Start);
        assert_eq!(bad_iterator.collect::<Vec<_>>(), vec![]);

        assert_iter(
            db.full_iterator(IteratorMode::Start),
            &[pair(A1, A1), pair(A2, A2), pair(B1, B1), pair(B2, B2)],
        );
    }
}

fn custom_iter(db: &'_ DB) -> impl Iterator<Item = usize> + '_ {
    db.iterator(IteratorMode::Start)
        .map(Result::unwrap)
        .map(|(_, db_value)| db_value.len())
}

#[test]
fn test_custom_iterator() {
    let path = DBPath::new("_rust_rocksdb_custom_iterator_test");
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, &path).unwrap();
        let _data = custom_iter(&db).collect::<Vec<usize>>();
    }
}

#[test]
fn test_iterator_outlive_db() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail/iterator_outlive_db.rs");
}

#[test]
fn test_iter_range() {
    #[rustfmt::skip]
    const ALL_KEYS: [&[u8]; 12] = [
        /*  0 */ b"a0",
        /*  1 */ b"a1",
        /*  2 */ b"a11",
        /*  3 */ b"a2",
        /*  4 */ b"a\xff0",
        /*  5 */ b"a\xff1",
        /*  6 */ b"b0",
        /*  7 */ b"b1",
        /*  8 */ b"\xff",
        /*  9 */ b"\xff0",
        /* 10 */ b"\xff1",
        /* 11 */ b"\xff2",
    ];

    let path = DBPath::new("_rust_rocksdb_iter_range_test");
    let db = DB::open_default(&path).unwrap();
    for key in ALL_KEYS.iter() {
        assert!(db.put(key, key).is_ok());
    }

    fn test(
        db: &DB,
        mode: IteratorMode,
        range: impl rocksdb::IterateBounds,
        want: std::ops::Range<usize>,
        reverse: bool,
    ) {
        let mut ro = rocksdb::ReadOptions::default();
        // Set bounds to test that set_iterate_range clears old bounds.
        ro.set_iterate_lower_bound(vec![b'z']);
        ro.set_iterate_upper_bound(vec![b'z']);
        ro.set_iterate_range(range);
        let got = db
            .iterator_opt(mode, ro)
            .map(Result::unwrap)
            .map(|(key, _value)| key)
            .collect::<Vec<_>>();
        let mut got = got.iter().map(Box::as_ref).collect::<Vec<_>>();
        if reverse {
            got.reverse();
        }
        assert_eq!(&ALL_KEYS[want], got);
    }

    fn prefix(key: &[u8]) -> rocksdb::PrefixRange<&[u8]> {
        rocksdb::PrefixRange(key)
    }

    // Test Start and End modes
    {
        fn check<R>(db: &DB, range: R, want: std::ops::Range<usize>)
        where
            R: rocksdb::IterateBounds + Clone,
        {
            test(db, IteratorMode::Start, range.clone(), want.clone(), false);
            test(db, IteratorMode::End, range, want, true);
        }

        check(&db, .., 0..12);
        check(&db, "b1".as_bytes().., 7..12);
        check(&db, .."b1".as_bytes(), 0..7);
        check(&db, "a1".as_bytes().."b1".as_bytes(), 1..7);

        check(&db, prefix(b""), 0..12);
        check(&db, prefix(b"a"), 0..6);
        check(&db, prefix(b"a1"), 1..3);
        check(&db, prefix(b"a\xff"), 4..6);
        check(&db, prefix(b"\xff"), 8..12);
    }

    // Test From mode with Forward direction
    {
        fn check<R>(db: &DB, from: &[u8], range: R, want: std::ops::Range<usize>)
        where
            R: rocksdb::IterateBounds + Clone,
        {
            let mode = IteratorMode::From(from, Direction::Forward);
            test(db, mode, range, want, false);
        }

        check(&db, b"b0", .., 6..12);
        check(&db, b"b0", "a2".as_bytes().., 6..12);
        check(&db, b"b0", .."a1".as_bytes(), 0..0);
        check(&db, b"b0", .."b0".as_bytes(), 0..0);
        check(&db, b"b0", .."b1".as_bytes(), 6..7);
        check(&db, b"b0", "a1".as_bytes().."b0".as_bytes(), 0..0);
        check(&db, b"b0", "a1".as_bytes().."b1".as_bytes(), 6..7);

        check(&db, b"b0", prefix(b""), 6..12);
        check(&db, b"a1", prefix(b"a"), 1..6);
        check(&db, b"b0", prefix(b"a"), 0..0);
        check(&db, b"a1", prefix(b"a1"), 1..3);
        check(&db, b"b0", prefix(b"a1"), 0..0);
        check(&db, b"a1", prefix(b"a\xff"), 4..6);
        check(&db, b"b0", prefix(b"a\xff"), 0..0);
        check(&db, b"b0", prefix(b"\xff"), 8..12);
    }

    // Test From mode with Reverse direction
    {
        fn check<R>(db: &DB, from: &[u8], range: R, want: std::ops::Range<usize>)
        where
            R: rocksdb::IterateBounds + Clone,
        {
            let mode = IteratorMode::From(from, Direction::Reverse);
            test(db, mode, range, want, true);
        }

        check(&db, b"b0", .., 0..7);
        check(&db, b"b0", "a2".as_bytes().., 3..7);
        check(&db, b"b0", .."a1".as_bytes(), 0..1);
        check(&db, b"b0", .."b0".as_bytes(), 0..6);
        check(&db, b"b0", .."b1".as_bytes(), 0..7);
        check(&db, b"b0", "a1".as_bytes().."b0".as_bytes(), 1..6);
        check(&db, b"b0", "a1".as_bytes().."b1".as_bytes(), 1..7);

        check(&db, b"b0", prefix(b""), 0..7);
        check(&db, b"a1", prefix(b"a"), 0..2);
        check(&db, b"b0", prefix(b"a"), 0..6);
        check(&db, b"a1", prefix(b"a1"), 1..2);
        check(&db, b"b0", prefix(b"a1"), 1..3);
        check(&db, b"a1", prefix(b"a\xff"), 0..0);
        check(&db, b"b0", prefix(b"a\xff"), 4..6);
        check(&db, b"b0", prefix(b"\xff"), 0..0);
    }
}

#[test]
fn test_iterator_user_defined_timestamp() {
    let path = DBPath::new("_rust_rocksdb_iterator_user_defined_timestamp_test");
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
        let db = DB::open(&db_opts, &path).unwrap();

        let key1 = b"k1";
        let key2 = b"k2";
        let val1 = b"value1";
        let val2 = b"value2";
        let val1_updated = b"value1_updated";

        let ts1 = U64Timestamp::new(1);
        let ts2 = U64Timestamp::new(2);
        let ts3 = U64Timestamp::new(3);

        // Insert data at different timestamps
        db.put_with_ts(key1, ts1, val1).unwrap();
        db.put_with_ts(key2, ts2, val2).unwrap();
        db.put_with_ts(key1, ts3, val1_updated).unwrap();

        // Test iterator timestamp() returns correct timestamps
        {
            let mut opts = ReadOptions::default();
            opts.set_timestamp(ts3);
            let mut iter = db.raw_iterator_opt(opts);

            // Before seeking, iterator is invalid, timestamp should be None
            assert!(!iter.valid());
            assert_eq!(iter.timestamp(), None);

            // Seek to first key
            iter.seek_to_first();
            assert!(iter.valid());
            assert_eq!(iter.key(), Some(key1.as_slice()));
            assert_eq!(iter.value(), Some(val1_updated.as_slice()));
            // The timestamp returned should match ts3 (the write timestamp for key1's latest value)
            let ts = iter
                .timestamp()
                .expect("timestamp should be present for valid iterator");
            assert_eq!(U64Timestamp::from(ts), ts3);

            // Move to next key
            iter.next();
            assert!(iter.valid());
            assert_eq!(iter.key(), Some(key2.as_slice()));
            assert_eq!(iter.value(), Some(val2.as_slice()));
            // The timestamp returned should match ts2 (the write timestamp for key2)
            let ts = iter
                .timestamp()
                .expect("timestamp should be present for valid iterator");
            assert_eq!(U64Timestamp::from(ts), ts2);

            // Move past end
            iter.next();
            assert!(!iter.valid());
            assert_eq!(iter.timestamp(), None);
        }

        // Test reading with older timestamp sees older data with correct timestamp
        {
            let mut opts = ReadOptions::default();
            opts.set_timestamp(ts1);
            let mut iter = db.raw_iterator_opt(opts);

            iter.seek_to_first();
            assert!(iter.valid());
            assert_eq!(iter.key(), Some(key1.as_slice()));
            // At ts1, key1 has val1
            assert_eq!(iter.value(), Some(val1.as_slice()));
            let ts = iter
                .timestamp()
                .expect("timestamp should be present for valid iterator");
            assert_eq!(U64Timestamp::from(ts), ts1);

            // key2 was written at ts2, so it shouldn't be visible at ts1
            iter.next();
            assert!(!iter.valid());
            assert_eq!(iter.timestamp(), None);
        }
    }
}

#[test]
fn test_iterator_without_user_defined_timestamps() {
    let path = DBPath::new("_rust_rocksdb_iterator_no_udt_test");
    {
        let db = DB::open_default(&path).unwrap();

        db.put(b"key1", b"value1").unwrap();
        db.put(b"key2", b"value2").unwrap();

        let mut iter = db.raw_iterator();

        assert!(!iter.valid());
        assert_eq!(iter.timestamp(), None);

        iter.seek_to_first();
        assert!(iter.valid());
        assert_eq!(iter.key(), Some(b"key1".as_slice()));

        // For a database without user-defined timestamps, timestamp() returns an empty slice
        let empty_slice: &[u8] = &[];
        assert_eq!(iter.timestamp(), Some(empty_slice));

        iter.next();
        assert!(iter.valid());
        assert_eq!(iter.key(), Some(b"key2".as_slice()));
        assert_eq!(iter.timestamp(), Some(empty_slice));

        iter.next();
        assert!(!iter.valid());
        assert_eq!(iter.timestamp(), None);
    }
}
