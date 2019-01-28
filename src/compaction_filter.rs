// Copyright 2016 Tyler Neely
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
use std::ffi::CString;
use std::mem;
use std::slice;

/// Decision about how to handle compacting an object
///
/// This is returned by a compaction filter callback. Depending
/// on the value, the object may be kept, removed, or changed
/// in the database during a compaction.
pub enum Decision {
    /// Keep the old value
    Keep,
    /// Remove the object from the database
    Remove,
    /// Change the value for the key
    Change(&'static [u8]),
}

/// Function to filter compaction with.
///
/// This function takes the level of compaction, the key, and the existing value
/// and returns the decision about how to handle the Key-Value pair.
///
///  See [Options::set_compaction_filter][set_compaction_filter] for more details
///
///  [set_compaction_filter]: ../struct.Options.html#method.set_compaction_filter
pub trait CompactionFilterFn: FnMut(u32, &[u8], &[u8]) -> Decision {}
impl<F> CompactionFilterFn for F
where
    F: FnMut(u32, &[u8], &[u8]) -> Decision,
    F: Send + 'static,
{}

pub struct CompactionFilterCallback<F>
where
    F: CompactionFilterFn,
{
    pub name: CString,
    pub filter_fn: F,
}

pub unsafe extern "C" fn destructor_callback<F>(raw_cb: *mut c_void)
where
    F: CompactionFilterFn,
{
    let _: Box<CompactionFilterCallback<F>> = mem::transmute(raw_cb);
}

pub unsafe extern "C" fn name_callback<F>(raw_cb: *mut c_void) -> *const c_char
where
    F: CompactionFilterFn,
{
    let cb = &*(raw_cb as *mut CompactionFilterCallback<F>);
    cb.name.as_ptr()
}

pub unsafe extern "C" fn filter_callback<F>(
    raw_cb: *mut c_void,
    level: c_int,
    raw_key: *const c_char,
    key_length: size_t,
    existing_value: *const c_char,
    value_length: size_t,
    new_value: *mut *mut c_char,
    new_value_length: *mut size_t,
    value_changed: *mut c_uchar,
) -> c_uchar
where
    F: CompactionFilterFn,
{
    use self::Decision::*;

    let cb = &mut *(raw_cb as *mut CompactionFilterCallback<F>);
    let key = slice::from_raw_parts(raw_key as *const u8, key_length as usize);
    let oldval = slice::from_raw_parts(existing_value as *const u8, value_length as usize);
    let result = (cb.filter_fn)(level as u32, key, oldval);
    match result {
        Keep => 0,
        Remove => 1,
        Change(newval) => {
            *new_value = newval.as_ptr() as *mut c_char;
            *new_value_length = newval.len() as size_t;
            *value_changed = 1 as c_uchar;
            0
        }
    }
}