// Copyright 2016 Alex Regueiro
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

use crate::Error;
use libc::{self, c_char, c_void};
use std::ffi::{CStr, CString};
use std::path::Path;
use std::ptr;

pub(crate) unsafe fn from_cstr(ptr: *const c_char) -> String {
    let cstr = CStr::from_ptr(ptr as *const _);
    String::from_utf8_lossy(cstr.to_bytes()).into_owned()
}

pub(crate) unsafe fn raw_data(ptr: *const c_char, size: usize) -> Option<Vec<u8>> {
    if ptr.is_null() {
        None
    } else {
        let mut dst = vec![0; size];
        ptr::copy_nonoverlapping(ptr as *const u8, dst.as_mut_ptr(), size);

        Some(dst)
    }
}

pub fn error_message(ptr: *const c_char) -> String {
    unsafe {
        let s = from_cstr(ptr);
        libc::free(ptr as *mut c_void);
        s
    }
}

pub fn opt_bytes_to_ptr<T: AsRef<[u8]>>(opt: Option<T>) -> *const c_char {
    match opt {
        Some(v) => v.as_ref().as_ptr() as *const c_char,
        None => ptr::null(),
    }
}

pub(crate) fn to_cpath<P: AsRef<Path>>(path: P) -> Result<CString, Error> {
    match CString::new(path.as_ref().to_string_lossy().as_bytes()) {
        Ok(c) => Ok(c),
        Err(e) => Err(Error::new(format!(
            "Failed to convert path to CString: {}",
            e,
        ))),
    }
}

macro_rules! ffi_try {
    ( $($function:ident)::*() ) => {
        ffi_try_impl!($($function)::*())
    };

    ( $($function:ident)::*( $arg1:expr $(, $arg:expr)* $(,)? ) ) => {
        ffi_try_impl!($($function)::*($arg1 $(, $arg)* ,))
    };
}

macro_rules! ffi_try_impl {
    ( $($function:ident)::*( $($arg:expr,)*) ) => {{
        let mut err: *mut ::libc::c_char = ::std::ptr::null_mut();
        let result = $($function)::*($($arg,)* &mut err);
        if !err.is_null() {
            return Err(Error::new($crate::ffi_util::error_message(err)));
        }
        result
    }};
}
