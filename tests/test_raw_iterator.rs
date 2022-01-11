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

use rocksdb::DB;
use util::DBPath;

#[test]
pub fn test_forwards_iteration() {
    let n = DBPath::new("forwards_iteration");
    {
        let db = DB::open_default(&n).unwrap();
        db.put(b"k1", b"v1").unwrap();
        db.put(b"k2", b"v2").unwrap();
        db.put(b"k3", b"v3").unwrap();
        db.put(b"k4", b"v4").unwrap();

        let mut iter = db.raw_iterator();
        iter.seek_to_first();

        assert!(iter.valid());
        assert_eq!(iter.key(), Some(b"k1".as_ref()));
        assert_eq!(iter.value(), Some(b"v1".as_ref()));

        iter.next();

        assert!(iter.valid());
        assert_eq!(iter.key(), Some(b"k2".as_ref()));
        assert_eq!(iter.value(), Some(b"v2".as_ref()));

        iter.next(); // k3
        iter.next(); // k4
        iter.next(); // invalid!

        assert!(!iter.valid());
        assert_eq!(iter.key(), None);
        assert_eq!(iter.value(), None);
    }
}

#[test]
pub fn test_seek_last() {
    let n = DBPath::new("backwards_iteration");
    {
        let db = DB::open_default(&n).unwrap();
        db.put(b"k1", b"v1").unwrap();
        db.put(b"k2", b"v2").unwrap();
        db.put(b"k3", b"v3").unwrap();
        db.put(b"k4", b"v4").unwrap();

        let mut iter = db.raw_iterator();
        iter.seek_to_last();

        assert!(iter.valid());
        assert_eq!(iter.key(), Some(b"k4".as_ref()));
        assert_eq!(iter.value(), Some(b"v4".as_ref()));

        iter.prev();

        assert!(iter.valid());
        assert_eq!(iter.key(), Some(b"k3".as_ref()));
        assert_eq!(iter.value(), Some(b"v3".as_ref()));

        iter.prev(); // k2
        iter.prev(); // k1
        iter.prev(); // invalid!

        assert!(!iter.valid());
        assert_eq!(iter.key(), None);
        assert_eq!(iter.value(), None);
    }
}

#[test]
pub fn test_seek() {
    let n = DBPath::new("seek");
    {
        let db = DB::open_default(&n).unwrap();
        db.put(b"k1", b"v1").unwrap();
        db.put(b"k2", b"v2").unwrap();
        db.put(b"k4", b"v4").unwrap();

        let mut iter = db.raw_iterator();
        iter.seek(b"k2");

        assert!(iter.valid());
        assert_eq!(iter.key(), Some(b"k2".as_ref()));
        assert_eq!(iter.value(), Some(b"v2".as_ref()));

        // Check it gets the next key when the key doesn't exist
        iter.seek(b"k3");

        assert!(iter.valid());
        assert_eq!(iter.key(), Some(b"k4".as_ref()));
        assert_eq!(iter.value(), Some(b"v4".as_ref()));
    }
}

#[test]
pub fn test_seek_to_nonexistant() {
    let n = DBPath::new("seek_to_nonexistant");
    {
        let db = DB::open_default(&n).unwrap();
        db.put(b"k1", b"v1").unwrap();
        db.put(b"k3", b"v3").unwrap();
        db.put(b"k4", b"v4").unwrap();

        let mut iter = db.raw_iterator();
        iter.seek(b"k2");

        assert!(iter.valid());
        assert_eq!(iter.key(), Some(b"k3".as_ref()));
        assert_eq!(iter.value(), Some(b"v3".as_ref()));
    }
}

#[test]
pub fn test_seek_for_prev() {
    let n = DBPath::new("seek_for_prev");
    {
        let db = DB::open_default(&n).unwrap();
        db.put(b"k1", b"v1").unwrap();
        db.put(b"k2", b"v2").unwrap();
        db.put(b"k4", b"v4").unwrap();

        let mut iter = db.raw_iterator();
        iter.seek(b"k2");

        assert!(iter.valid());
        assert_eq!(iter.key(), Some(b"k2".as_ref()));
        assert_eq!(iter.value(), Some(b"v2".as_ref()));

        // Check it gets the previous key when the key doesn't exist
        iter.seek_for_prev(b"k3");

        assert!(iter.valid());
        assert_eq!(iter.key(), Some(b"k2".as_ref()));
        assert_eq!(iter.value(), Some(b"v2".as_ref()));
    }
}
