// Copyright 2014 Tyler Neely
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
extern crate rocksdb;

use rocksdb::{DB, Direction, IteratorMode, Options};

fn cba(input: &Box<[u8]>) -> Box<[u8]> {
    input.iter().cloned().collect::<Vec<_>>().into_boxed_slice()
}

#[test]
pub fn test_iterator() {
    let path = "_rust_rocksdb_iteratortest";
    {
        let k1: Box<[u8]> = b"k1".to_vec().into_boxed_slice();
        let k2: Box<[u8]> = b"k2".to_vec().into_boxed_slice();
        let k3: Box<[u8]> = b"k3".to_vec().into_boxed_slice();
        let k4: Box<[u8]> = b"k4".to_vec().into_boxed_slice();
        let v1: Box<[u8]> = b"v1111".to_vec().into_boxed_slice();
        let v2: Box<[u8]> = b"v2222".to_vec().into_boxed_slice();
        let v3: Box<[u8]> = b"v3333".to_vec().into_boxed_slice();
        let v4: Box<[u8]> = b"v4444".to_vec().into_boxed_slice();
        let db = DB::open_default(path).unwrap();
        let p = db.put(&*k1, &*v1);
        assert!(p.is_ok());
        let p = db.put(&*k2, &*v2);
        assert!(p.is_ok());
        let p = db.put(&*k3, &*v3);
        assert!(p.is_ok());
        let expected = vec![(cba(&k1), cba(&v1)), (cba(&k2), cba(&v2)), (cba(&k3), cba(&v3))];
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
        let expected2 = vec![(cba(&k1), cba(&v1)),
                             (cba(&k2), cba(&v2)),
                             (cba(&k3), cba(&v3)),
                             (cba(&k4), cba(&v4))];
        {
            assert_eq!(old_iterator.collect::<Vec<_>>(), expected);
        }
        {
            let iterator1 = db.iterator(IteratorMode::Start);
            assert_eq!(iterator1.collect::<Vec<_>>(), expected2);
        }
        {
            let iterator1 = db.iterator(IteratorMode::From(b"k2", Direction::Forward));
            let expected = vec![(cba(&k2), cba(&v2)), (cba(&k3), cba(&v3)), (cba(&k4), cba(&v4))];
            assert_eq!(iterator1.collect::<Vec<_>>(), expected);
        }
        {
            let iterator1 = db.iterator(IteratorMode::From(b"k2", Direction::Reverse));
            let expected = vec![(cba(&k2), cba(&v2)), (cba(&k1), cba(&v1))];
            assert_eq!(iterator1.collect::<Vec<_>>(), expected);
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
            assert!(iterator5.valid());
            let iterator6 = db.iterator(IteratorMode::From(b"k1", Direction::Reverse));
            assert!(iterator6.valid());
            let iterator7 = db.iterator(IteratorMode::From(b"k11", Direction::Reverse));
            assert!(iterator7.valid());
            let iterator8 = db.iterator(IteratorMode::From(b"k5", Direction::Reverse));
            assert!(!iterator8.valid());
        }
        {
            let mut iterator1 = db.iterator(IteratorMode::From(b"k4", Direction::Forward));
            iterator1.next();
            assert!(iterator1.valid());
            iterator1.next();
            assert!(!iterator1.valid());
        }
    }
    let opts = Options::default();
    assert!(DB::destroy(&opts, path).is_ok());
}
