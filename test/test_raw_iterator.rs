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

use rocksdb::DB;


fn setup_test_db(name: &str) -> DB {
    use std::fs::remove_dir_all;

    let path = "_rust_rocksdb_rawiteratortest_".to_string() + name;

    match remove_dir_all(&path) {
        Ok(_) => {}
        Err(_) => {}  // Don't care if tis fails
    }

    DB::open_default(path).unwrap()
}


#[test]
pub fn test_forwards_iteration() {
    let db = setup_test_db("forwards_iteration");
    db.put(b"k1", b"v1").unwrap();
    db.put(b"k2", b"v2").unwrap();
    db.put(b"k3", b"v3").unwrap();
    db.put(b"k4", b"v4").unwrap();

    let mut iter = db.raw_iterator();
    iter.seek_to_first();

    // Shouldn't be valid yet
    assert_eq!(iter.valid(), false);
    assert_eq!(iter.key(), None);
    assert_eq!(iter.value(), None);

    let valid = iter.next();

    assert_eq!(valid, true);
    assert_eq!(iter.valid(), true);
    assert_eq!(iter.key(), Some(b"k1".to_vec()));
    assert_eq!(iter.value(), Some(b"v1".to_vec()));

    let valid = iter.next();

    assert_eq!(valid, true);
    assert_eq!(iter.valid(), true);
    assert_eq!(iter.key(), Some(b"k2".to_vec()));
    assert_eq!(iter.value(), Some(b"v2".to_vec()));

    iter.next();  // k3
    iter.next();  // k4

    let valid = iter.next();

    // Should be invalid again
    assert_eq!(valid, false);
    assert_eq!(iter.valid(), false);
    assert_eq!(iter.key(), None);
    assert_eq!(iter.value(), None);
}


#[test]
pub fn test_seek_last() {
    let db = setup_test_db("backwards_iteration");
    db.put(b"k1", b"v1").unwrap();
    db.put(b"k2", b"v2").unwrap();
    db.put(b"k3", b"v3").unwrap();
    db.put(b"k4", b"v4").unwrap();

    let mut iter = db.raw_iterator();
    iter.seek_to_last();

    // Shouldn't be valid yet
    assert_eq!(iter.valid(), false);
    assert_eq!(iter.key(), None);
    assert_eq!(iter.value(), None);

    let valid = iter.prev();

    assert_eq!(valid, true);
    assert_eq!(iter.valid(), true);
    assert_eq!(iter.key(), Some(b"k4".to_vec()));
    assert_eq!(iter.value(), Some(b"v4".to_vec()));

    let valid = iter.prev();

    assert_eq!(valid, true);
    assert_eq!(iter.valid(), true);
    assert_eq!(iter.key(), Some(b"k3".to_vec()));
    assert_eq!(iter.value(), Some(b"v3".to_vec()));

    iter.prev();  // k2
    iter.prev();  // k1

    let valid = iter.prev();

    // Should be invalid again
    assert_eq!(valid, false);
    assert_eq!(iter.valid(), false);
    assert_eq!(iter.key(), None);
    assert_eq!(iter.value(), None);
}


#[test]
pub fn test_seek() {
    let db = setup_test_db("seek");
    db.put(b"k1", b"v1").unwrap();
    db.put(b"k2", b"v2").unwrap();
    db.put(b"k3", b"v3").unwrap();
    db.put(b"k4", b"v4").unwrap();

    let mut iter = db.raw_iterator();
    iter.seek(b"k2");

    // Shouldn't be valid yet
    assert_eq!(iter.valid(), false);
    assert_eq!(iter.key(), None);
    assert_eq!(iter.value(), None);

    let valid = iter.next();

    // Should now be valid
    assert_eq!(valid, true);
    assert_eq!(iter.valid(), true);
    assert_eq!(iter.key(), Some(b"k2".to_vec()));
    assert_eq!(iter.value(), Some(b"v2".to_vec()));
}


#[test]
pub fn test_seek_then_prev() {
    let db = setup_test_db("seek_then_prev");
    db.put(b"k1", b"v1").unwrap();
    db.put(b"k2", b"v2").unwrap();
    db.put(b"k3", b"v3").unwrap();
    db.put(b"k4", b"v4").unwrap();

    let mut iter = db.raw_iterator();
    iter.seek(b"k2");

    // Shouldn't be valid yet
    assert_eq!(iter.valid(), false);
    assert_eq!(iter.key(), None);
    assert_eq!(iter.value(), None);

    let valid = iter.prev();

    // Should now be valid
    assert_eq!(valid, true);
    assert_eq!(iter.valid(), true);
    assert_eq!(iter.key(), Some(b"k2".to_vec()));
    assert_eq!(iter.value(), Some(b"v2".to_vec()));
}


#[test]
pub fn test_seek_to_nonexistant() {
    let db = setup_test_db("seek_to_nonexistant");
    db.put(b"k1", b"v1").unwrap();
    db.put(b"k3", b"v3").unwrap();
    db.put(b"k4", b"v4").unwrap();

    let mut iter = db.raw_iterator();
    iter.seek(b"k2");

    // Shouldn't be valid yet
    assert_eq!(iter.valid(), false);
    assert_eq!(iter.key(), None);
    assert_eq!(iter.value(), None);

    let valid = iter.next();

    assert_eq!(valid, true);
    assert_eq!(iter.valid(), true);
    assert_eq!(iter.key(), Some(b"k3".to_vec()));
    assert_eq!(iter.value(), Some(b"v3".to_vec()));
}
