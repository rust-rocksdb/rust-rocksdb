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

use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::ptr;
use std::slice;
use std::str::from_utf8;
use std::fmt;

use libc::{self, c_char, c_uchar, c_int, c_void, size_t};

use {Options, WriteOptions};
use ffi;

fn error_message(ptr: *const i8) -> String {
    let c_str = unsafe { CStr::from_ptr(ptr as *const _) };
    let s = from_utf8(c_str.to_bytes()).unwrap().to_owned();
    unsafe {
        libc::free(ptr as *mut c_void);
    }
    s
}

pub fn new_bloom_filter(bits: c_int) -> *mut ffi::rocksdb_filterpolicy_t {
    unsafe { ffi::rocksdb_filterpolicy_create_bloom(bits) }
}

pub fn new_cache(capacity: size_t) -> *mut ffi::rocksdb_cache_t {
    unsafe { ffi::rocksdb_cache_create_lru(capacity) }
}

pub struct DB {
    inner: *mut ffi::rocksdb_t,
    cfs: BTreeMap<String, *mut ffi::rocksdb_column_family_handle_t>,
    path: PathBuf,
}

unsafe impl Send for DB { }
unsafe impl Sync for DB { }

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DBCompressionType {
    None = 0,
    Snappy = 1,
    Zlib = 2,
    Bz2 = 3,
    Lz4 = 4,
    Lz4hc = 5,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DBCompactionStyle {
    Level = 0,
    Universal = 1,
    Fifo = 2,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DBRecoveryMode {
    TolerateCorruptedTailRecords = 0,
    AbsoluteConsistency = 1,
    PointInTime = 2,
    SkipAnyCorruptedRecords = 3,
}

pub struct WriteBatch {
    inner: *mut ffi::rocksdb_writebatch_t,
}

pub struct ReadOptions {
    inner: *mut ffi::rocksdb_readoptions_t,
}

pub struct Snapshot<'a> {
    db: &'a DB,
    inner: *const ffi::rocksdb_snapshot_t,
}

pub struct DBIterator {
    inner: *mut ffi::rocksdb_iterator_t,
    direction: Direction,
    just_seeked: bool,
}

pub enum Direction {
    Forward,
    Reverse,
}

pub type KVBytes = (Box<[u8]>, Box<[u8]>);

#[derive(Debug, PartialEq)]
pub struct Error {
    message: String,
}

impl Error {
    fn new(message: String) -> Error {
        Error { message: message }
    }

    pub fn to_string(self) -> String {
        self.into()
    }
}

impl From<Error> for String {
    fn from(e: Error) -> String {
        e.message
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.message.fmt(formatter)
    }
}

impl Iterator for DBIterator {
    type Item = KVBytes;

    fn next(&mut self) -> Option<KVBytes> {
        let native_iter = self.inner;
        if !self.just_seeked {
            match self.direction {
                Direction::Forward => unsafe { ffi::rocksdb_iter_next(native_iter) },
                Direction::Reverse => unsafe { ffi::rocksdb_iter_prev(native_iter) },
            }
        } else {
            self.just_seeked = false;
        }
        if unsafe { ffi::rocksdb_iter_valid(native_iter) != 0 } {
            let mut key_len: size_t = 0;
            let key_len_ptr: *mut size_t = &mut key_len;
            let mut val_len: size_t = 0;
            let val_len_ptr: *mut size_t = &mut val_len;
            let key_ptr = unsafe { ffi::rocksdb_iter_key(native_iter, key_len_ptr) as *const c_uchar };
            let key = unsafe { slice::from_raw_parts(key_ptr, key_len as usize) };
            let val_ptr = unsafe { ffi::rocksdb_iter_value(native_iter, val_len_ptr) as *const c_uchar };
            let val = unsafe { slice::from_raw_parts(val_ptr, val_len as usize) };

            Some((key.to_vec().into_boxed_slice(), val.to_vec().into_boxed_slice()))
        } else {
            None
        }
    }
}

pub enum IteratorMode<'a> {
    Start,
    End,
    From(&'a [u8], Direction),
}

impl DBIterator {
    fn new(db: &DB, readopts: &ReadOptions, mode: IteratorMode) -> DBIterator {
        unsafe {
            let iterator = ffi::rocksdb_create_iterator(db.inner, readopts.inner);

            let mut rv = DBIterator {
                inner: iterator,
                direction: Direction::Forward, // blown away by set_mode()
                just_seeked: false,
            };
            rv.set_mode(mode);
            rv
        }
    }

    pub fn set_mode(&mut self, mode: IteratorMode) {
        unsafe {
            match mode {
                IteratorMode::Start => {
                    ffi::rocksdb_iter_seek_to_first(self.inner);
                    self.direction = Direction::Forward;
                }
                IteratorMode::End => {
                    ffi::rocksdb_iter_seek_to_last(self.inner);
                    self.direction = Direction::Reverse;
                }
                IteratorMode::From(key, dir) => {
                    ffi::rocksdb_iter_seek(self.inner, key.as_ptr() as *const c_char, key.len() as size_t);
                    self.direction = dir;
                }
            };
            self.just_seeked = true;
        }
    }

    pub fn valid(&self) -> bool {
        unsafe { ffi::rocksdb_iter_valid(self.inner) != 0 }
    }

    fn new_cf(db: &DB, cf_handle: *mut ffi::rocksdb_column_family_handle_t, readopts: &ReadOptions, mode: IteratorMode) -> Result<DBIterator, Error> {
        unsafe {
            let iterator = ffi::rocksdb_create_iterator_cf(db.inner, readopts.inner, cf_handle);

            let mut rv = DBIterator {
                inner: iterator,
                direction: Direction::Forward, // blown away by set_mode()
                just_seeked: false,
            };
            rv.set_mode(mode);
            Ok(rv)
        }
    }
}

impl Drop for DBIterator {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_iter_destroy(self.inner);
        }
    }
}

