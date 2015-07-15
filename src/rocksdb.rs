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
use std::ffi::{CString, CStr};
use std::fs::{self, PathExt};
use std::ops::Deref;
use std::path::Path;
use std::ptr::Unique;
use std::slice;
use std::str::from_utf8;
use std::marker::PhantomData;

use rocksdb_ffi;
use rocksdb_options::Options;

pub struct RocksDB {
    inner: rocksdb_ffi::RocksDBInstance,
}

pub struct WriteBatch {
    inner: rocksdb_ffi::RocksDBWriteBatch,
}

pub struct ReadOptions {
    inner: rocksdb_ffi::RocksDBReadOptions,
}

pub struct Snapshot<'a> {
    db: &'a RocksDB,
    inner: rocksdb_ffi::RocksDBSnapshot,
}

pub struct DBIterator {
//TODO: should have a reference to DB to enforce scope, but it's trickier than I thought to add
    inner: rocksdb_ffi::RocksDBIterator,
    direction: Direction,
    just_seeked: bool
}

pub enum Direction {
  forward, reverse
}

pub struct SubDBIterator<'a> {
    iter: &'a mut DBIterator,
    direction: Direction,
}

impl <'a> Iterator for SubDBIterator<'a> {
    type Item = (&'a [u8], &'a [u8]);

    fn next(&mut self) -> Option<(&'a[u8], &'a[u8])> {
        let native_iter = self.iter.inner;
        if !self.iter.just_seeked {
            match self.direction {
                Direction::forward => unsafe { rocksdb_ffi::rocksdb_iter_next(native_iter) },
                Direction::reverse => unsafe { rocksdb_ffi::rocksdb_iter_prev(native_iter) },
            }
        } else {
            self.iter.just_seeked = false;
        }
        if unsafe { rocksdb_ffi::rocksdb_iter_valid(native_iter) } {
            let mut key_len: size_t = 0;
            let key_len_ptr: *mut size_t = &mut key_len;
            let mut val_len: size_t = 0;
            let val_len_ptr: *mut size_t = &mut val_len;
            let key_ptr = unsafe { rocksdb_ffi::rocksdb_iter_key(native_iter, key_len_ptr) };
            let key = unsafe { slice::from_raw_parts(key_ptr, key_len as usize) };
            let val_ptr = unsafe { rocksdb_ffi::rocksdb_iter_value(native_iter, val_len_ptr) };
            let val = unsafe { slice::from_raw_parts(val_ptr, val_len as usize) };
            Some((key,val))
        } else {
            None
        }
    }
}

impl DBIterator {
//TODO alias db & opts to different lifetimes, and DBIterator to the db's lifetime 
    fn new(db: &RocksDB, readopts: &ReadOptions) -> DBIterator {
        unsafe {
            let iterator = rocksdb_ffi::rocksdb_create_iterator(db.inner, readopts.inner);
            rocksdb_ffi::rocksdb_iter_seek_to_first(iterator);
            DBIterator{ inner: iterator, direction: Direction::forward, just_seeked: true }
        }
    }

    pub fn from_start(&mut self) -> SubDBIterator {
        self.just_seeked = true;
        unsafe {
            rocksdb_ffi::rocksdb_iter_seek_to_first(self.inner);
        };
        SubDBIterator{ iter: self, direction: Direction::forward, }
    }

    pub fn from_end(&mut self) -> SubDBIterator {
        self.just_seeked = true;
        unsafe {
            rocksdb_ffi::rocksdb_iter_seek_to_last(self.inner);
        };
        SubDBIterator{ iter: self, direction: Direction::reverse, }
    }

    pub fn from(&mut self, key: &[u8], dir: Direction) -> SubDBIterator {
        self.just_seeked = true;
        unsafe {
            rocksdb_ffi::rocksdb_iter_seek(self.inner, key.as_ptr(), key.len() as size_t);
        }
        SubDBIterator{ iter: self, direction: dir, }
    }
}

impl Drop for DBIterator {
    fn drop(&mut self) {
        unsafe {
            rocksdb_ffi::rocksdb_iter_destroy(self.inner);
        }
    }
}

impl <'a> Snapshot<'a> {
    pub fn new(db: &RocksDB) -> Snapshot {
        let snapshot = unsafe { rocksdb_ffi::rocksdb_create_snapshot(db.inner) };
        Snapshot{db: db, inner: snapshot}
    }

