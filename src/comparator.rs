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

use libc::{c_char, c_int, c_void, size_t};
use std::cmp::Ordering;
use std::ffi::CString;
use std::slice;

pub type CompareFn = fn(&[u8], &[u8]) -> Ordering;

pub struct ComparatorCallback {
    pub name: CString,
    pub f: CompareFn,
}

pub unsafe extern "C" fn destructor_callback(raw_cb: *mut c_void) {
    drop(Box::from_raw(raw_cb as *mut ComparatorCallback));
}

pub unsafe extern "C" fn name_callback(raw_cb: *mut c_void) -> *const c_char {
    let cb: &mut ComparatorCallback = &mut *(raw_cb as *mut ComparatorCallback);
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
    let cb: &mut ComparatorCallback = &mut *(raw_cb as *mut ComparatorCallback);
    let a: &[u8] = slice::from_raw_parts(a_raw as *const u8, a_len);
    let b: &[u8] = slice::from_raw_parts(b_raw as *const u8, b_len);
    match (cb.f)(a, b) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}
