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
use std::convert::TryInto;
use std::ffi::CString;
use std::mem::size_of;
use std::slice;

pub type CompareFn = dyn Fn(&[u8], &[u8]) -> Ordering;

pub fn strip_timestamp_from_user_key(user_key: &[u8], ts_sz: usize) -> &[u8] {
    &user_key[..user_key.len() - ts_sz]
}

pub fn extract_timestamp_from_user_key(user_key: &[u8], ts_sz: usize) -> &[u8] {
    &user_key[user_key.len() - ts_sz..]
}

#[inline]
pub fn decode_timestamp(ptr: &[u8]) -> u64 {
    u64::from_be_bytes(ptr[..8].try_into().unwrap())
}

pub fn compare_ts(a: &[u8], b: &[u8]) -> c_int {
    let a = decode_timestamp(a);
    let b = decode_timestamp(b);
    return if a < b {
        -1
    } else if a > b {
        1
    } else {
        0
    };
}

pub struct ComparatorCallback {
    pub name: CString,
    pub f: Box<CompareFn>,
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

pub unsafe extern "C" fn compare_ts_callback(
    raw_cb: *mut c_void,
    a_ts: *const c_char,
    a_ts_len: size_t,
    b_ts: *const c_char,
    b_ts_len: size_t,
) -> c_int {
    let _: &mut ComparatorCallback = &mut *(raw_cb as *mut ComparatorCallback);
    assert_eq!(a_ts_len, size_of::<u64>());
    assert_eq!(b_ts_len, size_of::<u64>());
    let a: &[u8] = slice::from_raw_parts(a_ts as *const u8, a_ts_len);
    let b: &[u8] = slice::from_raw_parts(b_ts as *const u8, b_ts_len);
    return compare_ts(a, b);
}

pub unsafe extern "C" fn compare_with_ts_callback(
    raw_cb: *mut c_void,
    a_raw: *const c_char,
    a_len: size_t,
    b_raw: *const c_char,
    b_len: size_t,
) -> c_int {
    let cb: &mut ComparatorCallback = &mut *(raw_cb as *mut ComparatorCallback);
    let a: &[u8] = slice::from_raw_parts(a_raw as *const u8, a_len);
    let b: &[u8] = slice::from_raw_parts(b_raw as *const u8, b_len);
    let ts_sz = size_of::<u64>();
    let a_key = strip_timestamp_from_user_key(a, ts_sz);
    let b_key = strip_timestamp_from_user_key(b, ts_sz);

    let res = match (cb.f)(a_key, b_key) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    };
    if res != 0 {
        return res;
    }
    let a_ts = extract_timestamp_from_user_key(a, ts_sz);
    let b_ts = extract_timestamp_from_user_key(b, ts_sz);
    return -compare_ts(a_ts, b_ts);
}

pub unsafe extern "C" fn compare_without_ts_callback(
    raw_cb: *mut c_void,
    a_raw: *const c_char,
    a_len: size_t,
    a_has_ts: c_uchar,
    b_raw: *const c_char,
    b_len: size_t,
    b_has_ts: c_uchar,
) -> c_int {
    let cb: &mut ComparatorCallback = &mut *(raw_cb as *mut ComparatorCallback);
    let a: &[u8] = slice::from_raw_parts(a_raw as *const u8, a_len);
    let b: &[u8] = slice::from_raw_parts(b_raw as *const u8, b_len);
    let ts_sz = size_of::<u64>();
    let a_has_ts = a_has_ts != 0;
    let b_has_ts = b_has_ts != 0;
    assert!(!a_has_ts || a.len() >= ts_sz);
    assert!(!b_has_ts || b.len() >= ts_sz);
    let lhs = if a_has_ts {
        strip_timestamp_from_user_key(a, ts_sz)
    } else {
        a
    };
    let rhs = if b_has_ts {
        strip_timestamp_from_user_key(b, ts_sz)
    } else {
        b
    };
    match (cb.f)(lhs, rhs) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}