impl<'a> Snapshot<'a> {
    pub fn new(db: &DB) -> Snapshot {
        let snapshot = unsafe { ffi::rocksdb_create_snapshot(db.inner) };
        Snapshot {
            db: db,
            inner: snapshot,
        }
    }

    pub fn iterator(&self, mode: IteratorMode) -> DBIterator {
        let mut readopts = ReadOptions::default();
        readopts.set_snapshot(self);
        DBIterator::new(self.db, &readopts, mode)
    }

    pub fn iterator_cf(&self, cf_handle: *mut ffi::rocksdb_column_family_handle_t, mode: IteratorMode) -> Result<DBIterator, Error> {
        let mut readopts = ReadOptions::default();
        readopts.set_snapshot(self);
        DBIterator::new_cf(self.db, cf_handle, &readopts, mode)
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<DBVector>, Error> {
        let mut readopts = ReadOptions::default();
        readopts.set_snapshot(self);
        self.db.get_opt(key, &readopts)
    }

    pub fn get_cf(&self, cf: *mut ffi::rocksdb_column_family_handle_t, key: &[u8]) -> Result<Option<DBVector>, Error> {
        let mut readopts = ReadOptions::default();
        readopts.set_snapshot(self);
        self.db.get_cf_opt(cf, key, &readopts)
    }
}

impl<'a> Drop for Snapshot<'a> {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_release_snapshot(self.db.inner, self.inner);
        }
    }
}

// This is for the DB and write batches to share the same API.
pub trait Writable {
    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), Error>;
    fn put_cf(&self, cf: *mut ffi::rocksdb_column_family_handle_t, key: &[u8], value: &[u8]) -> Result<(), Error>;
    fn merge(&self, key: &[u8], value: &[u8]) -> Result<(), Error>;
    fn merge_cf(&self, cf: *mut ffi::rocksdb_column_family_handle_t, key: &[u8], value: &[u8]) -> Result<(), Error>;
    fn delete(&self, key: &[u8]) -> Result<(), Error>;
    fn delete_cf(&self, cf: *mut ffi::rocksdb_column_family_handle_t, key: &[u8]) -> Result<(), Error>;
}

