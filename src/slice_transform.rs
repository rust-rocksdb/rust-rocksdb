// Copyright 2017 PingCAP, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// See the License for the specific language governing permissions and
// limitations under the License.

use crocksdb_ffi::{self, DBSliceTransform};
use libc::{c_char, c_void, size_t};
use std::ffi::CString;
use std::slice;

// `SliceTranform` is a generic pluggable way of transforming one string
// mainly used for prefix blooms.
pub trait SliceTransform {
    // Extract a prefix from a specified key
    fn transform<'a>(&mut self, key: &'a [u8]) -> &'a [u8];

    // Determine whether the specified key is compatible with the logic
    // specified in the Transform method. This method is invoked for every
    // key that is inserted into the db. If this method returns true,
    // then Transform is called to translate the key to its prefix and
    // that returned prefix is inserted into the bloom filter. If this
    // method returns false, then the call to Transform is skipped and
    // no prefix is inserted into the bloom filters.
    fn in_domain(&mut self, key: &[u8]) -> bool;

    // This is currently not used and remains here for backward compatibility.
    fn in_range(&mut self, _: &[u8]) -> bool {
        true
    }
}

#[repr(C)]
pub struct SliceTransformProxy {
    name: CString,
    transform: Box<dyn SliceTransform>,
}

extern "C" fn name(transform: *mut c_void) -> *const c_char {
    unsafe { (*(transform as *mut SliceTransformProxy)).name.as_ptr() }
}

extern "C" fn destructor(transform: *mut c_void) {
    unsafe {
        Box::from_raw(transform as *mut SliceTransformProxy);
    }
}

extern "C" fn transform(
    transform: *mut c_void,
    key: *const u8,
    key_len: size_t,
    dest_len: *mut size_t,
) -> *const u8 {
    unsafe {
        let transform = &mut *(transform as *mut SliceTransformProxy);
        let key = slice::from_raw_parts(key, key_len);
        let prefix = transform.transform.transform(key);
        *dest_len = prefix.len() as size_t;
        prefix.as_ptr() as *const u8
    }
}

extern "C" fn in_domain(transform: *mut c_void, key: *const u8, key_len: size_t) -> u8 {
    unsafe {
        let transform = &mut *(transform as *mut SliceTransformProxy);
        let key = slice::from_raw_parts(key, key_len);
        transform.transform.in_domain(key) as u8
    }
}

extern "C" fn in_range(transform: *mut c_void, key: *const u8, key_len: size_t) -> u8 {
    unsafe {
        let transform = &mut *(transform as *mut SliceTransformProxy);
        let key = slice::from_raw_parts(key, key_len);
        transform.transform.in_range(key) as u8
    }
}

pub unsafe fn new_slice_transform(
    c_name: CString,
    f: Box<dyn SliceTransform>,
) -> Result<*mut DBSliceTransform, String> {
    let proxy = Box::into_raw(Box::new(SliceTransformProxy {
        name: c_name,
        transform: f,
    }));
    let transform = crocksdb_ffi::crocksdb_slicetransform_create(
        proxy as *mut c_void,
        destructor,
        transform,
        in_domain,
        in_range,
        name,
    );
    Ok(transform)
}
