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

use Comparator;
use ffi;
use libc::{c_char, c_int, c_void, size_t};
use std::cmp::Ordering;
use std::ffi::CString;
use std::mem;
use std::slice;

pub type CompareFn = fn(&[u8], &[u8]) -> Ordering;

struct ComparatorState {
    pub name: CString,
    pub compare_fn: CompareFn,
}

impl Comparator {
    pub fn new(name: &str, compare_fn: CompareFn) -> Self {
        let state = ComparatorState {
            name: CString::new(name.as_bytes()).unwrap(),
            compare_fn: compare_fn,
        };

        Comparator {
            inner: unsafe {
                ffi::rocksdb_comparator_create(mem::transmute(Box::new(state)),
                                               Some(destructor_callback),
                                               Some(compare_callback),
                                               Some(name_callback))
            },
        }
    }
}

impl Drop for Comparator {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_comparator_destroy(self.inner);
        }
    }
}

unsafe extern "C" fn destructor_callback(state: *mut c_void) {
    let _: Box<ComparatorState> = mem::transmute(state);
}

unsafe extern "C" fn name_callback(state: *mut c_void) -> *const c_char {
    let state = &mut *(state as *mut ComparatorState);
    state.name.as_ptr() as *const c_char
}

unsafe extern "C" fn compare_callback(state: *mut c_void,
                                      a: *const c_char,
                                      alen: size_t,
                                      b: *const c_char,
                                      blen: size_t)
                                      -> c_int {
    let state = &mut *(state as *mut ComparatorState);
    let a: &[u8] = slice::from_raw_parts(a as *const u8, alen as usize);
    let b: &[u8] = slice::from_raw_parts(b as *const u8, blen as usize);
    match (state.compare_fn)(a, b) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}
