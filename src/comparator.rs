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

use libc::{c_char, c_int, c_uchar, c_void, size_t};
use std::cmp::Ordering;
use std::ffi::CString;
use std::slice;

pub type CompareFn = dyn Fn(&[u8], &[u8]) -> Ordering;

pub type CompareTsFn = dyn Fn(&[u8], &[u8]) -> Ordering;

pub type CompareWithoutTsFn = dyn Fn(&[u8], bool, &[u8], bool) -> Ordering;

pub struct ComparatorCallback {
    pub name: CString,
    pub compare_fn: Box<CompareFn>,
}

impl ComparatorCallback {
    pub unsafe extern "C" fn destructor_callback(raw_cb: *mut c_void) {
        drop(Box::from_raw(raw_cb as *mut Self));
    }

    pub unsafe extern "C" fn name_callback(raw_cb: *mut c_void) -> *const c_char {
        let cb: &mut Self = &mut *(raw_cb as *mut Self);
        let ptr = cb.name.as_ptr();
        ptr as *const c_char
    }

    pub unsafe extern "C" fn compare_callback(
        raw_cb: *mut c_void,
        a_raw: *const c_char,
        a_len: size_t,
        b_raw: *const c_char,
        b_len: size_t,
    ) -> c_int {
        let cb: &mut Self = &mut *(raw_cb as *mut Self);
        let a: &[u8] = slice::from_raw_parts(a_raw as *const u8, a_len);
        let b: &[u8] = slice::from_raw_parts(b_raw as *const u8, b_len);
        (cb.compare_fn)(a, b) as c_int
    }
}

pub struct ComparatorWithTsCallback {
    pub name: CString,
    pub compare_fn: Box<CompareFn>,
    pub compare_ts_fn: Box<CompareTsFn>,
    pub compare_without_ts_fn: Box<CompareWithoutTsFn>,
}

impl ComparatorWithTsCallback {
    pub unsafe extern "C" fn destructor_callback(raw_cb: *mut c_void) {
        drop(Box::from_raw(raw_cb as *mut Self));
    }

    pub unsafe extern "C" fn name_callback(raw_cb: *mut c_void) -> *const c_char {
        let cb: &mut Self = &mut *(raw_cb as *mut Self);
        let ptr = cb.name.as_ptr();
        ptr as *const c_char
    }

    pub unsafe extern "C" fn compare_callback(
        raw_cb: *mut c_void,
        a_raw: *const c_char,
        a_len: size_t,
        b_raw: *const c_char,
        b_len: size_t,
    ) -> c_int {
        let cb: &mut Self = &mut *(raw_cb as *mut Self);
        let a: &[u8] = slice::from_raw_parts(a_raw as *const u8, a_len);
        let b: &[u8] = slice::from_raw_parts(b_raw as *const u8, b_len);
        (cb.compare_fn)(a, b) as c_int
    }

    pub unsafe extern "C" fn compare_ts_callback(
        raw_cb: *mut c_void,
        a_ts_raw: *const c_char,
        a_ts_len: size_t,
        b_ts_raw: *const c_char,
        b_ts_len: size_t,
    ) -> c_int {
        let cb: &mut Self = &mut *(raw_cb as *mut Self);
        let a_ts: &[u8] = slice::from_raw_parts(a_ts_raw as *const u8, a_ts_len);
        let b_ts: &[u8] = slice::from_raw_parts(b_ts_raw as *const u8, b_ts_len);
        (cb.compare_ts_fn)(a_ts, b_ts) as c_int
    }

    pub unsafe extern "C" fn compare_without_ts_callback(
        raw_cb: *mut c_void,
        a_raw: *const c_char,
        a_len: size_t,
        a_has_ts_raw: c_uchar,
        b_raw: *const c_char,
        b_len: size_t,
        b_has_ts_raw: c_uchar,
    ) -> c_int {
        let cb: &mut Self = &mut *(raw_cb as *mut Self);
        let a: &[u8] = slice::from_raw_parts(a_raw as *const u8, a_len);
        let a_has_ts = a_has_ts_raw != 0;
        let b: &[u8] = slice::from_raw_parts(b_raw as *const u8, b_len);
        let b_has_ts = b_has_ts_raw != 0;
        (cb.compare_without_ts_fn)(a, a_has_ts, b, b_has_ts) as c_int
    }
}
