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

extern crate libc;
use std::mem;
use std::collections::BTreeMap;
use std::ffi::CString;
use std::fs;
use std::ops::Deref;
use std::path::Path;
use std::slice;
use std::str::from_utf8;
use std::fmt;

use self::libc::size_t;

use rocksdb_ffi::{self, DBCFHandle, error_message};
use rocksdb_options::{Options, WriteOptions};

pub struct DB {
    inner: rocksdb_ffi::DBInstance,
    cfs: BTreeMap<String, DBCFHandle>,
    path: String,
}

unsafe impl Send for DB {}
unsafe impl Sync for DB {}

pub struct WriteBatch {
    inner: rocksdb_ffi::DBWriteBatch,
}

pub struct ReadOptions {
    inner: rocksdb_ffi::DBReadOptions,
}

pub struct Snapshot<'a> {
    db: &'a DB,
    inner: rocksdb_ffi::DBSnapshot,
}

pub struct DBIterator {
    inner: rocksdb_ffi::DBIterator,
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
                Direction::Forward => unsafe {
                    rocksdb_ffi::rocksdb_iter_next(native_iter)
                },
                Direction::Reverse => unsafe {
                    rocksdb_ffi::rocksdb_iter_prev(native_iter)
                },
            }
        } else {
            self.just_seeked = false;
        }
        if unsafe { rocksdb_ffi::rocksdb_iter_valid(native_iter) } {
            let mut key_len: size_t = 0;
            let key_len_ptr: *mut size_t = &mut key_len;
            let mut val_len: size_t = 0;
            let val_len_ptr: *mut size_t = &mut val_len;
            let key_ptr = unsafe {
                rocksdb_ffi::rocksdb_iter_key(native_iter, key_len_ptr)
            };
            let key =
                unsafe { slice::from_raw_parts(key_ptr, key_len as usize) };
            let val_ptr = unsafe {
                rocksdb_ffi::rocksdb_iter_value(native_iter, val_len_ptr)
            };
            let val =
                unsafe { slice::from_raw_parts(val_ptr, val_len as usize) };

            Some((key.to_vec().into_boxed_slice(),
                  val.to_vec().into_boxed_slice()))
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
            let iterator = rocksdb_ffi::rocksdb_create_iterator(db.inner,
                                                                readopts.inner);

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
                    rocksdb_ffi::rocksdb_iter_seek_to_first(self.inner);
                    self.direction = Direction::Forward;
                }
                IteratorMode::End => {
                    rocksdb_ffi::rocksdb_iter_seek_to_last(self.inner);
                    self.direction = Direction::Reverse;
                }
                IteratorMode::From(key, dir) => {
                    rocksdb_ffi::rocksdb_iter_seek(self.inner,
                                                   key.as_ptr(),
                                                   key.len() as size_t);
                    self.direction = dir;
                }
            };
            self.just_seeked = true;
        }
    }

    pub fn valid(&self) -> bool {
        unsafe { rocksdb_ffi::rocksdb_iter_valid(self.inner) }
    }

    fn new_cf(db: &DB,
              cf_handle: DBCFHandle,
              readopts: &ReadOptions,
              mode: IteratorMode)
              -> Result<DBIterator, Error> {
        unsafe {
            let iterator =
                rocksdb_ffi::rocksdb_create_iterator_cf(db.inner,
                                                        readopts.inner,
                                                        cf_handle);

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
            rocksdb_ffi::rocksdb_iter_destroy(self.inner);
        }
    }
}

