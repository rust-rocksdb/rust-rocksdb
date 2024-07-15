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

// Use u64 as the timestamp. This is based on two reasons:
// 1. Follows the logic of [BytewiseComparatorWithU64Ts](https://github.com/facebook/rocksdb/blob/3db030d7ee1b887ce818ec6f6a8d10949f9e9a22/util/comparator.cc#L238)
// 2. u64 is the return type of [Duration::as_secs()](https://doc.rust-lang.org/nightly/std/time/struct.Duration.html#method.as_secs)
fn strip_timestamp_from_user_key(user_key: &[u8], ts_sz: usize) -> &[u8] {
    &user_key[..user_key.len() - ts_sz]
}

fn extract_timestamp_from_user_key(user_key: &[u8], ts_sz: usize) -> &[u8] {
    &user_key[user_key.len() - ts_sz..]
}

// Caller should ensure the pointer is valid and has at least 8 bytes,
// As the slice::from_raw_parts does in compare_ts_callback
#[inline]
fn decode_timestamp(ptr: &[u8]) -> u64 {
    u64::from_be_bytes(ptr[..8].try_into().unwrap())
}

fn compare_ts(a: &[u8], b: &[u8]) -> c_int {
    let a = decode_timestamp(a);
    let b = decode_timestamp(b);
    match a.cmp(&b) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
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

/// For two events e1 and e2 whose timestamps are t1 and t2 respectively,
/// Returns value:
/// < 0  iff t1 < t2
/// == 0 iff t1 == t2
/// > 0  iff t1 > t2
/// Note that an all-zero byte array will be the smallest (oldest) timestamp
/// of the same length, and a byte array with all bits 1 will be the largest.
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
    compare_ts(a, b)
}

/// Three-way comparison.  Returns value:
///   < 0 iff "a" < "b",
///   == 0 iff "a" == "b",
///   > 0 iff "a" > "b"
/// Note this callback also compares timestamp.
/// For the same user key with different timestamps, larger (newer)
/// timestamp comes first.
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
    -compare_ts(a_ts, b_ts)
}

/// Three-way comparison.  Returns value:
///   < 0 iff "a" < "b",
///   == 0 iff "a" == "b",
///   > 0 iff "a" > "b"
/// Note this callback ignores timestamp during comparison.
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
