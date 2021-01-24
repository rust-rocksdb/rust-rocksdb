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
use util::DBPath;

fn cba(input: &[u8]) -> Box<[u8]> {
    input.to_vec().into_boxed_slice()
}

#[test]
#[allow(clippy::cognitive_complexity)]
fn test_iterator() {
    let n = DBPath::new("_rust_rocksdb_iterator_test");
    {
        let k1: Box<[u8]> = b"k1".to_vec().into_boxed_slice();
        let k2: Box<[u8]> = b"k2".to_vec().into_boxed_slice();
        let k3: Box<[u8]> = b"k3".to_vec().into_boxed_slice();
        let k4: Box<[u8]> = b"k4".to_vec().into_boxed_slice();
        let v1: Box<[u8]> = b"v1111".to_vec().into_boxed_slice();
        let v2: Box<[u8]> = b"v2222".to_vec().into_boxed_slice();
        let v3: Box<[u8]> = b"v3333".to_vec().into_boxed_slice();
        let v4: Box<[u8]> = b"v4444".to_vec().into_boxed_slice();
        let db = DB::open_default(&n).unwrap();
        let p = db.put(&*k1, &*v1);
        assert!(p.is_ok());
        let p = db.put(&*k2, &*v2);
        assert!(p.is_ok());
        let p = db.put(&*k3, &*v3);
        assert!(p.is_ok());
        let expected = vec![
            (cba(&k1), cba(&v1)),
            (cba(&k2), cba(&v2)),
            (cba(&k3), cba(&v3)),
        ];
        {
            let iterator1 = db.iterator(IteratorMode::Start);
            assert_eq!(iterator1.collect::<Vec<_>>(), expected);
        }
        // Test that it's idempotent
        {
            let iterator1 = db.iterator(IteratorMode::Start);
            assert_eq!(iterator1.collect::<Vec<_>>(), expected);
        }
        {
            let iterator1 = db.iterator(IteratorMode::Start);
            assert_eq!(iterator1.collect::<Vec<_>>(), expected);
        }
        {
            let iterator1 = db.iterator(IteratorMode::Start);
            assert_eq!(iterator1.collect::<Vec<_>>(), expected);
        }
        // Test it in reverse a few times
        {
            let iterator1 = db.iterator(IteratorMode::End);
            let mut tmp_vec = iterator1.collect::<Vec<_>>();
            tmp_vec.reverse();
            assert_eq!(tmp_vec, expected);
        }
        {
            let iterator1 = db.iterator(IteratorMode::End);
            let mut tmp_vec = iterator1.collect::<Vec<_>>();
            tmp_vec.reverse();
            assert_eq!(tmp_vec, expected);
        }
        {
            let iterator1 = db.iterator(IteratorMode::End);
            let mut tmp_vec = iterator1.collect::<Vec<_>>();
            tmp_vec.reverse();
            assert_eq!(tmp_vec, expected);
        }
        {
            let iterator1 = db.iterator(IteratorMode::End);
            let mut tmp_vec = iterator1.collect::<Vec<_>>();
            tmp_vec.reverse();
            assert_eq!(tmp_vec, expected);
        }
        {
            let iterator1 = db.iterator(IteratorMode::End);
            let mut tmp_vec = iterator1.collect::<Vec<_>>();
            tmp_vec.reverse();
            assert_eq!(tmp_vec, expected);
        }
        // Try it forward again
        {
            let iterator1 = db.iterator(IteratorMode::Start);
            assert_eq!(iterator1.collect::<Vec<_>>(), expected);
        }
        {
            let iterator1 = db.iterator(IteratorMode::Start);
            assert_eq!(iterator1.collect::<Vec<_>>(), expected);
        }

        let old_iterator = db.iterator(IteratorMode::Start);
        let p = db.put(&*k4, &*v4);
        assert!(p.is_ok());
        let expected2 = vec![
            (cba(&k1), cba(&v1)),
            (cba(&k2), cba(&v2)),
            (cba(&k3), cba(&v3)),
            (cba(&k4), cba(&v4)),
        ];
        {
            assert_eq!(old_iterator.collect::<Vec<_>>(), expected);
        }
        {
            let iterator1 = db.iterator(IteratorMode::Start);
            assert_eq!(iterator1.collect::<Vec<_>>(), expected2);
        }
        {
            let iterator1 = db.iterator(IteratorMode::From(b"k2", Direction::Forward));
            let expected = vec![
                (cba(&k2), cba(&v2)),
                (cba(&k3), cba(&v3)),
                (cba(&k4), cba(&v4)),
            ];
            assert_eq!(iterator1.collect::<Vec<_>>(), expected);
        }
        {
            let iterator1 = db.iterator(IteratorMode::From(b"k2", Direction::Reverse));
            let expected = vec![(cba(&k2), cba(&v2)), (cba(&k1), cba(&v1))];
            assert_eq!(iterator1.collect::<Vec<_>>(), expected);
        }
        {
            let iterator1 = db.iterator(IteratorMode::From(b"zz", Direction::Reverse));
            let expected = vec![(cba(&k4), cba(&v4)), (cba(&k3), cba(&v3))];
            assert_eq!(iterator1.take(2).collect::<Vec<_>>(), expected);
        }
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

fn key(k: &[u8]) -> Box<[u8]> {
    k.to_vec().into_boxed_slice()
}

#[test]
fn test_prefix_iterator() {
    let n = DBPath::new("_rust_rocksdb_prefix_iterator_test");
    {
        let a1: Box<[u8]> = key(b"aaa1");
        let a2: Box<[u8]> = key(b"aaa2");
        let b1: Box<[u8]> = key(b"bbb1");
        let b2: Box<[u8]> = key(b"bbb2");

        let prefix_extractor = rocksdb::SliceTransform::create_fixed_prefix(3);

        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_prefix_extractor(prefix_extractor);

        let db = DB::open(&opts, &n).unwrap();

        assert!(db.put(&*a1, &*a1).is_ok());
        assert!(db.put(&*a2, &*a2).is_ok());
        assert!(db.put(&*b1, &*b1).is_ok());
        assert!(db.put(&*b2, &*b2).is_ok());

        {
            let expected = vec![(cba(&a1), cba(&a1)), (cba(&a2), cba(&a2))];
            let a_iterator = db.prefix_iterator(b"aaa");
            assert_eq!(a_iterator.collect::<Vec<_>>(), expected)
        }

        {
            let expected = vec![(cba(&b1), cba(&b1)), (cba(&b2), cba(&b2))];
            let b_iterator = db.prefix_iterator(b"bbb");
            assert_eq!(b_iterator.collect::<Vec<_>>(), expected)
        }
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

        let prefix = [0, 1, 1];
        let results: Vec<_> = db
            .prefix_iterator(&prefix)
            .map(|(_, v)| std::str::from_utf8(&v).unwrap().to_string())
            .collect();

        assert_eq!(results, vec!("444", "555", "666"));
    }
}

#[test]
fn test_full_iterator() {
    let path = DBPath::new("full_iterator_test");
    {
        let a1: Box<[u8]> = key(b"aaa1");
        let a2: Box<[u8]> = key(b"aaa2");
        let b1: Box<[u8]> = key(b"bbb1");
        let b2: Box<[u8]> = key(b"bbb2");

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

        assert!(db.put(&*a1, &*a1).is_ok());
        assert!(db.put(&*a2, &*a2).is_ok());
        assert!(db.put(&*b1, &*b1).is_ok());
        assert!(db.put(&*b2, &*b2).is_ok());

        // A normal iterator won't work here since we're using a HashSkipList for our memory table
        // implementation (which buckets keys based on their prefix):
        let bad_iterator = db.iterator(IteratorMode::Start);
        assert_eq!(bad_iterator.collect::<Vec<_>>(), vec![]);

        let expected = vec![
            (cba(&a1), cba(&a1)),
            (cba(&a2), cba(&a2)),
            (cba(&b1), cba(&b1)),
            (cba(&b2), cba(&b2)),
        ];

        let a_iterator = db.full_iterator(IteratorMode::Start);
        assert_eq!(a_iterator.collect::<Vec<_>>(), expected)
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