impl DB {
    /// Open a database with default options.
    pub fn open_default<P: AsRef<Path>>(path: P) -> Result<DB, Error> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        DB::open(&opts, path)
    }

    /// Open the database with specified options
    pub fn open<P: AsRef<Path>>(opts: &Options, path: P) -> Result<DB, Error> {
        DB::open_cf(opts, path, &[])
    }

    /// Open a database with specified options and column family
    ///
    /// A column family must be created first by calling `DB::create_cf`
    ///
    /// # Panics
    ///
    /// * Panics if the column family doesn't exist
    pub fn open_cf<P: AsRef<Path>>(opts: &Options, path: P, cfs: &[&str]) -> Result<DB, Error> {
        let path = path.as_ref();

        let cpath = match CString::new(path.to_string_lossy().as_bytes()) {
            Ok(c) => c,
            Err(_) => {
                return Err(Error::new("Failed to convert path to CString when opening rocksdb".to_owned()))
            }
        };
        let cpath_ptr = cpath.as_ptr();

        if let Err(e) = fs::create_dir_all(&path) {
            return Err(Error::new(format!("Failed to create rocksdb directory: {:?}", e)));
        }

        let mut err: *mut c_char = ptr::null_mut();
        let db: *mut ffi::rocksdb_t;
        let mut cf_map = BTreeMap::new();

        if cfs.len() == 0 {
            unsafe {
                db = ffi::rocksdb_open(opts.inner, cpath_ptr as *const _, &mut err);
            }
        } else {
            let mut cfs_v = cfs.to_vec();
            // Always open the default column family.
            if !cfs_v.contains(&"default") {
                cfs_v.push("default");
            }

            // We need to store our CStrings in an intermediate vector so that their pointers remain valid.
            let c_cfs: Vec<CString> = cfs_v.iter().map(|cf| CString::new(cf.as_bytes()).unwrap()).collect();

            let cfnames: Vec<_> = c_cfs.iter().map(|cf| cf.as_ptr()).collect();

            // These handles will be populated by DB.
            let mut cfhandles: Vec<_> = cfs_v.iter().map(|_| ptr::null_mut()).collect();

            // TODO(tyler) allow options to be passed in.
            let cfopts: Vec<_> = cfs_v.iter().map(|_| unsafe { ffi::rocksdb_options_create() as *const _ }).collect();

            unsafe {
                db = ffi::rocksdb_open_column_families(opts.inner, cpath_ptr as *const _, cfs_v.len() as c_int, cfnames.as_ptr() as *const _, cfopts.as_ptr(), cfhandles.as_mut_ptr(), &mut err);
            }

            for handle in &cfhandles {
                if handle.is_null() {
                    return Err(Error::new("Received null column family handle from DB.".to_owned()));
                }
            }

            for (n, h) in cfs_v.iter().zip(cfhandles) {
                cf_map.insert(n.to_string(), h);
            }
        }

        if !err.is_null() {
            return Err(Error::new(error_message(err)));
        }
        if db.is_null() {
            return Err(Error::new("Could not initialize database.".to_owned()));
        }

        Ok(DB {
            inner: db,
            cfs: cf_map,
            path: path.to_path_buf(),
        })
    }

    pub fn destroy<P: AsRef<Path>>(opts: &Options, path: P) -> Result<(), Error> {
        let cpath = CString::new(path.as_ref().to_string_lossy().as_bytes()).unwrap();
        let mut err: *mut c_char = ptr::null_mut();
        unsafe {
            ffi::rocksdb_destroy_db(opts.inner, cpath.as_ptr(), &mut err);
        }
        if !err.is_null() {
            return Err(Error::new(error_message(err)));
        }
        Ok(())
    }

    pub fn repair<P: AsRef<Path>>(opts: Options, path: P) -> Result<(), Error> {
        let cpath = CString::new(path.as_ref().to_string_lossy().as_bytes()).unwrap();
        let mut err: *mut c_char = ptr::null_mut();
        unsafe {
            ffi::rocksdb_repair_db(opts.inner, cpath.as_ptr(), &mut err);
        }
        if !err.is_null() {
            return Err(Error::new(error_message(err)));
        }
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path.as_path()
    }

    pub fn write_opt(&self, batch: &WriteBatch, writeopts: &WriteOptions) -> Result<(), Error> {
        let mut err: *mut c_char = ptr::null_mut();
        unsafe {
            ffi::rocksdb_write(self.inner, writeopts.inner, batch.inner, &mut err);
        }
        if !err.is_null() {
            return Err(Error::new(error_message(err)));
        }
        Ok(())
    }

    pub fn write(&self, batch: &WriteBatch) -> Result<(), Error> {
        self.write_opt(batch, &WriteOptions::default())
    }

    pub fn write_without_wal(&self, batch: &WriteBatch) -> Result<(), Error> {
        let mut wo = WriteOptions::new();
        wo.disable_wal(true);
        self.write_opt(batch, &wo)
    }

    pub fn get_opt(&self, key: &[u8], readopts: &ReadOptions) -> Result<Option<DBVector>, Error> {
        if readopts.inner.is_null() {
            return Err(Error::new("Unable to create rocksdb read options. This is a fairly trivial call, and its failure may be indicative of a mis-compiled or mis-loaded rocksdb library.".to_owned()));
        }

        unsafe {
            let mut val_len: size_t = 0;
            let mut err: *mut c_char = ptr::null_mut();
            let val = ffi::rocksdb_get(self.inner, readopts.inner, key.as_ptr() as *const c_char, key.len() as size_t, &mut val_len, &mut err) as *mut u8;
            if !err.is_null() {
                return Err(Error::new(error_message(err)));
            }
            if val.is_null() {
                Ok(None)
            } else {
                Ok(Some(DBVector::from_c(val, val_len)))
            }
        }
    }

    /// Return the bytes associated with a key value
    pub fn get(&self, key: &[u8]) -> Result<Option<DBVector>, Error> {
        self.get_opt(key, &ReadOptions::default())
    }

    pub fn get_cf_opt(&self, cf: *mut ffi::rocksdb_column_family_handle_t, key: &[u8], readopts: &ReadOptions) -> Result<Option<DBVector>, Error> {
        if readopts.inner.is_null() {
            return Err(Error::new("Unable to create rocksdb read options. This is a fairly trivial call, and its failure may be indicative of a mis-compiled or mis-loaded rocksdb library.".to_owned()));
        }

        unsafe {
            let mut val_len: size_t = 0;
            let mut err: *mut c_char = ptr::null_mut();
            let val = ffi::rocksdb_get_cf(self.inner, readopts.inner, cf, key.as_ptr() as *const c_char, key.len() as size_t, &mut val_len, &mut err) as *mut u8;
            if !err.is_null() {
                return Err(Error::new(error_message(err)));
            }
            if val.is_null() {
                Ok(None)
            } else {
                Ok(Some(DBVector::from_c(val, val_len)))
            }
        }
    }

    pub fn get_cf(&self, cf: *mut ffi::rocksdb_column_family_handle_t, key: &[u8]) -> Result<Option<DBVector>, Error> {
        self.get_cf_opt(cf, key, &ReadOptions::default())
    }

    pub fn create_cf(&mut self, name: &str, opts: &Options) -> Result<*mut ffi::rocksdb_column_family_handle_t, Error> {
        let cname = match CString::new(name.as_bytes()) {
            Ok(c) => c,
            Err(_) => {
                return Err(Error::new("Failed to convert path to CString when opening rocksdb".to_owned()))
            }
        };
        let mut err: *mut c_char = ptr::null_mut();
        let cf_handler = unsafe {
            let cf_handler = ffi::rocksdb_create_column_family(self.inner, opts.inner, cname.as_ptr(), &mut err);
            self.cfs.insert(name.to_string(), cf_handler);
            cf_handler
        };
        if !err.is_null() {
            return Err(Error::new(error_message(err)));
        }
        Ok(cf_handler)
    }

    pub fn drop_cf(&mut self, name: &str) -> Result<(), Error> {
        let cf = self.cfs.get(name);
        if cf.is_none() {
            return Err(Error::new(format!("Invalid column family: {}", name).to_owned()));
        }
        let mut err: *mut c_char = ptr::null_mut();
        unsafe {
            ffi::rocksdb_drop_column_family(self.inner, *cf.unwrap(), &mut err);
        }
        if !err.is_null() {
            return Err(Error::new(error_message(err)));
        }
        Ok(())
    }

    /// Return the underlying column family handle.
    pub fn cf_handle(&self, name: &str) -> Option<&*mut ffi::rocksdb_column_family_handle_t> {
        self.cfs.get(name)
    }

    pub fn iterator(&self, mode: IteratorMode) -> DBIterator {
        let opts = ReadOptions::default();
        DBIterator::new(self, &opts, mode)
    }

    pub fn iterator_cf(&self, cf_handle: *mut ffi::rocksdb_column_family_handle_t, mode: IteratorMode) -> Result<DBIterator, Error> {
        let opts = ReadOptions::default();
        DBIterator::new_cf(self, cf_handle, &opts, mode)
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot::new(self)
    }

    pub fn put_opt(&self, key: &[u8], value: &[u8], writeopts: &WriteOptions) -> Result<(), Error> {
        unsafe {
            let mut err: *mut c_char = ptr::null_mut();
            ffi::rocksdb_put(self.inner, writeopts.inner, key.as_ptr() as *const c_char, key.len() as size_t, value.as_ptr() as *const c_char, value.len() as size_t, &mut err);
            if !err.is_null() {
                return Err(Error::new(error_message(err)));
            }
            Ok(())
        }
    }

    pub fn put_cf_opt(&self, cf: *mut ffi::rocksdb_column_family_handle_t, key: &[u8], value: &[u8], writeopts: &WriteOptions) -> Result<(), Error> {
        unsafe {
            let mut err: *mut c_char = ptr::null_mut();
            ffi::rocksdb_put_cf(self.inner, writeopts.inner, cf, key.as_ptr() as *const c_char, key.len() as size_t, value.as_ptr() as *const c_char, value.len() as size_t, &mut err);
            if !err.is_null() {
                return Err(Error::new(error_message(err)));
            }
            Ok(())
        }
    }

    pub fn merge_opt(&self, key: &[u8], value: &[u8], writeopts: &WriteOptions) -> Result<(), Error> {
        unsafe {
            let mut err: *mut c_char = ptr::null_mut();
            ffi::rocksdb_merge(self.inner, writeopts.inner, key.as_ptr() as *const c_char, key.len() as size_t, value.as_ptr() as *const c_char, value.len() as size_t, &mut err);
            if !err.is_null() {
                return Err(Error::new(error_message(err)));
            }
            Ok(())
        }
    }

    fn merge_cf_opt(&self, cf: *mut ffi::rocksdb_column_family_handle_t, key: &[u8], value: &[u8], writeopts: &WriteOptions) -> Result<(), Error> {
        unsafe {
            let mut err: *mut c_char = ptr::null_mut();
            ffi::rocksdb_merge_cf(self.inner, writeopts.inner, cf, key.as_ptr() as *const i8, key.len() as size_t, value.as_ptr() as *const i8, value.len() as size_t, &mut err);
            if !err.is_null() {
                return Err(Error::new(error_message(err)));
            }
            Ok(())
        }
    }

    fn delete_opt(&self, key: &[u8], writeopts: &WriteOptions) -> Result<(), Error> {
        unsafe {
            let mut err: *mut c_char = ptr::null_mut();
            ffi::rocksdb_delete(self.inner, writeopts.inner, key.as_ptr() as *const c_char, key.len() as size_t, &mut err);
            if !err.is_null() {
                return Err(Error::new(error_message(err)));
            }
            Ok(())
        }
    }

    fn delete_cf_opt(&self, cf: *mut ffi::rocksdb_column_family_handle_t, key: &[u8], writeopts: &WriteOptions) -> Result<(), Error> {
        unsafe {
            let mut err: *mut c_char = ptr::null_mut();
            ffi::rocksdb_delete_cf(self.inner, writeopts.inner, cf, key.as_ptr() as *const c_char, key.len() as size_t, &mut err);
            if !err.is_null() {
                return Err(Error::new(error_message(err)));
            }
            Ok(())
        }
    }
}

