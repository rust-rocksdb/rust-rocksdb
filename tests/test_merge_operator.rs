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
use rocksdb::{merge_operator::MergeFn, DBCompactionStyle, MergeOperands, Options, DB};
use serde::{Deserialize, Serialize};
use util::DBPath;

fn test_provided_merge(
    _new_key: &[u8],
    existing_val: Option<&[u8]>,
    operands: &MergeOperands,
) -> Option<Vec<u8>> {
    let nops = operands.len();
    let mut result: Vec<u8> = Vec::with_capacity(nops);
    if let Some(v) = existing_val {
        for e in v {
            result.push(*e);
        }
    }
    for op in operands {
        for e in op {
            result.push(*e);
        }
    }
    Some(result)
}

#[test]
fn merge_test() {
    use crate::{Options, DB};

    let db_path = DBPath::new("_rust_rocksdb_merge_test");
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.set_merge_operator_associative("test operator", test_provided_merge);

    let db = DB::open(&opts, &db_path).unwrap();
    let p = db.put(b"k1", b"a");
    assert!(p.is_ok());
    let _ = db.merge(b"k1", b"b");
    let _ = db.merge(b"k1", b"c");
    let _ = db.merge(b"k1", b"d");
    let _ = db.merge(b"k1", b"efg");
    let m = db.merge(b"k1", b"h");
    assert!(m.is_ok());
    match db.get(b"k1") {
        Ok(Some(value)) => {
            if let Ok(v) = std::str::from_utf8(&value) {
                println!("retrieved utf8 value: {}", v)
            } else {
                println!("did not read valid utf-8 out of the db")
            }
        }
        Err(_) => println!("error reading value"),
        _ => panic!("value not present"),
    }

    assert!(m.is_ok());
    let r = db.get(b"k1");
    assert_eq!(r.unwrap().unwrap(), b"abcdefgh");
    assert!(db.delete(b"k1").is_ok());
    assert!(db.get(b"k1").unwrap().is_none());
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, Default)]
struct ValueCounts {
    num_a: u32,
    num_b: u32,
    num_c: u32,
    num_d: u32,
}

impl ValueCounts {
    fn from_slice(slice: &[u8]) -> Option<Self> {
        bincode::deserialize::<Self>(slice).ok()
    }

    fn as_bytes(&self) -> Option<Vec<u8>> {
        bincode::serialize(self).ok()
    }
}

fn test_counting_partial_merge(
    _new_key: &[u8],
    _existing_val: Option<&[u8]>,
    operands: &MergeOperands,
) -> Option<Vec<u8>> {
    let nops = operands.len();
    let mut result: Vec<u8> = Vec::with_capacity(nops);
    for op in operands {
        for e in op {
            result.push(*e);
        }
    }
    Some(result)
}

fn test_counting_full_merge(
    _new_key: &[u8],
    existing_val: Option<&[u8]>,
    operands: &MergeOperands,
) -> Option<Vec<u8>> {
    let mut counts = existing_val
        .map(ValueCounts::from_slice)
        .flatten()
        .unwrap_or_default();

    for op in operands {
        for e in op {
            match *e {
                b'a' => counts.num_a += 1,
                b'b' => counts.num_b += 1,
                b'c' => counts.num_c += 1,
                b'd' => counts.num_d += 1,
                _ => {}
            }
        }
    }

    counts.as_bytes()
}

#[test]
fn counting_merge_test() {
    use std::{sync::Arc, thread};

    let db_path = DBPath::new("_rust_rocksdb_partial_merge_test");
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.set_compaction_style(DBCompactionStyle::Universal);
    opts.set_min_write_buffer_number_to_merge(10);

    opts.set_merge_operator(
        "sort operator",
        test_counting_full_merge,
        test_counting_partial_merge,
    );

    let db = Arc::new(DB::open(&opts, &db_path).unwrap());
    let _ = db.delete(b"k1");
    let _ = db.delete(b"k2");
    let _ = db.merge(b"k1", b"a");
    let _ = db.merge(b"k1", b"b");
    let _ = db.merge(b"k1", b"d");
    let _ = db.merge(b"k1", b"a");
    let _ = db.merge(b"k1", b"a");
    let _ = db.merge(b"k1", b"efg");
    for i in 0..500 {
        let _ = db.merge(b"k2", b"c");
        if i % 20 == 0 {
            let _ = db.get(b"k2");
        }
    }
    for i in 0..500 {
        let _ = db.merge(b"k2", b"c");
        if i % 20 == 0 {
            let _ = db.get(b"k2");
        }
    }
    db.compact_range(None::<&[u8]>, None::<&[u8]>);
    let d1 = db.clone();
    let d2 = db.clone();
    let d3 = db.clone();

    let h1 = thread::spawn(move || {
        for i in 0..500 {
            let _ = d1.merge(b"k2", b"c");
            if i % 20 == 0 {
                let _ = d1.get(b"k2");
            }
        }
        for i in 0..500 {
            let _ = d1.merge(b"k2", b"a");
            if i % 20 == 0 {
                let _ = d1.get(b"k2");
            }
        }
    });
    let h2 = thread::spawn(move || {
        for i in 0..500 {
            let _ = d2.merge(b"k2", b"b");
            if i % 20 == 0 {
                let _ = d2.get(b"k2");
            }
        }
        for i in 0..500 {
            let _ = d2.merge(b"k2", b"d");
            if i % 20 == 0 {
                let _ = d2.get(b"k2");
            }
        }
        d2.compact_range(None::<&[u8]>, None::<&[u8]>);
    });
    h2.join().unwrap();
    let h3 = thread::spawn(move || {
        for i in 0..500 {
            let _ = d3.merge(b"k2", b"a");
            if i % 20 == 0 {
                let _ = d3.get(b"k2");
            }
        }
        for i in 0..500 {
            let _ = d3.merge(b"k2", b"c");
            if i % 20 == 0 {
                let _ = d3.get(b"k2");
            }
        }
    });
    let m = db.merge(b"k1", b"b");

    assert!(m.is_ok());
    h3.join().unwrap();
    h1.join().unwrap();

    let value_getter = |key| match db.get(key) {
        Ok(Some(value)) => ValueCounts::from_slice(&value)
            .map_or_else(|| panic!("unable to create ValueCounts from bytes"), |v| v),
        Ok(None) => panic!("value not present"),
        Err(e) => panic!("error reading value {:?}", e),
    };

    let counts = value_getter(b"k2");
    assert_eq!(counts.num_a, 1000);
    assert_eq!(counts.num_b, 500);
    assert_eq!(counts.num_c, 2000);
    assert_eq!(counts.num_d, 500);

    let counts = value_getter(b"k1");
    assert_eq!(counts.num_a, 3);
    assert_eq!(counts.num_b, 2);
    assert_eq!(counts.num_c, 0);
    assert_eq!(counts.num_d, 1);
}