    pub fn iterator(&self) -> DBIterator {
        let mut readopts = ReadOptions::new();
        readopts.set_snapshot(self);
        DBIterator::new(self.db, &readopts)
    }
}

impl <'a> Drop for Snapshot<'a> {
    fn drop(&mut self) {
        unsafe {
            rocksdb_ffi::rocksdb_release_snapshot(self.db.inner, self.inner);
        }
    }
}

// This is for the RocksDB and write batches to share the same API
pub trait Writable {
    fn put(&mut self, key: &[u8], value: &[u8]) -> Result<(), String>;
    fn merge(&mut self, key: &[u8], value: &[u8]) -> Result<(), String>;
    fn delete(&mut self, key: &[u8]) -> Result<(), String>;
}

fn error_message(ptr: *const i8) -> String {
    let c_str = unsafe { CStr::from_ptr(ptr) };
//TODO I think we're leaking the c string here; should be a call to free once realloced into rust String
    from_utf8(c_str.to_bytes()).unwrap().to_owned()
}

impl RocksDB {
    pub fn open_default(path: &str) -> Result<RocksDB, String> {
        let mut opts = Options::new();
        opts.create_if_missing(true);
        RocksDB::open(&opts, path)
    }

    pub fn open(opts: &Options, path: &str) -> Result<RocksDB, String> {
        let cpath = match CString::new(path.as_bytes()) {
                        Ok(c) => c,
                        Err(_) => return Err("Failed to convert path to CString when opening rocksdb".to_string()),
        };
        let cpath_ptr = cpath.as_ptr();

        let ospath = Path::new(path);
        if !ospath.exists() {
            match fs::create_dir_all(&ospath) {
                Err(e) => return Err("Failed to create rocksdb directory.".to_string()),
                Ok(_) => (),
            }
        }

        let mut err: *const i8 = 0 as *const i8;
        let err_ptr: *mut *const i8 = &mut err;
        let db: rocksdb_ffi::RocksDBInstance;

        unsafe {
            db = rocksdb_ffi::rocksdb_open(opts.inner, cpath_ptr, err_ptr);
        }

        if !err.is_null() {
            return Err(error_message(err));
        }
        let rocksdb_ffi::RocksDBInstance(db_ptr) = db;
        if db_ptr.is_null() {
            return Err("Could not initialize database.".to_string());
        }
        Ok(RocksDB{inner: db})
    }

    pub fn destroy(opts: &Options, path: &str) -> Result<(), String> {
        let cpath = CString::new(path.as_bytes()).unwrap();
        let cpath_ptr = cpath.as_ptr();

        let ospath = Path::new(path);
        if !ospath.exists() {
            return Err("path does not exist".to_string());
        }

        let mut err: *const i8 = 0 as *const i8;
        let err_ptr: *mut *const i8 = &mut err;
        unsafe {
            rocksdb_ffi::rocksdb_destroy_db(opts.inner, cpath_ptr, err_ptr);
        }
        if !err.is_null() {
            return Err(error_message(err));
        }
        Ok(())
    }

    pub fn repair(opts: Options, path: &str) -> Result<(), String> {
        let cpath = CString::new(path.as_bytes()).unwrap();
        let cpath_ptr = cpath.as_ptr();

        let ospath = Path::new(path);
        if !ospath.exists() {
            return Err("path does not exist".to_string());
        }

        let mut err: *const i8 = 0 as *const i8;
        let err_ptr: *mut *const i8 = &mut err;
        unsafe {
            rocksdb_ffi::rocksdb_repair_db(opts.inner, cpath_ptr, err_ptr);
        }
        if !err.is_null() {
            return Err(error_message(err));
        }
        Ok(())
    }

    pub fn write(&self, batch: WriteBatch) -> Result<(), String> {
        let writeopts = unsafe { rocksdb_ffi::rocksdb_writeoptions_create() };
        let mut err: *const i8 = 0 as *const i8;
        let err_ptr: *mut *const i8 = &mut err;
        unsafe {
            rocksdb_ffi::rocksdb_write(self.inner, writeopts.clone(), batch.inner, err_ptr);
            rocksdb_ffi::rocksdb_writeoptions_destroy(writeopts);
        }
        if !err.is_null() {
            return Err(error_message(err));
        }
        return Ok(())
    }

