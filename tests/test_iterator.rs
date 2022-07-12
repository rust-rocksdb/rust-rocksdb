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

use rocksdb::{Direction, IteratorMode, MemtableFactory, Options, DB};
use util::{assert_iter, assert_iter_reversed, pair, DBPath};

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
            let iterator1 = db.iterator(IteratorMode::From(b"k0", Direction::Forward));
            assert!(iterator1.valid());
            let iterator2 = db.iterator(IteratorMode::From(b"k1", Direction::Forward));
            assert!(iterator2.valid());
            let iterator3 = db.iterator(IteratorMode::From(b"k11", Direction::Forward));
            assert!(iterator3.valid());
            let iterator4 = db.iterator(IteratorMode::From(b"k5", Direction::Forward));
            assert!(!iterator4.valid());
            let iterator5 = db.iterator(IteratorMode::From(b"k0", Direction::Reverse));
            assert!(!iterator5.valid());
            let iterator6 = db.iterator(IteratorMode::From(b"k1", Direction::Reverse));
            assert!(iterator6.valid());
            let iterator7 = db.iterator(IteratorMode::From(b"k11", Direction::Reverse));
            assert!(iterator7.valid());
            let iterator8 = db.iterator(IteratorMode::From(b"k5", Direction::Reverse));
            assert!(iterator8.valid());
        }
        {
            let mut iterator1 = db.iterator(IteratorMode::From(b"k4", Direction::Forward));
            iterator1.next();
            assert!(iterator1.valid());
            iterator1.next();
            assert!(!iterator1.valid());
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
            db.prefix_iterator(&[0, 1, 1]),
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