impl Writable for DB {
    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        self.put_opt(key, value, &WriteOptions::default())
    }

    fn put_cf(&self, cf: *mut ffi::rocksdb_column_family_handle_t, key: &[u8], value: &[u8]) -> Result<(), Error> {
        self.put_cf_opt(cf, key, value, &WriteOptions::default())
    }

    fn merge(&self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        self.merge_opt(key, value, &WriteOptions::default())
    }

    fn merge_cf(&self, cf: *mut ffi::rocksdb_column_family_handle_t, key: &[u8], value: &[u8]) -> Result<(), Error> {
        self.merge_cf_opt(cf, key, value, &WriteOptions::default())
    }

    fn delete(&self, key: &[u8]) -> Result<(), Error> {
        self.delete_opt(key, &WriteOptions::default())
    }

    fn delete_cf(&self, cf: *mut ffi::rocksdb_column_family_handle_t, key: &[u8]) -> Result<(), Error> {
        self.delete_cf_opt(cf, key, &WriteOptions::default())
    }
}

impl WriteBatch {
    pub fn len(&self) -> usize {
        unsafe { ffi::rocksdb_writebatch_count(self.inner) as usize }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&self) {
        unsafe { ffi::rocksdb_writebatch_clear(self.inner) }
    }
}

impl Default for WriteBatch {
    fn default() -> WriteBatch {
        WriteBatch {
            inner: unsafe { ffi::rocksdb_writebatch_create() },
        }
    }
}

