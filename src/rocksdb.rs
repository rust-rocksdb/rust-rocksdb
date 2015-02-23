/*
   Copyright 2014 Tyler Neely

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

extern crate libc;
use self::libc::{c_void, size_t};
use std::ffi::CString;
use std::fs::{self, PathExt};
use std::ops::Deref;
use std::path::Path;
use std::ptr::Unique;
use std::slice;
use std::str::from_c_str;
use std::str::from_utf8;

use rocksdb_ffi;
use rocksdb_ffi::RocksDBSnapshot;
use rocksdb_options::RocksDBOptions;

pub struct RocksDB {
    inner: rocksdb_ffi::RocksDBInstance,
}

impl RocksDB {
    pub fn open_default(path: &str) -> Result<RocksDB, &str> {
        let opts = RocksDBOptions::new();
        opts.create_if_missing(true);
        RocksDB::open(opts, path)
    }

    pub fn open(opts: RocksDBOptions, path: &str) -> Result<RocksDB, &str> {
        unsafe {
            let cpath = CString::from_slice(path.as_bytes());
            let cpath_ptr = cpath.as_ptr();

            let ospath = Path::new(path);
            if !ospath.exists() {
                match fs::create_dir_all(&ospath) {
                    Err(e) => return Err(""),
                    Ok(_) => (),
                }
            }

            let err = 0 as *mut i8;
            let db = rocksdb_ffi::rocksdb_open(opts.inner, cpath_ptr, err);
            if !err.is_null() {
                let cs = from_c_str(err as *const i8);
                return Err(cs);
            }
            if db.0.is_null() {
                return Err("Could not initialize database.");
            }
            Ok(RocksDB{inner: db})
        }
    }

    pub fn destroy(opts: RocksDBOptions, path: &str) -> Result<(), &str> {
        unsafe {
            let cpath = CString::from_slice(path.as_bytes());
            let cpath_ptr = cpath.as_ptr();

            let ospath = Path::new(path);
            if !ospath.exists() {
                return Err("path does not exist");
            }

            let err = 0 as *mut i8;
            let result = rocksdb_ffi::rocksdb_destroy_db(
                opts.inner, cpath_ptr, err);
            if !err.is_null() {
                let cs = from_c_str(err as *const i8);
                return Err(cs);
            }
            Ok(())
        }
    }

    pub fn repair(opts: RocksDBOptions, path: &str) -> Result<(), &str> {
        unsafe {
            let cpath = CString::from_slice(path.as_bytes());
            let cpath_ptr = cpath.as_ptr();

            let ospath = Path::new(path);
            if !ospath.exists() {
                return Err("path does not exist");
            }

            let err = 0 as *mut i8;
            let result = rocksdb_ffi::rocksdb_repair_db(
                opts.inner, cpath_ptr, err);
            if !err.is_null() {
                let cs = from_c_str(err as *const i8);
                return Err(cs);
            }
            Ok(())
        }
    }

    pub fn create_snapshot(self) -> RocksDBSnapshot {
        unsafe {
            rocksdb_ffi::rocksdb_create_snapshot(self.inner)
        }
    }

    pub fn put(&self, key: &[u8], value: &[u8]) -> Result<(), &str> {
        unsafe {
            let writeopts = rocksdb_ffi::rocksdb_writeoptions_create();
            let err = 0 as *mut i8;
            rocksdb_ffi::rocksdb_put(self.inner, writeopts, key.as_ptr(),
                        key.len() as size_t, value.as_ptr(),
                        value.len() as size_t, err);
            if !err.is_null() {
                let cs = from_c_str(err as *const i8);
                return Err(cs);
            }
            return Ok(())
        }
    }

    pub fn merge(&self, key: &[u8], value: &[u8]) -> Result<(), &str> {
        unsafe {
            let writeopts = rocksdb_ffi::rocksdb_writeoptions_create();
            let err = 0 as *mut i8;
            rocksdb_ffi::rocksdb_merge(self.inner, writeopts, key.as_ptr(),
                        key.len() as size_t, value.as_ptr(),
                        value.len() as size_t, err);
            if !err.is_null() {
                let cs = from_c_str(err as *const i8);
                return Err(cs);
            }
            return Ok(())
        }
    }

    pub fn get(&self, key: &[u8]) -> RocksDBResult<RocksDBVector, &str> {
        unsafe {
            let readopts = rocksdb_ffi::rocksdb_readoptions_create();
            if readopts.0.is_null() {
                return RocksDBResult::Error("Unable to create rocksdb read \
                    options.  This is a fairly trivial call, and its failure \
                    may be indicative of a mis-compiled or mis-loaded rocksdb \
                    library.");
            }

            let val_len: size_t = 0;
            let val_len_ptr = &val_len as *const size_t;
            let err = 0 as *mut i8;
            let val = rocksdb_ffi::rocksdb_get(self.inner, readopts,
                key.as_ptr(), key.len() as size_t, val_len_ptr, err) as *mut u8;
            if !err.is_null() {
                let cs = from_c_str(err as *const i8);
                return RocksDBResult::Error(cs);
            }
            match val.is_null() {
                true => RocksDBResult::None,
                false => {
                    RocksDBResult::Some(RocksDBVector::from_c(val, val_len))
                }
            }
        }
    }

    pub fn delete(&self, key: &[u8]) -> Result<(),&str> {
        unsafe {
            let writeopts = rocksdb_ffi::rocksdb_writeoptions_create();
            let err = 0 as *mut i8;
            rocksdb_ffi::rocksdb_delete(self.inner, writeopts, key.as_ptr(),
                        key.len() as size_t, err);
            if !err.is_null() {
                let cs = from_c_str(err as *const i8);
                return Err(cs);
            }
            return Ok(())
        }
    }

    pub fn close(&self) {
        unsafe { rocksdb_ffi::rocksdb_close(self.inner); }
    }
}

pub struct RocksDBVector {
    base: Unique<u8>,
    len: usize,
}

impl Deref for RocksDBVector {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        unsafe { slice::from_raw_mut_buf(self.base.deref(), self.len) }
    }
}

impl Drop for RocksDBVector {
    fn drop(&mut self) {
        unsafe {
            libc::free(*self.base.deref() as *mut libc::c_void);
        }
    }
}

impl RocksDBVector {
    pub fn from_c(val: *mut u8, val_len: size_t) -> RocksDBVector {
        unsafe {
            let base = Unique::new(val);
            RocksDBVector {
                base: base,
                len: val_len as usize,
            }
        }
    }

    pub fn to_utf8<'a>(&'a self) -> Option<&'a str> {
        from_utf8(self.deref()).ok()
    }
}

// RocksDBResult exists because of the inherent difference between
// an operational failure and the absence of a possible result.
#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Debug)]
pub enum RocksDBResult<T, E> {
    Some(T),
    None,
    Error(E),
}

impl <T, E> RocksDBResult<T, E> {
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> RocksDBResult<U, E> {
        match self {
            RocksDBResult::Some(x) => RocksDBResult::Some(f(x)),
            RocksDBResult::None => RocksDBResult::None,
            RocksDBResult::Error(e) => RocksDBResult::Error(e),
        }
    }

    pub fn unwrap(self) -> T {
        match self {
            RocksDBResult::Some(x) => x,
            RocksDBResult::None =>
                panic!("Attempted unwrap on RocksDBResult::None"),
            RocksDBResult::Error(_) =>
                panic!("Attempted unwrap on RocksDBResult::Error"),
        }
    }

    pub fn on_error<U, F: FnOnce(E) -> U>(self, f: F) -> RocksDBResult<T, U> {
        match self {
            RocksDBResult::Some(x) => RocksDBResult::Some(x),
            RocksDBResult::None => RocksDBResult::None,
            RocksDBResult::Error(e) => RocksDBResult::Error(f(e)),
        }
    }

    pub fn on_absent<F: FnOnce()->()>(self, f: F) -> RocksDBResult<T, E> {
        match self {
            RocksDBResult::Some(x) => RocksDBResult::Some(x),
            RocksDBResult::None => {
                f();
                RocksDBResult::None
            },
            RocksDBResult::Error(e) => RocksDBResult::Error(e),
        }
    }

    pub fn is_some(self) -> bool {
        match self {
            RocksDBResult::Some(_) => true,
            RocksDBResult::None => false,
            RocksDBResult::Error(_) => false,
        }
    }
    pub fn is_none(self) -> bool {
        match self {
            RocksDBResult::Some(_) => false,
            RocksDBResult::None => true,
            RocksDBResult::Error(_) => false,
        }
    }
    pub fn is_error(self) -> bool {
        match self {
            RocksDBResult::Some(_) => false,
            RocksDBResult::None => false,
            RocksDBResult::Error(_) => true,
        }
    }
}

#[allow(dead_code)]
#[test]
fn external() {
    let path = "_rust_rocksdb_externaltest";
    let db = RocksDB::open_default(path).unwrap();
    let p = db.put(b"k1", b"v1111");
    assert!(p.is_ok());
    let r: RocksDBResult<RocksDBVector, &str> = db.get(b"k1");
    assert!(r.unwrap().to_utf8().unwrap() == "v1111");
    assert!(db.delete(b"k1").is_ok());
    assert!(db.get(b"k1").is_none());
    db.close();
    let opts = RocksDBOptions::new();
    assert!(RocksDB::destroy(opts, path).is_ok());
}
