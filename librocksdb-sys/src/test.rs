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

use libc::*;
use std::ffi::{CStr, CString};
use std::ptr;
use std::str;

use super::*;

pub fn error_message(ptr: *const i8) -> String {
    let c_str = unsafe { CStr::from_ptr(ptr as *const _) };
    let s = str::from_utf8(c_str.to_bytes()).unwrap().to_owned();
    unsafe {
        free(ptr as *mut c_void);
    }
    s
}

#[test]
fn internal() {
    unsafe {
        let opts = rocksdb_options_create();
        assert!(!opts.is_null());

        rocksdb_options_increase_parallelism(opts, 0);
        rocksdb_options_optimize_level_style_compaction(opts, 0);
        rocksdb_options_set_create_if_missing(opts, u8::from(true));

        let rustpath = "_rust_rocksdb_internaltest";
        let cpath = CString::new(rustpath).unwrap();

        let mut err: *mut c_char = ptr::null_mut();
        let err_ptr: *mut *mut c_char = &mut err;
        let db = rocksdb_open(opts, cpath.as_ptr() as *const _, err_ptr);
        if !err.is_null() {
            println!("failed to open rocksdb: {}", error_message(err));
        }
        assert!(err.is_null());

        let writeopts = rocksdb_writeoptions_create();
        assert!(!writeopts.is_null());

        let key = b"name\x00";
        let val = b"spacejam\x00";
        rocksdb_put(
            db,
            writeopts.clone(),
            key.as_ptr() as *const c_char,
            4,
            val.as_ptr() as *const c_char,
            8,
            err_ptr,
        );
        rocksdb_writeoptions_destroy(writeopts);
        assert!(err.is_null());

        let readopts = rocksdb_readoptions_create();
        assert!(!readopts.is_null());

        let mut val_len: size_t = 0;
        let val_len_ptr = &mut val_len as *mut size_t;
        rocksdb_get(
            db,
            readopts.clone(),
            key.as_ptr() as *const c_char,
            4,
            val_len_ptr,
            err_ptr,
        );
        rocksdb_readoptions_destroy(readopts);
        assert!(err.is_null());
        rocksdb_close(db);
        rocksdb_destroy_db(opts, cpath.as_ptr() as *const _, err_ptr);
        assert!(err.is_null());
    }
}