impl Drop for WriteBatch {
    fn drop(&mut self) {
        unsafe { ffi::rocksdb_writebatch_destroy(self.inner) }
    }
}

impl Drop for DB {
    fn drop(&mut self) {
        unsafe {
            for cf in self.cfs.values() {
                ffi::rocksdb_column_family_handle_destroy(*cf);
            }
            ffi::rocksdb_close(self.inner);
        }
    }
}

impl fmt::Debug for DB {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RocksDB {{ path: {:?} }}", self.path())
    }
}

impl Writable for WriteBatch {
    /// Insert a value into the database under the given key.
    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        unsafe {
            ffi::rocksdb_writebatch_put(self.inner, key.as_ptr() as *const i8, key.len() as size_t, value.as_ptr() as *const i8, value.len() as size_t);
            Ok(())
        }
    }

    fn put_cf(&self, cf: *mut ffi::rocksdb_column_family_handle_t, key: &[u8], value: &[u8]) -> Result<(), Error> {
        unsafe {
            ffi::rocksdb_writebatch_put_cf(self.inner, cf, key.as_ptr() as *const i8, key.len() as size_t, value.as_ptr() as *const i8, value.len() as size_t);
            Ok(())
        }
    }

    fn merge(&self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        unsafe {
            ffi::rocksdb_writebatch_merge(self.inner, key.as_ptr() as *const i8, key.len() as size_t, value.as_ptr() as *const i8, value.len() as size_t);
            Ok(())
        }
    }

    fn merge_cf(&self, cf: *mut ffi::rocksdb_column_family_handle_t, key: &[u8], value: &[u8]) -> Result<(), Error> {
        unsafe {
            ffi::rocksdb_writebatch_merge_cf(self.inner, cf, key.as_ptr() as *const i8, key.len() as size_t, value.as_ptr() as *const i8, value.len() as size_t);
            Ok(())
        }
    }

    /// Remove the database entry for key.
    ///
    /// Returns Err if the key was not found
    fn delete(&self, key: &[u8]) -> Result<(), Error> {
        unsafe {
            ffi::rocksdb_writebatch_delete(self.inner, key.as_ptr() as *const i8, key.len() as size_t);
            Ok(())
        }
    }

    fn delete_cf(&self, cf: *mut ffi::rocksdb_column_family_handle_t, key: &[u8]) -> Result<(), Error> {
        unsafe {
            ffi::rocksdb_writebatch_delete_cf(self.inner, cf, key.as_ptr() as *const i8, key.len() as size_t);
            Ok(())
        }
    }
}

