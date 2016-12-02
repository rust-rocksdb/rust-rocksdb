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

use SliceTransform;
use ffi;
use libc::{c_char, c_uchar, c_void, size_t};
use std::ffi::CString;
use std::mem;
use std::slice;

pub type TransformFn = fn(&[u8]) -> Vec<u8>;
pub type InDomainFn = fn(&[u8]) -> bool;

struct SliceTransformState {
    pub name: CString,
    pub transform_res: Vec<u8>,
    pub transform_fn: TransformFn,
    pub in_domain_fn: InDomainFn,
}

impl SliceTransform {
    pub fn new(name: &str, transform_fn: TransformFn, in_domain_fn: InDomainFn) -> Self {
        let state = SliceTransformState {
            name: CString::new(name.as_bytes()).unwrap(),
            transform_res: Vec::new(),
            transform_fn: transform_fn,
            in_domain_fn: in_domain_fn,
        };

        SliceTransform {
            inner: unsafe {
                ffi::rocksdb_slicetransform_create(mem::transmute(Box::new(state)),
                                                   Some(destructor_callback),
                                                   Some(transform_callback),
                                                   Some(in_domain_callback),
                                                   None,
                                                   Some(name_callback))
            },
        }
    }

    pub fn noop() -> Self {
        SliceTransform { inner: unsafe { ffi::rocksdb_slicetransform_create_noop() } }
    }

    pub fn fixed_prefix(length: usize) -> Self {
        SliceTransform {
            inner: unsafe { ffi::rocksdb_slicetransform_create_fixed_prefix(length as size_t) },
        }
    }
}

impl Default for SliceTransform {
    fn default() -> Self {
        Self::noop()
    }
}

impl Drop for SliceTransform {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_slicetransform_destroy(self.inner);
        }
    }
}

pub unsafe extern "C" fn destructor_callback(state: *mut c_void) {
    let _: Box<SliceTransformState> = mem::transmute(state);
}

pub unsafe extern "C" fn name_callback(state: *mut c_void) -> *const c_char {
    let state = &mut *(state as *mut SliceTransformState);
    state.name.as_ptr() as *const c_char
}

pub unsafe extern "C" fn transform_callback(state: *mut c_void,
                                            key: *const c_char,
                                            length: size_t,
                                            dst_length: *mut size_t)
                                            -> *mut c_char {
    let state = &mut *(state as *mut SliceTransformState);
    let key: &[u8] = slice::from_raw_parts(key as *const u8, length as usize);
    state.transform_res = (state.transform_fn)(key);
    *dst_length = state.transform_res.len();
    state.transform_res.as_ptr() as *mut c_char
}

pub unsafe extern "C" fn in_domain_callback(state: *mut c_void,
                                            key: *const c_char,
                                            length: size_t)
                                            -> c_uchar {
    let state = &mut *(state as *mut SliceTransformState);
    let key: &[u8] = slice::from_raw_parts(key as *const u8, length as usize);
    (state.in_domain_fn)(key) as c_uchar
}