    pub fn get(&self, key: &[u8]) -> RocksDBResult<RocksDBVector, String> {
        unsafe {
            let readopts = rocksdb_ffi::rocksdb_readoptions_create();
            if readopts.0.is_null() {
                return RocksDBResult::Error("Unable to create rocksdb read \
                    options.  This is a fairly trivial call, and its failure \
                    may be indicative of a mis-compiled or mis-loaded rocksdb \
                    library.".to_string());
            }

            let val_len: size_t = 0;
            let val_len_ptr = &val_len as *const size_t;
            let mut err: *const i8 = 0 as *const i8;
            let err_ptr: *mut *const i8 = &mut err;
            let val = rocksdb_ffi::rocksdb_get(self.inner, readopts.clone(),
                key.as_ptr(), key.len() as size_t, val_len_ptr, err_ptr) as *mut u8;
            rocksdb_ffi::rocksdb_readoptions_destroy(readopts);
            if !err.is_null() {
                return RocksDBResult::Error(error_message(err));
            }
            match val.is_null() {
                true => RocksDBResult::None,
                false => {
                    RocksDBResult::Some(RocksDBVector::from_c(val, val_len))
                }
            }
        }
    }

    pub fn iterator(&self) -> DBIterator {
        let opts = ReadOptions::new();
        DBIterator::new(&self, &opts)
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot::new(self)
    }
}

impl Writable for RocksDB {
    fn put(&mut self, key: &[u8], value: &[u8]) -> Result<(), String> {
        unsafe {
            let writeopts = rocksdb_ffi::rocksdb_writeoptions_create();
            let mut err: *const i8 = 0 as *const i8;
            let err_ptr: *mut *const i8 = &mut err;
            rocksdb_ffi::rocksdb_put(self.inner, writeopts.clone(), key.as_ptr(),
                        key.len() as size_t, value.as_ptr(),
                        value.len() as size_t, err_ptr);
            rocksdb_ffi::rocksdb_writeoptions_destroy(writeopts);
            if !err.is_null() {
                return Err(error_message(err));
            }
            return Ok(())
        }
    }

    fn merge(&mut self, key: &[u8], value: &[u8]) -> Result<(), String> {
        unsafe {
            let writeopts = rocksdb_ffi::rocksdb_writeoptions_create();
            let mut err: *const i8 = 0 as *const i8;
            let err_ptr: *mut *const i8 = &mut err;
            rocksdb_ffi::rocksdb_merge(self.inner, writeopts.clone(), key.as_ptr(),
                        key.len() as size_t, value.as_ptr(),
                        value.len() as size_t, err_ptr);
            rocksdb_ffi::rocksdb_writeoptions_destroy(writeopts);
            if !err.is_null() {
                return Err(error_message(err));
            }
            return Ok(())
        }
    }

    fn delete(&mut self, key: &[u8]) -> Result<(), String> {
        unsafe {
            let writeopts = rocksdb_ffi::rocksdb_writeoptions_create();
            let mut err: *const i8 = 0 as *const i8;
            let err_ptr: *mut *const i8 = &mut err;
            rocksdb_ffi::rocksdb_delete(self.inner, writeopts.clone(), key.as_ptr(),
                        key.len() as size_t, err_ptr);
            rocksdb_ffi::rocksdb_writeoptions_destroy(writeopts);
            if !err.is_null() {
                return Err(error_message(err));
            }
            return Ok(())
        }
    }
}

impl WriteBatch {
    pub fn new() -> WriteBatch {
        WriteBatch {
            inner: unsafe {
                       rocksdb_ffi::rocksdb_writebatch_create()
                   }
        }
    }
}

impl Drop for WriteBatch {
    fn drop(&mut self) {
        unsafe {
            rocksdb_ffi::rocksdb_writebatch_destroy(self.inner)
        }
    }
}

impl Drop for RocksDB {
    fn drop(&mut self) {
        unsafe { rocksdb_ffi::rocksdb_close(self.inner); }
    }
}

impl Writable for WriteBatch {
    fn put(&mut self, key: &[u8], value: &[u8]) -> Result<(), String> {
        unsafe {
            rocksdb_ffi::rocksdb_writebatch_put(self.inner, key.as_ptr(),
                        key.len() as size_t, value.as_ptr(),
                        value.len() as size_t);
            return Ok(())
        }
    }