impl Drop for ReadOptions {
    fn drop(&mut self) {
        unsafe { ffi::rocksdb_readoptions_destroy(self.inner) }
    }
}

impl ReadOptions {
    // TODO add snapshot setting here
    // TODO add snapshot wrapper structs with proper destructors;
    // that struct needs an "iterator" impl too.
    #[allow(dead_code)]
    fn fill_cache(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_readoptions_set_fill_cache(self.inner, v as c_uchar);
        }
    }

    fn set_snapshot(&mut self, snapshot: &Snapshot) {
        unsafe {
            ffi::rocksdb_readoptions_set_snapshot(self.inner, snapshot.inner);
        }
    }

    pub fn set_iterate_upper_bound(&mut self, key: &[u8]) {
        unsafe {
            ffi::rocksdb_readoptions_set_iterate_upper_bound(self.inner, key.as_ptr() as *const i8, key.len() as size_t);
        }
    }
}

impl Default for ReadOptions {
    fn default() -> ReadOptions {
        unsafe {
            ReadOptions { inner: ffi::rocksdb_readoptions_create() }
        }
    }
}

/// Wrapper around bytes stored in the database
pub struct DBVector {
    base: *mut u8,
    len: usize,
}

impl Deref for DBVector {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.base, self.len) }
    }
}

