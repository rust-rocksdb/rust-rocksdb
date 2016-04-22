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

use rocksdb::{DB, Direction, IteratorMode, Options, Writable, Kv};

fn collect<'a, T: Iterator<Item=Kv<'a>>>(iter: T) -> Vec<(Vec<u8>, Vec<u8>)> {
    iter.map(|(k, v)| (k.to_vec(), v.to_vec())).collect()
}

#[test]
pub fn test_iterator() {
    let path = "_rust_rocksdb_iteratortest";
    {
        let k1 = b"k1";
        let k2 = b"k2";
        let k3 = b"k3";
        let k4 = b"k4";
        let v1 = b"v1111";
        let v2 = b"v2222";
        let v3 = b"v3333";
        let v4 = b"v4444";
        let db = DB::open_default(path).unwrap();
        let p = db.put(k1, v1);
        assert!(p.is_ok());
        let p = db.put(k2, v2);
        assert!(p.is_ok());
        let p = db.put(k3, v3);
        assert!(p.is_ok());
        let expected = vec![(k1.to_vec(), v1.to_vec()),
                            (k2.to_vec(), v2.to_vec()),
                            (k3.to_vec(), v3.to_vec())];
        {
            let iterator1 = db.iterator(IteratorMode::Start);
            assert_eq!(collect(iterator1), expected);
        }
        // Test that it's idempotent
        {
            let iterator1 = db.iterator(IteratorMode::Start);
            assert_eq!(collect(iterator1), expected);
        }
        {
            let iterator1 = db.iterator(IteratorMode::Start);
            assert_eq!(collect(iterator1), expected);
        }
        {
            let iterator1 = db.iterator(IteratorMode::Start);
            assert_eq!(collect(iterator1), expected);
        }
        // Test it in reverse a few times
        {
            let iterator1 = db.iterator(IteratorMode::End);
            let mut tmp_vec = collect(iterator1);
            tmp_vec.reverse();
            assert_eq!(tmp_vec, expected);
        }
        {
            let iterator1 = db.iterator(IteratorMode::End);
            let mut tmp_vec = collect(iterator1);
            tmp_vec.reverse();
            assert_eq!(tmp_vec, expected);
        }
        {
            let iterator1 = db.iterator(IteratorMode::End);
            let mut tmp_vec = collect(iterator1);
            tmp_vec.reverse();
            assert_eq!(tmp_vec, expected);
        }
        {
            let iterator1 = db.iterator(IteratorMode::End);
            let mut tmp_vec = collect(iterator1);
            tmp_vec.reverse();
            assert_eq!(tmp_vec, expected);
        }
        {
            let iterator1 = db.iterator(IteratorMode::End);
            let mut tmp_vec = collect(iterator1);
            tmp_vec.reverse();
            assert_eq!(tmp_vec, expected);
        }
        // Try it forward again
        {
            let iterator1 = db.iterator(IteratorMode::Start);
            assert_eq!(collect(iterator1), expected);
        }
        {
            let iterator1 = db.iterator(IteratorMode::Start);
            assert_eq!(collect(iterator1), expected);
        }

        let old_iterator = db.iterator(IteratorMode::Start);
        let p = db.put(&*k4, &*v4);
        assert!(p.is_ok());
        let expected2 = vec![(k1.to_vec(), v1.to_vec()),
                             (k2.to_vec(), v2.to_vec()),
                             (k3.to_vec(), v3.to_vec()),
                             (k4.to_vec(), v4.to_vec())];
        {
            assert_eq!(collect(old_iterator), expected);
        }
        {
            let iterator1 = db.iterator(IteratorMode::Start);
            assert_eq!(collect(iterator1), expected2);
        }
        {
            let iterator1 =
                db.iterator(IteratorMode::From(b"k2", Direction::Forward));
            let expected = vec![(k2.to_vec(), v2.to_vec()),
                                (k3.to_vec(), v3.to_vec()),
                                (k4.to_vec(), v4.to_vec())];
            assert_eq!(collect(iterator1), expected);
        }
        {
            let iterator1 =
                db.iterator(IteratorMode::From(b"k2", Direction::Reverse));
            let expected = vec![(k2.to_vec(), v2.to_vec()), (k1.to_vec(), v1.to_vec())];
            assert_eq!(collect(iterator1), expected);
        }
        {
            let iterator1 =
                db.iterator(IteratorMode::From(b"k0", Direction::Forward));
            assert!(iterator1.valid());
            let iterator2 =
                db.iterator(IteratorMode::From(b"k1", Direction::Forward));
            assert!(iterator2.valid());
            let iterator3 =
                db.iterator(IteratorMode::From(b"k11", Direction::Forward));
            assert!(iterator3.valid());
            let iterator4 =
                db.iterator(IteratorMode::From(b"k5", Direction::Forward));
            assert!(!iterator4.valid());
            let iterator5 =
                db.iterator(IteratorMode::From(b"k0", Direction::Reverse));
            assert!(iterator5.valid());
            let iterator6 =
                db.iterator(IteratorMode::From(b"k1", Direction::Reverse));
            assert!(iterator6.valid());
            let iterator7 =
                db.iterator(IteratorMode::From(b"k11", Direction::Reverse));
            assert!(iterator7.valid());
            let iterator8 =
                db.iterator(IteratorMode::From(b"k5", Direction::Reverse));
            assert!(!iterator8.valid());
        }
        {
            let mut iterator1 =
                db.iterator(IteratorMode::From(b"k4", Direction::Forward));
            iterator1.next();
            assert!(iterator1.valid());
            iterator1.next();
            assert!(!iterator1.valid());
        }
    }
    let opts = Options::default();
    assert!(DB::destroy(&opts, path).is_ok());
}