#[test]
fn failed_merge_test() {
    fn test_failing_merge(
        _key: &[u8],
        _val: Option<&[u8]>,
        _operands: &MergeOperands,
    ) -> Option<Vec<u8>> {
        None
    }
    use crate::{Options, DB};

    let db_path = DBPath::new("_rust_rocksdb_failed_merge_test");
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.set_merge_operator_associative("test operator", test_failing_merge);

    let db = DB::open(&opts, &db_path).expect("open with a merge operator");
    db.put(b"key", b"value").expect("put_ok");
    let res = db.merge(b"key", b"new value");
    match res.and_then(|_e| db.get(b"key")) {
        Ok(val) => panic!("expected merge failure to propagate, got: {:?}", val),
        Err(e) => {
            assert!(e.into_string().contains("Could not perform merge."));
        }
    }
}

fn make_merge_max_with_limit(limit: u64) -> impl MergeFn + Clone {
    move |_key: &[u8], first: Option<&[u8]>, rest: &MergeOperands| {
        let max = first
            .into_iter()
            .chain(rest)
            .map(|slice| {
                let mut bytes: [u8; 8] = Default::default();
                bytes.clone_from_slice(slice);
                u64::from_ne_bytes(bytes)
            })
            .fold(0, u64::max);
        let new_value = max.min(limit);
        Some(Vec::from(new_value.to_ne_bytes().as_ref()))
    }
}

#[test]
fn test_merge_state() {
    use {Options, DB};
    let path = "_rust_rocksdb_merge_test_state";
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.set_merge_operator_associative("max-limit-12", make_merge_max_with_limit(12));
    {
        let db = DB::open(&opts, path).unwrap();
        let p = db.put(b"k1", 1u64.to_ne_bytes());
        assert!(p.is_ok());
        let _ = db.merge(b"k1", 7u64.to_ne_bytes());
        let m = db.merge(b"k1", 64u64.to_ne_bytes());
        assert!(m.is_ok());
        match db.get(b"k1") {
            Ok(Some(value)) => {
                let mut bytes: [u8; 8] = Default::default();
                bytes.copy_from_slice(&value);
                assert_eq!(u64::from_ne_bytes(bytes), 12);
            }
            Err(_) => println!("error reading value"),
            _ => panic!("value not present"),
        }

        assert!(db.delete(b"k1").is_ok());
        assert!(db.get(b"k1").unwrap().is_none());
    }
    assert!(DB::destroy(&opts, path).is_ok());

    opts.set_merge_operator_associative("max-limit-128", make_merge_max_with_limit(128));
    {
        let db = DB::open(&opts, path).unwrap();
        let p = db.put(b"k1", 1u64.to_ne_bytes());
        assert!(p.is_ok());
        let _ = db.merge(b"k1", 7u64.to_ne_bytes());
        let m = db.merge(b"k1", 64u64.to_ne_bytes());
        assert!(m.is_ok());
        match db.get(b"k1") {
            Ok(Some(value)) => {
                let mut bytes: [u8; 8] = Default::default();
                bytes.copy_from_slice(&value);
                assert_eq!(u64::from_ne_bytes(bytes), 64);
            }
            Err(_) => println!("error reading value"),
            _ => panic!("value not present"),
        }

        assert!(db.delete(b"k1").is_ok());
        assert!(db.get(b"k1").unwrap().is_none());
    }
    assert!(DB::destroy(&opts, path).is_ok());
}