impl Drop for DBVector {
    fn drop(&mut self) {
        unsafe {
            libc::free(self.base as *mut c_void);
        }
    }
}

impl DBVector {
    pub fn from_c(val: *mut u8, val_len: size_t) -> DBVector {
        DBVector {
            base: val,
            len: val_len as usize,
        }
    }

    pub fn to_utf8(&self) -> Option<&str> {
        from_utf8(self.deref()).ok()
    }
}

#[test]
fn external() {
    let path = "_rust_rocksdb_externaltest";
    {
        let db = DB::open_default(path).unwrap();
        let p = db.put(b"k1", b"v1111");
        assert!(p.is_ok());
        let r: Result<Option<DBVector>, Error> = db.get(b"k1");
        assert!(r.unwrap().unwrap().to_utf8().unwrap() == "v1111");
        assert!(db.delete(b"k1").is_ok());
        assert!(db.get(b"k1").unwrap().is_none());
    }
    let opts = Options::default();
    let result = DB::destroy(&opts, path);
    assert!(result.is_ok());
}

#[test]
fn errors_do_stuff() {
    let path = "_rust_rocksdb_error";
    let db = DB::open_default(path).unwrap();
    let opts = Options::default();
    // The DB will still be open when we try to destroy and the lock should fail
    match DB::destroy(&opts, path) {
        Err(s) => {
            assert!(s == Error::new("IO error: lock _rust_rocksdb_error/LOCK: No locks available".to_owned()))
        }
        Ok(_) => panic!("should fail"),
    }
}

#[test]
fn writebatch_works() {
    let path = "_rust_rocksdb_writebacktest";
    {
        let db = DB::open_default(path).unwrap();
        {
            // test put
            let batch = WriteBatch::default();
            assert!(db.get(b"k1").unwrap().is_none());
            assert_eq!(batch.len(), 0);
            assert!(batch.is_empty());
            let _ = batch.put(b"k1", b"v1111");
            assert_eq!(batch.len(), 1);
            assert!(!batch.is_empty());
            assert!(db.get(b"k1").unwrap().is_none());
            let p = db.write(batch);
            assert!(p.is_ok());
            let r: Result<Option<DBVector>, Error> = db.get(b"k1");
            assert!(r.unwrap().unwrap().to_utf8().unwrap() == "v1111");
        }
        {
            // test delete
            let batch = WriteBatch::default();
            let _ = batch.delete(b"k1");
            assert_eq!(batch.len(), 1);
            assert!(!batch.is_empty());
            let p = db.write(batch);
            assert!(p.is_ok());
            assert!(db.get(b"k1").unwrap().is_none());
        }
    }
    let opts = Options::default();
    assert!(DB::destroy(&opts, path).is_ok());
}

#[test]
fn iterator_test() {
    let path = "_rust_rocksdb_iteratortest";
    {
        let db = DB::open_default(path).unwrap();
        let p = db.put(b"k1", b"v1111");
        assert!(p.is_ok());
        let p = db.put(b"k2", b"v2222");
        assert!(p.is_ok());
        let p = db.put(b"k3", b"v3333");
        assert!(p.is_ok());
        let iter = db.iterator(IteratorMode::Start);
        for (k, v) in iter {
            println!("Hello {}: {}", from_utf8(&*k).unwrap(), from_utf8(&*v).unwrap());
        }
    }
    let opts = Options::default();
    assert!(DB::destroy(&opts, path).is_ok());
}

#[test]
fn snapshot_test() {
    let path = "_rust_rocksdb_snapshottest";
    {
        let db = DB::open_default(path).unwrap();
        let p = db.put(b"k1", b"v1111");
        assert!(p.is_ok());

        let snap = db.snapshot();
        let r: Result<Option<DBVector>, Error> = snap.get(b"k1");
        assert!(r.unwrap().unwrap().to_utf8().unwrap() == "v1111");

        let p = db.put(b"k2", b"v2222");
        assert!(p.is_ok());

        assert!(db.get(b"k2").unwrap().is_some());
        assert!(snap.get(b"k2").unwrap().is_none());
    }
    let opts = Options::default();
    assert!(DB::destroy(&opts, path).is_ok());
}