impl<'a> Snapshot<'a> {
    pub fn new(db: &DB) -> Snapshot {
        let snapshot =
            unsafe { rocksdb_ffi::rocksdb_create_snapshot(db.inner) };
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

    pub fn iterator_cf(&self,
                       cf_handle: DBCFHandle,
                       mode: IteratorMode)
                       -> Result<DBIterator, Error> {
        let mut readopts = ReadOptions::default();
        readopts.set_snapshot(self);
        DBIterator::new_cf(self.db, cf_handle, &readopts, mode)
    }


    pub fn get(&self, key: &[u8]) -> Result<Option<DBVector>, Error> {
        let mut readopts = ReadOptions::default();
        readopts.set_snapshot(self);
        self.db.get_opt(key, &readopts)
    }

    pub fn get_cf(&self,
                  cf: DBCFHandle,
                  key: &[u8])
                  -> Result<Option<DBVector>, Error> {
        let mut readopts = ReadOptions::default();
        readopts.set_snapshot(self);
        self.db.get_cf_opt(cf, key, &readopts)
    }
}

impl<'a> Drop for Snapshot<'a> {
    fn drop(&mut self) {
        unsafe {
            rocksdb_ffi::rocksdb_release_snapshot(self.db.inner, self.inner);
        }
    }
}

// This is for the DB and write batches to share the same API
pub trait Writable {
    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), Error>;
    fn put_cf(&self,
              cf: DBCFHandle,
              key: &[u8],
              value: &[u8])
              -> Result<(), Error>;
    fn merge(&self, key: &[u8], value: &[u8]) -> Result<(), Error>;
    fn merge_cf(&self,
                cf: DBCFHandle,
                key: &[u8],
                value: &[u8])
                -> Result<(), Error>;
    fn delete(&self, key: &[u8]) -> Result<(), Error>;
    fn delete_cf(&self, cf: DBCFHandle, key: &[u8]) -> Result<(), Error>;
}

