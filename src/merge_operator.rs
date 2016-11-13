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

//! rustic merge operator
//!
//! ```
//! use rocksdb::{Options, DB, MergeOperands};
//!
//! fn concat_merge(new_key: &[u8],
//!                 existing_val: Option<&[u8]>,
//!                 operands: &mut MergeOperands)
//!                 -> Vec<u8> {
//!
//!    let mut result: Vec<u8> = Vec::with_capacity(operands.size_hint().0);
//!    existing_val.map(|v| {
//!        for e in v {
//!            result.push(*e)
//!        }
//!    });
//!    for op in operands {
//!        for e in op {
//!            result.push(*e)
//!        }
//!    }
//!    result
//! }
//!
//! fn main() {
//!    let path = "path/to/rocksdb";
//!    let mut opts = Options::default();
//!    opts.create_if_missing(true);
//!    opts.add_merge_operator("test operator", concat_merge);
//!    let db = DB::open(&opts, path).unwrap();
//!    let p = db.put(b"k1", b"a");
//!    db.merge(b"k1", b"b");
//!    db.merge(b"k1", b"c");
//!    db.merge(b"k1", b"d");
//!    db.merge(b"k1", b"efg");
//!    let r = db.get(b"k1");
//!    assert!(r.unwrap().unwrap().to_utf8().unwrap() == "abcdefg");
//! }
//! ```

use std::ffi::CString;
use std::mem;
use std::ptr;
use std::slice;

use libc::{self, c_char, c_int, c_void, size_t};

pub type MergeFn = fn(&[u8], Option<&[u8]>, &mut MergeOperands) -> Vec<u8>;

pub struct MergeOperatorCallback {
    pub name: CString,
    pub merge_fn: MergeFn,
}

pub unsafe extern "C" fn destructor_callback(raw_cb: *mut c_void) {
    let _: Box<MergeOperatorCallback> = mem::transmute(raw_cb);
}

pub unsafe extern "C" fn name_callback(raw_cb: *mut c_void) -> *const c_char {
    let cb = &mut *(raw_cb as *mut MergeOperatorCallback);
    cb.name.as_ptr()
}

pub unsafe extern "C" fn full_merge_callback(raw_cb: *mut c_void,
                                             raw_key: *const c_char,
                                             key_len: size_t,
                                             existing_value: *const c_char,
                                             existing_value_len: size_t,
                                             operands_list: *const *const c_char,
                                             operands_list_len: *const size_t,
                                             num_operands: c_int,
                                             success: *mut u8,
                                             new_value_length: *mut size_t)
                                             -> *mut c_char {
    let cb = &mut *(raw_cb as *mut MergeOperatorCallback);
    let operands = &mut MergeOperands::new(operands_list, operands_list_len, num_operands);
    let key = slice::from_raw_parts(raw_key as *const u8, key_len as usize);
    let oldval = slice::from_raw_parts(existing_value as *const u8, existing_value_len as usize);
    let mut result = (cb.merge_fn)(key, Some(oldval), operands);
    result.shrink_to_fit();
    // TODO(tan) investigate zero-copy techniques to improve performance
    let buf = libc::malloc(result.len() as size_t);
    assert!(!buf.is_null());
    *new_value_length = result.len() as size_t;
    *success = 1 as u8;
    ptr::copy(result.as_ptr() as *mut c_void, &mut *buf, result.len());
    buf as *mut c_char
}

pub unsafe extern "C" fn partial_merge_callback(raw_cb: *mut c_void,
                                                raw_key: *const c_char,
                                                key_len: size_t,
                                                operands_list: *const *const c_char,
                                                operands_list_len: *const size_t,
                                                num_operands: c_int,
                                                success: *mut u8,
                                                new_value_length: *mut size_t)
                                                -> *mut c_char {
    let cb = &mut *(raw_cb as *mut MergeOperatorCallback);
    let operands = &mut MergeOperands::new(operands_list, operands_list_len, num_operands);
    let key = slice::from_raw_parts(raw_key as *const u8, key_len as usize);
    let mut result = (cb.merge_fn)(key, None, operands);
    result.shrink_to_fit();
    // TODO(tan) investigate zero-copy techniques to improve performance
    let buf = libc::malloc(result.len() as size_t);
    assert!(!buf.is_null());
    *new_value_length = 1 as size_t;
    *success = 1 as u8;
    ptr::copy(result.as_ptr() as *mut c_void, &mut *buf, result.len());
    buf as *mut c_char
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
           num_operands: c_int)
           -> MergeOperands {
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
        if self.cursor == self.num_operands {
            None
        } else {
            unsafe {
                let base = self.operands_list as usize;
                let base_len = self.operands_list_len as usize;
                let spacing = mem::size_of::<*const *const u8>();
                let spacing_len = mem::size_of::<*const size_t>();
                let len_ptr = (base_len + (spacing_len * self.cursor)) as *const size_t;
                let len = *len_ptr as usize;
                let ptr = base + (spacing * self.cursor);
                self.cursor += 1;
                Some(mem::transmute(slice::from_raw_parts(*(ptr as *const *const u8) as *const u8,
                                                          len)))
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.num_operands - self.cursor;
        (remaining, Some(remaining))
    }
}

#[cfg(test)]
#[allow(unused_variables)]
fn test_provided_merge(new_key: &[u8],
                       existing_val: Option<&[u8]>,
                       operands: &mut MergeOperands)
                       -> Vec<u8> {
    let nops = operands.size_hint().0;
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
    result
}

#[test]
fn mergetest() {
    use Options;
    use rocksdb::DB;

    let path = "_rust_rocksdb_mergetest";
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.set_merge_operator("test operator", test_provided_merge);
    {
        let db = DB::open(&opts, path).unwrap();
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
                match value.to_utf8() {
                    Some(v) => println!("retrieved utf8 value: {}", v),
                    None => println!("did not read valid utf-8 out of the db"),
                }
            }
            Err(_) => println!("error reading value"),
            _ => panic!("value not present"),
        }

        assert!(m.is_ok());
        let r = db.get(b"k1");
        assert!(r.unwrap().unwrap().to_utf8().unwrap() == "abcdefgh");
        assert!(db.delete(b"k1").is_ok());
        assert!(db.get(b"k1").unwrap().is_none());
    }
    assert!(DB::destroy(&opts, path).is_ok());
}
