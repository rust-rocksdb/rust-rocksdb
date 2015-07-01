/*
   Copyright 2014 Tyler Neely

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/
extern crate libc;
use self::libc::{c_char, c_int, c_void, size_t};
use std::ffi::CString;
use std::mem;
use std::ptr;
use std::slice;

use rocksdb_options::{RocksDBOptions};
use rocksdb::{RocksDB, RocksDBResult, RocksDBVector};

pub struct MergeOperatorCallback {
    pub name: CString,
    pub merge_fn: fn (&[u8], Option<&[u8]>, &mut MergeOperands) -> Vec<u8>,
}

pub extern "C" fn destructor_callback(raw_cb: *mut c_void) {
    // turn this back into a local variable so rust will reclaim it
    let _: Box<MergeOperatorCallback> = unsafe {mem::transmute(raw_cb)};

}

pub extern "C" fn name_callback(raw_cb: *mut c_void) -> *const c_char {
    unsafe {
        let cb: &mut MergeOperatorCallback =
            &mut *(raw_cb as *mut MergeOperatorCallback);
        let ptr = cb.name.as_ptr();
        ptr as *const c_char
    }
}

pub extern "C" fn full_merge_callback(
    raw_cb: *mut c_void, raw_key: *const c_char, key_len: size_t,
    existing_value: *const c_char, existing_value_len: size_t,
    operands_list: *const *const c_char, operands_list_len: *const size_t,
    num_operands: c_int,
    success: *mut u8, new_value_length: *mut size_t) -> *const c_char {
    unsafe {
        let cb: &mut MergeOperatorCallback =
            &mut *(raw_cb as *mut MergeOperatorCallback);
        let operands =
            &mut MergeOperands::new(operands_list,
                                    operands_list_len,
                                    num_operands);
        let key: &[u8] = slice::from_raw_parts(raw_key as *const u8, key_len as usize);
        let oldval: &[u8] = slice::from_raw_parts(existing_value as *const u8,
                                  existing_value_len as usize);
        let mut result =
            (cb.merge_fn)(key, Some(oldval), operands);
        result.shrink_to_fit();
        //TODO(tan) investigate zero-copy techniques to improve performance
        let buf = libc::malloc(result.len() as size_t);
        assert!(!buf.is_null());
        *new_value_length = result.len() as size_t;
        *success = 1 as u8;
        ptr::copy(result.as_ptr() as *mut c_void, &mut *buf, result.len());
        buf as *const c_char
    }
}

pub extern "C" fn partial_merge_callback(
    raw_cb: *mut c_void, raw_key: *const c_char, key_len: size_t,
    operands_list: *const *const c_char, operands_list_len: *const size_t,
    num_operands: c_int,
    success: *mut u8, new_value_length: *mut size_t) -> *const c_char {
    unsafe {
        let cb: &mut MergeOperatorCallback =
            &mut *(raw_cb as *mut MergeOperatorCallback);
        let operands = &mut MergeOperands::new(operands_list,
                                               operands_list_len,
                                               num_operands);
        let key: &[u8] = slice::from_raw_parts(raw_key as *const u8, key_len as usize);
        let mut result = (cb.merge_fn)(key, None, operands);
        result.shrink_to_fit();
        //TODO(tan) investigate zero-copy techniques to improve performance
        let buf = libc::malloc(result.len() as size_t);
        assert!(!buf.is_null());
        *new_value_length = 1 as size_t;
        *success = 1 as u8;
        ptr::copy(result.as_ptr() as *mut c_void, &mut *buf, result.len());
        buf as *const c_char
    }
}


pub struct MergeOperands {
    operands_list: *const *const c_char,
    operands_list_len: *const size_t,
    num_operands: usize,
    cursor: usize,
}

impl MergeOperands {
    fn new(operands_list: *const *const c_char,
           operands_list_len: *const size_t,
           num_operands: c_int) -> MergeOperands {
        assert!(num_operands >= 0);
        MergeOperands {
            operands_list: operands_list,
            operands_list_len: operands_list_len,
            num_operands: num_operands as usize,
            cursor: 0,
        }
    }
}

impl<'a> Iterator for &'a mut MergeOperands {
    type Item = &'a [u8];
    fn next(&mut self) -> Option<&'a [u8]> {
        use std::raw::Slice;
        match self.cursor == self.num_operands {
            true => None,
            false => {
                unsafe {
                    let base = self.operands_list as usize;
                    let base_len = self.operands_list_len as usize;
                    let spacing = mem::size_of::<*const *const u8>();
                    let spacing_len = mem::size_of::<*const size_t>();
                    let len_ptr = (base_len + (spacing_len * self.cursor))
                        as *const size_t;
                    let len = *len_ptr as usize;
                    let ptr = base + (spacing * self.cursor);
                    self.cursor += 1;
                    Some(mem::transmute(Slice{data:*(ptr as *const *const u8)
                        as *const u8, len: len}))
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.num_operands - self.cursor;
        (remaining, Some(remaining))
    }
}

fn test_provided_merge(new_key: &[u8], existing_val: Option<&[u8]>,
    mut operands: &mut MergeOperands) -> Vec<u8> {
    let nops = operands.size_hint().0;
    let mut result: Vec<u8> = Vec::with_capacity(nops);
    match existing_val {
        Some(v) => result.push_all(v),
        None => (),
    }
    for op in operands {
        result.push_all(op);
    }
    result
}

#[allow(dead_code)]
#[test]
fn mergetest() {
    let path = "_rust_rocksdb_mergetest";
    let opts = RocksDBOptions::new();
    opts.create_if_missing(true);
    opts.add_merge_operator("test operator", test_provided_merge);
    let db = RocksDB::open(opts, path).unwrap();
    let p = db.put(b"k1", b"a");
    assert!(p.is_ok());
    db.merge(b"k1", b"b");
    db.merge(b"k1", b"c");
    db.merge(b"k1", b"d");
    db.merge(b"k1", b"efg");
    let m = db.merge(b"k1", b"h");
    assert!(m.is_ok());
    db.get(b"k1").map( |value| {
        match value.to_utf8() {
            Some(v) =>
                println!("retrieved utf8 value: {}", v),
            None =>
                println!("did not read valid utf-8 out of the db"),
        }
    }).on_absent( || { println!("value not present!") })
      .on_error( |e| { println!("error reading value")}); //: {", e) });

    assert!(m.is_ok());
    let r: RocksDBResult<RocksDBVector, &str> = db.get(b"k1");
    assert!(r.unwrap().to_utf8().unwrap() == "abcdefgh");
    assert!(db.delete(b"k1").is_ok());
    assert!(db.get(b"k1").is_none());
    db.close();
    assert!(RocksDB::destroy(opts, path).is_ok());
}
