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
//

//! rustic merge operator
//!
//! ```
//! use rocksdb::{Options, DB, MergeOperands};
//!
//! fn concat_merge(new_key: &[u8],
//!                 existing_val: Option<&[u8]>,
//!                 operands: &MergeOperands)
//!                 -> Option<Vec<u8>> {
//!
//!    let mut result: Vec<u8> = Vec::with_capacity(operands.len());
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
//!    Some(result)
//! }
//!
//!let path = "_rust_path_to_rocksdb";
//!let mut opts = Options::default();
//!
//!opts.create_if_missing(true);
//!opts.set_merge_operator_associative("test operator", concat_merge);
//!{
//!    let db = DB::open(&opts, path).unwrap();
//!    let p = db.put(b"k1", b"a");
//!    db.merge(b"k1", b"b");
//!    db.merge(b"k1", b"c");
//!    db.merge(b"k1", b"d");
//!    db.merge(b"k1", b"efg");
//!    let r = db.get(b"k1");
//!    assert_eq!(r.unwrap().unwrap(), b"abcdefg");
//!}
//!let _ = DB::destroy(&opts, path);
//! ```

use libc::{self, c_char, c_int, c_void, size_t};
use std::ffi::CString;
use std::mem;
use std::ptr;
use std::slice;

pub trait MergeFn:
    Fn(&[u8], Option<&[u8]>, &MergeOperands) -> Option<Vec<u8>> + Send + Sync + 'static
{
}
impl<F> MergeFn for F where
    F: Fn(&[u8], Option<&[u8]>, &MergeOperands) -> Option<Vec<u8>> + Send + Sync + 'static
{
}

pub struct MergeOperatorCallback<F: MergeFn, PF: MergeFn> {
    pub name: CString,
    pub full_merge_fn: F,
    pub partial_merge_fn: PF,
}

pub unsafe extern "C" fn destructor_callback<F: MergeFn, PF: MergeFn>(raw_cb: *mut c_void) {
    Box::from_raw(raw_cb as *mut MergeOperatorCallback<F, PF>);
}

pub unsafe extern "C" fn delete_callback(
    _raw_cb: *mut c_void,
    value: *const c_char,
    value_length: size_t,
) {
    if !value.is_null() {
        Box::from_raw(slice::from_raw_parts_mut(
            value as *mut u8,
            value_length as usize,
        ));
    }
}

pub unsafe extern "C" fn name_callback<F: MergeFn, PF: MergeFn>(
    raw_cb: *mut c_void,
) -> *const c_char {
    let cb = &mut *(raw_cb as *mut MergeOperatorCallback<F, PF>);
    cb.name.as_ptr()
}

pub unsafe extern "C" fn full_merge_callback<F: MergeFn, PF: MergeFn>(
    raw_cb: *mut c_void,
    raw_key: *const c_char,
    key_len: size_t,
    existing_value: *const c_char,
    existing_value_len: size_t,
    operands_list: *const *const c_char,
    operands_list_len: *const size_t,
    num_operands: c_int,
    success: *mut u8,
    new_value_length: *mut size_t,
) -> *mut c_char {
    let cb = &mut *(raw_cb as *mut MergeOperatorCallback<F, PF>);
    let operands = &MergeOperands::new(operands_list, operands_list_len, num_operands);
    let key = slice::from_raw_parts(raw_key as *const u8, key_len as usize);
    let oldval = if existing_value.is_null() {
        None
    } else {
        Some(slice::from_raw_parts(
            existing_value as *const u8,
            existing_value_len as usize,
        ))
    };
    (cb.full_merge_fn)(key, oldval, operands).map_or_else(
        || {
            *new_value_length = 0;
            *success = 0_u8;
            ptr::null_mut() as *mut c_char
        },
        |result| {
            *new_value_length = result.len() as size_t;
            *success = 1_u8;
            Box::into_raw(result.into_boxed_slice()) as *mut c_char
        },
    )
}

pub unsafe extern "C" fn partial_merge_callback<F: MergeFn, PF: MergeFn>(
    raw_cb: *mut c_void,
    raw_key: *const c_char,
    key_len: size_t,
    operands_list: *const *const c_char,
    operands_list_len: *const size_t,
    num_operands: c_int,
    success: *mut u8,
    new_value_length: *mut size_t,
) -> *mut c_char {
    let cb = &mut *(raw_cb as *mut MergeOperatorCallback<F, PF>);
    let operands = &MergeOperands::new(operands_list, operands_list_len, num_operands);
    let key = slice::from_raw_parts(raw_key as *const u8, key_len as usize);
    (cb.partial_merge_fn)(key, None, operands).map_or_else(
        || {
            *new_value_length = 0;
            *success = 0_u8;
            ptr::null_mut::<c_char>()
        },
        |result| {
            *new_value_length = result.len() as size_t;
            *success = 1_u8;
            Box::into_raw(result.into_boxed_slice()) as *mut c_char
        },
    )
}

pub struct MergeOperands {
    operands_list: *const *const c_char,
    operands_list_len: *const size_t,
    num_operands: usize,
}

impl MergeOperands {
    fn new(
        operands_list: *const *const c_char,
        operands_list_len: *const size_t,
        num_operands: c_int,
    ) -> MergeOperands {
        assert!(num_operands >= 0);
        MergeOperands {
            operands_list,
            operands_list_len,
            num_operands: num_operands as usize,
        }
    }

    pub fn len(&self) -> usize {
        self.num_operands
    }

    pub fn is_empty(&self) -> bool {
        self.num_operands == 0
    }

    pub fn iter(&self) -> MergeOperandsIter {
        MergeOperandsIter {
            operands: self,
            cursor: 0,
        }
    }

    fn get_operand(&self, index: usize) -> Option<&[u8]> {
        if index >= self.num_operands {
            None
        } else {
            unsafe {
                let base = self.operands_list as usize;
                let base_len = self.operands_list_len as usize;
                let spacing = mem::size_of::<*const *const u8>();
                let spacing_len = mem::size_of::<*const size_t>();
                let len_ptr = (base_len + (spacing_len * index)) as *const size_t;
                let len = *len_ptr as usize;
                let ptr = base + (spacing * index);
                Some(slice::from_raw_parts(
                    *(ptr as *const *const u8) as *const u8,
                    len,
                ))
            }
        }
    }
}

pub struct MergeOperandsIter<'a> {
    operands: &'a MergeOperands,
    cursor: usize,
}

impl<'a> Iterator for MergeOperandsIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let operand = self.operands.get_operand(self.cursor)?;
        self.cursor += 1;
        Some(operand)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.operands.num_operands - self.cursor;
        (remaining, Some(remaining))
    }
}

impl<'a> IntoIterator for &'a MergeOperands {
    type Item = &'a [u8];
    type IntoIter = MergeOperandsIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            operands: self,
            cursor: 0,
        }
    }
}
