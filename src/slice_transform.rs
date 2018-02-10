// Copyright 2018 Tyler Neely
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

use std::ffi::CString;
use std::mem;
use std::ptr;
use std::slice;

use libc::{self, c_char, c_void, size_t};

use ffi;

/// A SliceTranform is a generic pluggable way of transforming one string
/// to another. Its primary use-case is in configuring rocksdb
/// to store prefix blooms by setting prefix_extractor in
/// ColumnFamilyOptions.
pub struct SliceTransform {
    pub inner: *mut ffi::rocksdb_slicetransform_t,
}

// NB we intentionally don't implement a Drop that passes
// through to rocksdb_slicetransform_destroy because
// this is currently only used (to my knowledge)
// by people passing it as a prefix extractor when
// opening a DB. 

impl SliceTransform {
    pub fn create(
        name: &str,
        transform_fn: TransformFn,
        in_domain_fn: Option<InDomainFn>,
    ) -> SliceTransform{
        let cb = Box::new(TransformCallback {
            name: CString::new(name.as_bytes()).unwrap(),
            transform_fn: transform_fn,
            in_domain_fn: in_domain_fn,
        });

        let st = unsafe {
             ffi::rocksdb_slicetransform_create(
                mem::transmute(cb),
                Some(slice_transform_destructor_callback),
                Some(transform_callback),

                // this is ugly, but I can't get the compiler
                // not to barf with "expected fn pointer, found fn item"
                // without this. sorry.
                if let Some(_) = in_domain_fn {
                    Some(in_domain_callback)
                } else {
                    None
                },

                // this None points to the deprecated InRange callback
                None,
                Some(slice_transform_name_callback),
            )
        };

        SliceTransform {
            inner: st
        }
    }

    pub fn create_fixed_prefix(len: size_t) -> SliceTransform {
        SliceTransform {
            inner: unsafe {
                ffi::rocksdb_slicetransform_create_fixed_prefix(len)
            },
        }
    }

    pub fn create_noop() -> SliceTransform {
        SliceTransform {
            inner: unsafe {
                ffi::rocksdb_slicetransform_create_noop()
            },
        }
    }
}

pub type TransformFn = fn(&[u8]) -> Vec<u8>;
pub type InDomainFn = fn(&[u8]) -> bool;

pub struct TransformCallback {
	pub name: CString,
	pub transform_fn: TransformFn,
	pub in_domain_fn: Option<InDomainFn>,
}

pub unsafe extern "C" fn slice_transform_destructor_callback(
    raw_cb: *mut c_void
) {
	let transform: Box<TransformCallback> = mem::transmute(raw_cb);
	drop(transform);
}

pub unsafe extern "C" fn slice_transform_name_callback(
    raw_cb: *mut c_void
) -> *const c_char {
	let cb = &mut *(raw_cb as *mut TransformCallback);
	cb.name.as_ptr()
}

pub unsafe extern "C" fn transform_callback(
	raw_cb: *mut c_void,
	raw_key: *const c_char,
	key_len: size_t,
	dst_length: *mut size_t,
) -> *mut c_char {
	let cb = &mut *(raw_cb as *mut TransformCallback);
	let key = slice::from_raw_parts(raw_key as *const u8, key_len as usize);
	let mut result = (cb.transform_fn)(key);
    result.shrink_to_fit();

    // copy the result into a C++ destroyable buffer
    let buf = libc::malloc(result.len() as size_t);
    assert!(!buf.is_null());
    ptr::copy(result.as_ptr() as *mut c_void, &mut *buf, result.len());

    *dst_length = result.len() as size_t;
    buf as *mut c_char
}

pub unsafe extern "C" fn in_domain_callback(
    raw_cb: *mut c_void,
	raw_key: *const c_char,
	key_len: size_t,
) -> u8 {
	let cb = &mut *(raw_cb as *mut TransformCallback);
	let key = slice::from_raw_parts(raw_key as *const u8, key_len as usize);

    if (cb.in_domain_fn.unwrap())(key) {
        1
    } else {
        0
    }
}