    fn merge(&mut self, key: &[u8], value: &[u8]) -> Result<(), String> {
        unsafe {
            rocksdb_ffi::rocksdb_writebatch_merge(self.inner, key.as_ptr(),
                        key.len() as size_t, value.as_ptr(),
                        value.len() as size_t);
            return Ok(())
        }
    }

    fn delete(&mut self, key: &[u8]) -> Result<(), String> {
        unsafe {
            rocksdb_ffi::rocksdb_writebatch_delete(self.inner, key.as_ptr(),
                        key.len() as size_t);
            return Ok(())
        }
    }
}

impl Drop for ReadOptions {
    fn drop(&mut self) {
        unsafe {
            rocksdb_ffi::rocksdb_readoptions_destroy(self.inner)
        }
    }
}

impl ReadOptions {
    fn new() -> ReadOptions {
        unsafe {
            ReadOptions{inner: rocksdb_ffi::rocksdb_readoptions_create()}
        }
    }
//TODO add snapshot setting here
//TODO add snapshot wrapper structs with proper destructors; that struct needs an "iterator" impl too.
    fn fill_cache(&mut self, v: bool) {
        unsafe {
            rocksdb_ffi::rocksdb_readoptions_set_fill_cache(self.inner, v);
        }
    }

    fn set_snapshot(&mut self, snapshot: &Snapshot) {
        unsafe {
            rocksdb_ffi::rocksdb_readoptions_set_snapshot(self.inner, snapshot.inner);
        }
    }
}

pub struct RocksDBVector {
    base: Unique<u8>,
    len: usize,
}

impl Deref for RocksDBVector {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.base.get(), self.len) }
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
            RocksDBVector {
                base: Unique::new(val),
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
    {
        let mut db = RocksDB::open_default(path).unwrap();
        let p = db.put(b"k1", b"v1111");
        assert!(p.is_ok());
        let r: RocksDBResult<RocksDBVector, String> = db.get(b"k1");
        assert!(r.unwrap().to_utf8().unwrap() == "v1111");
        assert!(db.delete(b"k1").is_ok());
        assert!(db.get(b"k1").is_none());
    }
    let opts = Options::new();
    let result = RocksDB::destroy(&opts, path);
    assert!(result.is_ok());
}

#[test]
fn errors_do_stuff() {
    let path = "_rust_rocksdb_error";
    let mut db = RocksDB::open_default(path).unwrap();
    let opts = Options::new();
    // The DB will still be open when we try to destroy and the lock should fail
    match RocksDB::destroy(&opts, path) {
        Err(ref s) => assert!(s == "IO error: lock _rust_rocksdb_error/LOCK: No locks available"),
        Ok(_) => panic!("should fail")
    }
}

#[test]
fn writebatch_works() {
    let path = "_rust_rocksdb_writebacktest";
    {
        let mut db = RocksDB::open_default(path).unwrap();
        { // test put
            let mut batch = WriteBatch::new();
            assert!(db.get(b"k1").is_none());
            batch.put(b"k1", b"v1111");
            assert!(db.get(b"k1").is_none());
            let p = db.write(batch);
            assert!(p.is_ok());
            let r: RocksDBResult<RocksDBVector, String> = db.get(b"k1");
            assert!(r.unwrap().to_utf8().unwrap() == "v1111");
        }
        { // test delete
            let mut batch = WriteBatch::new();
            batch.delete(b"k1");
            let p = db.write(batch);
            assert!(p.is_ok());
            assert!(db.get(b"k1").is_none());
        }
    }
    let opts = Options::new();
    assert!(RocksDB::destroy(&opts, path).is_ok());
}

#[test]
fn iterator_test() {
    let path = "_rust_rocksdb_iteratortest";
    {
        let mut db = RocksDB::open_default(path).unwrap();
        let p = db.put(b"k1", b"v1111");
        assert!(p.is_ok());
        let p = db.put(b"k2", b"v2222");
        assert!(p.is_ok());
        let p = db.put(b"k3", b"v3333");
        assert!(p.is_ok());
        let mut iter = db.iterator();
        for (k,v) in iter.from_start() {
            println!("Hello {}: {}", from_utf8(k).unwrap(), from_utf8(v).unwrap());
        }
    }
    let opts = Options::new();
    assert!(RocksDB::destroy(&opts, path).is_ok());
}