impl DB {
    /// Open a database with default options
    pub fn open_default(path: &str) -> Result<DB, Error> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        DB::open(&opts, path)
    }

    /// Open the database with specified options
    pub fn open(opts: &Options, path: &str) -> Result<DB, Error> {
        DB::open_cf(opts, path, &[])
    }

    /// Open a database with specified options and column family
    ///
    /// A column family must be created first by calling `DB::create_cf`
    ///
    /// # Panics
    ///
    /// * Panics if the column family doesn't exist
    pub fn open_cf(opts: &Options,
                   path: &str,
                   cfs: &[&str])
                   -> Result<DB, Error> {
        let cpath = match CString::new(path.as_bytes()) {
            Ok(c) => c,
            Err(_) => {
                return Err(Error::new("Failed to convert path to CString \
                                       when opening rocksdb"
                    .to_string()))
            }
        };
        let cpath_ptr = cpath.as_ptr();

        let ospath = Path::new(path);
        if let Err(e) = fs::create_dir_all(&ospath) {
            return Err(Error::new(format!("Failed to create rocksdb \
                                           directory: {:?}",
                                          e)));
        }

        let mut err: *const i8 = 0 as *const i8;
        let err_ptr: *mut *const i8 = &mut err;
        let db: rocksdb_ffi::DBInstance;
        let mut cf_map = BTreeMap::new();

        if cfs.len() == 0 {
            unsafe {
                db = rocksdb_ffi::rocksdb_open(opts.inner,
                                               cpath_ptr as *const _,
                                               err_ptr);
            }
        } else {
            let mut cfs_v = cfs.to_vec();
            // Always open the default column family
            if !cfs_v.contains(&"default") {
                cfs_v.push("default");
            }

            // We need to store our CStrings in an intermediate vector
            // so that their pointers remain valid.
            let c_cfs: Vec<CString> = cfs_v.iter()
                .map(|cf| CString::new(cf.as_bytes()).unwrap())
                .collect();

            let cfnames: Vec<*const _> = c_cfs.iter()
                .map(|cf| cf.as_ptr())
                .collect();

            // These handles will be populated by DB.
            let cfhandles: Vec<rocksdb_ffi::DBCFHandle> = cfs_v.iter()
                .map(|_| 0 as rocksdb_ffi::DBCFHandle)
                .collect();

            // TODO(tyler) allow options to be passed in.
            let cfopts: Vec<rocksdb_ffi::DBOptions> = cfs_v.iter()
                .map(|_| unsafe { rocksdb_ffi::rocksdb_options_create() })
                .collect();

            // Prepare to ship to C.
            let copts: *const rocksdb_ffi::DBOptions = cfopts.as_ptr();
            let handles: *const rocksdb_ffi::DBCFHandle = cfhandles.as_ptr();
            let nfam = cfs_v.len();
            unsafe {
                db = rocksdb_ffi::rocksdb_open_column_families(opts.inner, cpath_ptr as *const _,
                                                               nfam as libc::c_int,
                                                               cfnames.as_ptr() as *const _,
                                                               copts, handles, err_ptr);
            }

            for handle in &cfhandles {
                if handle.is_null() {
                    return Err(Error::new("Received null column family \
                                           handle from DB."
                        .to_string()));
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
            return Err(Error::new("Could not initialize database."
                .to_string()));
        }

        Ok(DB {
            inner: db,
            cfs: cf_map,
            path: path.to_owned(),
        })
    }

    pub fn destroy(opts: &Options, path: &str) -> Result<(), Error> {
        let cpath = CString::new(path.as_bytes()).unwrap();
        let cpath_ptr = cpath.as_ptr();

        let mut err: *const i8 = 0 as *const i8;
        let err_ptr: *mut *const i8 = &mut err;
        unsafe {
            rocksdb_ffi::rocksdb_destroy_db(opts.inner,
                                            cpath_ptr as *const _,
                                            err_ptr);
        }
        if !err.is_null() {
            return Err(Error::new(error_message(err)));
        }
        Ok(())
    }

    pub fn repair(opts: Options, path: &str) -> Result<(), Error> {
        let cpath = CString::new(path.as_bytes()).unwrap();
        let cpath_ptr = cpath.as_ptr();

        let mut err: *const i8 = 0 as *const i8;
        let err_ptr: *mut *const i8 = &mut err;
        unsafe {
            rocksdb_ffi::rocksdb_repair_db(opts.inner,
                                           cpath_ptr as *const _,
                                           err_ptr);
        }
        if !err.is_null() {
            return Err(Error::new(error_message(err)));
        }
        Ok(())
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn write_opt(&self,
                     batch: WriteBatch,
                     writeopts: &WriteOptions)
                     -> Result<(), Error> {
        let mut err: *const i8 = 0 as *const i8;
        let err_ptr: *mut *const i8 = &mut err;
        unsafe {
            rocksdb_ffi::rocksdb_write(self.inner,
                                       writeopts.inner,
                                       batch.inner,
                                       err_ptr);
        }
        if !err.is_null() {
            return Err(Error::new(error_message(err)));
        }
        Ok(())
    }

    pub fn write(&self, batch: WriteBatch) -> Result<(), Error> {
        self.write_opt(batch, &WriteOptions::default())
    }

    pub fn write_without_wal(&self, batch: WriteBatch) -> Result<(), Error> {
        let mut wo = WriteOptions::new();
        wo.disable_wal(true);
        self.write_opt(batch, &wo)
    }

    pub fn get_opt(&self,
                   key: &[u8],
                   readopts: &ReadOptions)
                   -> Result<Option<DBVector>, Error> {
        if readopts.inner.is_null() {
            return Err(Error::new("Unable to create rocksdb read options.  \
                                   This is a fairly trivial call, and its \
                                   failure may be indicative of a \
                                   mis-compiled or mis-loaded rocksdb \
                                   library."
                .to_string()));
        }

        unsafe {
            let val_len: size_t = 0;
            let val_len_ptr = &val_len as *const size_t;
            let mut err: *const i8 = 0 as *const i8;
            let err_ptr: *mut *const i8 = &mut err;
            let val =
                rocksdb_ffi::rocksdb_get(self.inner,
                                         readopts.inner,
                                         key.as_ptr(),
                                         key.len() as size_t,
                                         val_len_ptr,
                                         err_ptr) as *mut u8;
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

    pub fn get_cf_opt(&self,
                      cf: DBCFHandle,
                      key: &[u8],
                      readopts: &ReadOptions)
                      -> Result<Option<DBVector>, Error> {
        if readopts.inner.is_null() {
            return Err(Error::new("Unable to create rocksdb read options.  \
                                   This is a fairly trivial call, and its \
                                   failure may be indicative of a \
                                   mis-compiled or mis-loaded rocksdb \
                                   library."
                .to_string()));
        }

        unsafe {
            let val_len: size_t = 0;
            let val_len_ptr = &val_len as *const size_t;
            let mut err: *const i8 = 0 as *const i8;
            let err_ptr: *mut *const i8 = &mut err;
            let val =
                rocksdb_ffi::rocksdb_get_cf(self.inner,
                                            readopts.inner,
                                            cf,
                                            key.as_ptr(),
                                            key.len() as size_t,
                                            val_len_ptr,
                                            err_ptr) as *mut u8;
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

    pub fn get_cf(&self,
                  cf: DBCFHandle,
                  key: &[u8])
                  -> Result<Option<DBVector>, Error> {
        self.get_cf_opt(cf, key, &ReadOptions::default())
    }

    pub fn multi_get(&self,
                  keys: Vec<&[u8]>)
                  -> Vec<Result<Option<DBVector>, String>> {
        self.multi_get_opt(keys, &ReadOptions::default())
    }

    pub fn multi_get_opt(&self,
                      keys: Vec<&[u8]>,
                      readopts: &ReadOptions)
                      -> Vec<Result<Option<DBVector>, String>> {

        unsafe {
            let mut err_ptrs : Vec<*mut i8> = vec![0 as *mut i8; keys.len()];
            let mut value_ptrs : Vec<*mut u8> = vec![0 as *mut u8; keys.len()];
            let mut length_ptrs : Vec<size_t> = vec![0 as size_t; keys.len()];

            let mut key_lens : Vec<size_t> =  Vec::with_capacity(keys.len());
            let mut key_ptrs : Vec<*const u8> = Vec::with_capacity(keys.len());
            for ref key in &keys {
                key_lens.push(key.len());
                key_ptrs.push(key.as_ptr());
            }

            rocksdb_ffi::rocksdb_multi_get(self.inner,
                                            readopts.inner,
                                            keys.len() as size_t,
                                            key_ptrs.as_ptr(),
                                            key_lens.as_ptr(),
                                            value_ptrs.as_mut_ptr(),
                                            length_ptrs.as_mut_ptr(),
                                            err_ptrs.as_mut_ptr());


            let mut values = vec![];
            for i in 0..keys.len() {

                let err = err_ptrs[i];
                if !err.is_null() {
                    let em = error_message(err);
                    values.push(Err(em));
                }

                let val = value_ptrs[i];
                if val.is_null() {
                    values.push(Ok(None));
                } else {
                    values.push(Ok(Some(DBVector::from_c(val, length_ptrs[i]))));
                }
            }

            return values;
        }
    }

    pub fn multi_get_cf(&self,
                  cf: Vec<DBCFHandle>,
                  keys: Vec<&[u8]>)
                  -> Vec<Result<Option<DBVector>, String>> {
        self.multi_get_cf_opt(cf, keys, &ReadOptions::default())
    }

    pub fn multi_get_cf_opt(&self,
                      cf: Vec<DBCFHandle>,
                      keys: Vec<&[u8]>,
                      readopts: &ReadOptions)
                      -> Vec<Result<Option<DBVector>, String>> {

        unsafe {
            let mut err_ptrs : Vec<*mut i8> = vec![0 as *mut i8; keys.len()];
            let mut value_ptrs : Vec<*mut u8> = vec![0 as *mut u8; keys.len()];
            let mut length_ptrs : Vec<size_t> = vec![0 as size_t; keys.len()];

            let mut key_lens : Vec<size_t> =  Vec::with_capacity(keys.len());
            let mut key_ptrs : Vec<*const u8> = Vec::with_capacity(keys.len());
            for ref key in &keys {
                key_lens.push(key.len());
                key_ptrs.push(key.as_ptr());
            }

            rocksdb_ffi::rocksdb_multi_get_cf(self.inner,
                                            readopts.inner,
                                            cf.as_ptr(),
                                            keys.len() as size_t,
                                            key_ptrs.as_ptr(),
                                            key_lens.as_ptr(),
                                            value_ptrs.as_mut_ptr(),
                                            length_ptrs.as_mut_ptr(),
                                            err_ptrs.as_mut_ptr());


            let mut values = vec![];
            for i in 0..keys.len() {

                let err = err_ptrs[i];
                if !err.is_null() {
                    let em = error_message(err);
                    values.push(Err(em));
                }

                let val = value_ptrs[i];
                if val.is_null() {
                    values.push(Ok(None));
                } else {
                    values.push(Ok(Some(DBVector::from_c(val, length_ptrs[i]))));
                }
            }

            return values;
        }
    }

    pub fn create_cf(&mut self,
                     name: &str,
                     opts: &Options)
                     -> Result<DBCFHandle, Error> {
        let cname = match CString::new(name.as_bytes()) {
            Ok(c) => c,
            Err(_) => {
                return Err(Error::new("Failed to convert path to CString \
                                       when opening rocksdb"
                    .to_string()))
            }
        };
        let cname_ptr = cname.as_ptr();
        let mut err: *const i8 = 0 as *const i8;
        let err_ptr: *mut *const i8 = &mut err;
        let cf_handler = unsafe {
            let cf_handler =
                rocksdb_ffi::rocksdb_create_column_family(self.inner,
                                                          opts.inner,
                                                          cname_ptr as *const _,
                                                          err_ptr);
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
            return Err(Error::new(format!("Invalid column family: {}", name)
                .to_string()));
        }

        let mut err: *const i8 = 0 as *const i8;
        let err_ptr: *mut *const i8 = &mut err;
        unsafe {
            rocksdb_ffi::rocksdb_drop_column_family(self.inner,
                                                    *cf.unwrap(),
                                                    err_ptr);
        }
        if !err.is_null() {
            return Err(Error::new(error_message(err)));
        }

        Ok(())
    }

    /// Return the underlying column family handle
    pub fn cf_handle(&self, name: &str) -> Option<&DBCFHandle> {
        self.cfs.get(name)
    }

    pub fn iterator(&self, mode: IteratorMode) -> DBIterator {
        let opts = ReadOptions::default();
        DBIterator::new(self, &opts, mode)
    }

    pub fn iterator_cf(&self,
                       cf_handle: DBCFHandle,
                       mode: IteratorMode)
                       -> Result<DBIterator, Error> {
        let opts = ReadOptions::default();
        DBIterator::new_cf(self, cf_handle, &opts, mode)
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot::new(self)
    }

    pub fn put_opt(&self,
                   key: &[u8],
                   value: &[u8],
                   writeopts: &WriteOptions)
                   -> Result<(), Error> {
        unsafe {
            let mut err: *const i8 = 0 as *const i8;
            let err_ptr: *mut *const i8 = &mut err;
            rocksdb_ffi::rocksdb_put(self.inner,
                                     writeopts.inner,
                                     key.as_ptr(),
                                     key.len() as size_t,
                                     value.as_ptr(),
                                     value.len() as size_t,
                                     err_ptr);
            if !err.is_null() {
                return Err(Error::new(error_message(err)));
            }
            Ok(())
        }
    }

    pub fn put_cf_opt(&self,
                      cf: DBCFHandle,
                      key: &[u8],
                      value: &[u8],
                      writeopts: &WriteOptions)
                      -> Result<(), Error> {
        unsafe {
            let mut err: *const i8 = 0 as *const i8;
            let err_ptr: *mut *const i8 = &mut err;
            rocksdb_ffi::rocksdb_put_cf(self.inner,
                                        writeopts.inner,
                                        cf,
                                        key.as_ptr(),
                                        key.len() as size_t,
                                        value.as_ptr(),
                                        value.len() as size_t,
                                        err_ptr);
            if !err.is_null() {
                return Err(Error::new(error_message(err)));
            }
            Ok(())
        }
    }
    pub fn merge_opt(&self,
                     key: &[u8],
                     value: &[u8],
                     writeopts: &WriteOptions)
                     -> Result<(), Error> {
        unsafe {
            let mut err: *const i8 = 0 as *const i8;
            let err_ptr: *mut *const i8 = &mut err;
            rocksdb_ffi::rocksdb_merge(self.inner,
                                       writeopts.inner,
                                       key.as_ptr(),
                                       key.len() as size_t,
                                       value.as_ptr(),
                                       value.len() as size_t,
                                       err_ptr);
            if !err.is_null() {
                return Err(Error::new(error_message(err)));
            }
            Ok(())
        }
    }
    fn merge_cf_opt(&self,
                    cf: DBCFHandle,
                    key: &[u8],
                    value: &[u8],
                    writeopts: &WriteOptions)
                    -> Result<(), Error> {
        unsafe {
            let mut err: *const i8 = 0 as *const i8;
            let err_ptr: *mut *const i8 = &mut err;
            rocksdb_ffi::rocksdb_merge_cf(self.inner,
                                          writeopts.inner,
                                          cf,
                                          key.as_ptr(),
                                          key.len() as size_t,
                                          value.as_ptr(),
                                          value.len() as size_t,
                                          err_ptr);
            if !err.is_null() {
                return Err(Error::new(error_message(err)));
            }
            Ok(())
        }
    }
    fn delete_opt(&self,
                  key: &[u8],
                  writeopts: &WriteOptions)
                  -> Result<(), Error> {
        unsafe {
            let mut err: *const i8 = 0 as *const i8;
            let err_ptr: *mut *const i8 = &mut err;
            rocksdb_ffi::rocksdb_delete(self.inner,
                                        writeopts.inner,
                                        key.as_ptr(),
                                        key.len() as size_t,
                                        err_ptr);
            if !err.is_null() {
                return Err(Error::new(error_message(err)));
            }
            Ok(())
        }
    }
    fn delete_cf_opt(&self,
                     cf: DBCFHandle,
                     key: &[u8],
                     writeopts: &WriteOptions)
                     -> Result<(), Error> {
        unsafe {
            let mut err: *const i8 = 0 as *const i8;
            let err_ptr: *mut *const i8 = &mut err;
            rocksdb_ffi::rocksdb_delete_cf(self.inner,
                                           writeopts.inner,
                                           cf,
                                           key.as_ptr(),
                                           key.len() as size_t,
                                           err_ptr);
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

    fn put_cf(&self,
              cf: DBCFHandle,
              key: &[u8],
              value: &[u8])
              -> Result<(), Error> {
        self.put_cf_opt(cf, key, value, &WriteOptions::default())
    }

    fn merge(&self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        self.merge_opt(key, value, &WriteOptions::default())
    }

    fn merge_cf(&self,
                cf: DBCFHandle,
                key: &[u8],
                value: &[u8])
                -> Result<(), Error> {
        self.merge_cf_opt(cf, key, value, &WriteOptions::default())
    }

    fn delete(&self, key: &[u8]) -> Result<(), Error> {
        self.delete_opt(key, &WriteOptions::default())
    }

    fn delete_cf(&self, cf: DBCFHandle, key: &[u8]) -> Result<(), Error> {
        self.delete_cf_opt(cf, key, &WriteOptions::default())
    }
}

impl WriteBatch {
    pub fn len(&self) -> usize {
        unsafe { rocksdb_ffi::rocksdb_writebatch_count(self.inner) as usize }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for WriteBatch {
    fn default() -> WriteBatch {
        WriteBatch {
            inner: unsafe { rocksdb_ffi::rocksdb_writebatch_create() },
        }
    }
}

impl Drop for WriteBatch {
    fn drop(&mut self) {
        unsafe { rocksdb_ffi::rocksdb_writebatch_destroy(self.inner) }
    }
}

impl Drop for DB {
    fn drop(&mut self) {
        unsafe {
            for cf in self.cfs.values() {
                rocksdb_ffi::rocksdb_column_family_handle_destroy(*cf);
            }
            rocksdb_ffi::rocksdb_close(self.inner);
        }
    }
}

impl fmt::Debug for DB {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RocksDB {{ path: {:?} }}", self.path())
    }
}

impl Writable for WriteBatch {
    /// Insert a value into the database under the given key
    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        unsafe {
            rocksdb_ffi::rocksdb_writebatch_put(self.inner,
                                                key.as_ptr(),
                                                key.len() as size_t,
                                                value.as_ptr(),
                                                value.len() as size_t);
            Ok(())
        }
    }

    fn put_cf(&self,
              cf: DBCFHandle,
              key: &[u8],
              value: &[u8])
              -> Result<(), Error> {
        unsafe {
            rocksdb_ffi::rocksdb_writebatch_put_cf(self.inner,
                                                   cf,
                                                   key.as_ptr(),
                                                   key.len() as size_t,
                                                   value.as_ptr(),
                                                   value.len() as size_t);
            Ok(())
        }
    }

    fn merge(&self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        unsafe {
            rocksdb_ffi::rocksdb_writebatch_merge(self.inner,
                                                  key.as_ptr(),
                                                  key.len() as size_t,
                                                  value.as_ptr(),
                                                  value.len() as size_t);
            Ok(())
        }
    }

    fn merge_cf(&self,
                cf: DBCFHandle,
                key: &[u8],
                value: &[u8])
                -> Result<(), Error> {
        unsafe {
            rocksdb_ffi::rocksdb_writebatch_merge_cf(self.inner,
                                                     cf,
                                                     key.as_ptr(),
                                                     key.len() as size_t,
                                                     value.as_ptr(),
                                                     value.len() as size_t);
            Ok(())
        }
    }

    /// Remove the database entry for key
    ///
    /// Returns Err if the key was not found
    fn delete(&self, key: &[u8]) -> Result<(), Error> {
        unsafe {
            rocksdb_ffi::rocksdb_writebatch_delete(self.inner,
                                                   key.as_ptr(),
                                                   key.len() as size_t);
            Ok(())
        }
    }

    fn delete_cf(&self, cf: DBCFHandle, key: &[u8]) -> Result<(), Error> {
        unsafe {
            rocksdb_ffi::rocksdb_writebatch_delete_cf(self.inner,
                                                      cf,
                                                      key.as_ptr(),
                                                      key.len() as size_t);
            Ok(())
        }
    }
}

impl Drop for ReadOptions {
    fn drop(&mut self) {
        unsafe { rocksdb_ffi::rocksdb_readoptions_destroy(self.inner) }
    }
}

impl ReadOptions {
    // TODO add snapshot setting here
    // TODO add snapshot wrapper structs with proper destructors;
    // that struct needs an "iterator" impl too.
    #[allow(dead_code)]
    fn fill_cache(&mut self, v: bool) {
        unsafe {
            rocksdb_ffi::rocksdb_readoptions_set_fill_cache(self.inner, v);
        }
    }

    fn set_snapshot(&mut self, snapshot: &Snapshot) {
        unsafe {
            rocksdb_ffi::rocksdb_readoptions_set_snapshot(self.inner,
                                                          snapshot.inner);
        }
    }

    pub fn set_iterate_upper_bound(&mut self, key: &[u8]) {
        unsafe {
            rocksdb_ffi::rocksdb_readoptions_set_iterate_upper_bound(self.inner,
                                                                     key.as_ptr(),
                                                                     key.len() as size_t);
        }
    }
}

impl Default for ReadOptions {
    fn default() -> ReadOptions {
        unsafe {
            ReadOptions { inner: rocksdb_ffi::rocksdb_readoptions_create() }
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
            libc::free(self.base as *mut libc::c_void);
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
            assert!(s ==
                    Error::new("IO error: lock _rust_rocksdb_error/LOCK: No \
                                locks available"
                .to_string()))
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
fn multi_get_test() {
    let path = "_rust_rocksdb_multi_get_test";
    {
        let db = DB::open_default(path).unwrap();
        let p = db.put(b"mk1", b"v1111");
        assert!(p.is_ok());
        let p = db.put(b"mk2", b"v2222");
        assert!(p.is_ok());
        let p = db.put(b"mk3", b"v3333");
        assert!(p.is_ok());

        let all = db.multi_get(vec![b"mk1",b"mk2",b"mk3"]);
        assert_eq!(all.len(), 3);
        match all[0] {
            Ok(Some(ref u)) => assert_eq!(u.to_utf8(),Some("v1111")),
            _ =>  assert!(false)
        }
        match all[1] {
            Ok(Some(ref u)) => assert_eq!(u.to_utf8(),Some("v2222")),
            _ =>  assert!(false)
        }
        match all[2] {
            Ok(Some(ref u)) => assert_eq!(u.to_utf8(),Some("v3333")),
            _ =>  assert!(false)
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
            println!("Hello {}: {}",
                     from_utf8(&*k).unwrap(),
                     from_utf8(&*v).unwrap());
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
