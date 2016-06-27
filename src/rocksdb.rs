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

use std::collections::BTreeMap;
use std::ffi::CString;
use std::fs;
use std::ops::Deref;
use std::path::Path;
use std::slice;
use std::str::from_utf8;

use self::libc::size_t;

use rocksdb_ffi::{self, DBCFHandle, error_message};
use rocksdb_options::{Options, WriteOptions};

pub struct DB {
    inner: rocksdb_ffi::DBInstance,
    cfs: BTreeMap<String, DBCFHandle>,
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

#[allow(non_camel_case_types)]
pub enum Direction {
    Forward,
    Reverse,
}

impl Iterator for DBIterator {
    type Item = (Box<[u8]>, Box<[u8]>);

    fn next(&mut self) -> Option<(Box<[u8]>, Box<[u8]>)> {
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
            let key = unsafe {
                slice::from_raw_parts(key_ptr, key_len as usize)
            };
            let val_ptr = unsafe {
                rocksdb_ffi::rocksdb_iter_value(native_iter, val_len_ptr)
            };
            let val = unsafe {
                slice::from_raw_parts(val_ptr, val_len as usize)
            };

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
    fn new<'b>(db: &DB,
               readopts: &'b ReadOptions,
               mode: IteratorMode)
               -> DBIterator {
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
              -> Result<DBIterator, String> {
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
        let snapshot = unsafe {
            rocksdb_ffi::rocksdb_create_snapshot(db.inner)
        };
        Snapshot {
            db: db,
            inner: snapshot,
        }
    }

    pub fn iterator(&self, mode: IteratorMode) -> DBIterator {
        let mut readopts = ReadOptions::new();
        readopts.set_snapshot(self);
        DBIterator::new(self.db, &readopts, mode)
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<DBVector>, String> {
        let mut readopts = ReadOptions::new();
        readopts.set_snapshot(self);
        self.db.get_opt(key, &readopts)
    }

    pub fn get_cf(&self,
                  cf: DBCFHandle,
                  key: &[u8])
                  -> Result<Option<DBVector>, String> {
        let mut readopts = ReadOptions::new();
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
    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), String>;
    fn put_cf(&self,
              cf: DBCFHandle,
              key: &[u8],
              value: &[u8])
              -> Result<(), String>;
    fn merge(&self, key: &[u8], value: &[u8]) -> Result<(), String>;
    fn merge_cf(&self,
                cf: DBCFHandle,
                key: &[u8],
                value: &[u8])
                -> Result<(), String>;
    fn delete(&self, key: &[u8]) -> Result<(), String>;
    fn delete_cf(&self, cf: DBCFHandle, key: &[u8]) -> Result<(), String>;
}

impl DB {
    pub fn open_default(path: &str) -> Result<DB, String> {
        let mut opts = Options::new();
        opts.create_if_missing(true);
        DB::open(&opts, path)
    }

    pub fn open(opts: &Options, path: &str) -> Result<DB, String> {
        DB::open_cf(opts, path, &[])
    }

    pub fn open_cf(opts: &Options,
                   path: &str,
                   cfs: &[&str])
                   -> Result<DB, String> {
        let cpath = match CString::new(path.as_bytes()) {
            Ok(c) => c,
            Err(_) => {
                return Err("Failed to convert path to CString when opening \
                            rocksdb"
                               .to_string())
            }
        };
        let cpath_ptr = cpath.as_ptr();

        let ospath = Path::new(path);
        match fs::create_dir_all(&ospath) {
            Err(e) => {
                return Err(format!("Failed to create rocksdb directory: \
                                      {:?}",
                                   e))
            }
            Ok(_) => (),
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
                                           .map(|cf| {
                                               CString::new(cf.as_bytes())
                                                   .unwrap()
                                           })
                                           .collect();

            let cfnames: Vec<*const _> = c_cfs.iter()
                                              .map(|cf| cf.as_ptr())
                                              .collect();

            // These handles will be populated by DB.
            let cfhandles: Vec<rocksdb_ffi::DBCFHandle> =
                cfs_v.iter()
                     .map(|_| 0 as rocksdb_ffi::DBCFHandle)
                     .collect();

            // TODO(tyler) allow options to be passed in.
            let cfopts: Vec<rocksdb_ffi::DBOptions> =
                cfs_v.iter()
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

            for handle in cfhandles.iter() {
                if handle.is_null() {
                    return Err("Received null column family handle from DB."
                                   .to_string());
                }
            }

            for (n, h) in cfs_v.iter().zip(cfhandles) {
                cf_map.insert(n.to_string(), h);
            }
        }

        if !err.is_null() {
            return Err(error_message(err));
        }
        if db.is_null() {
            return Err("Could not initialize database.".to_string());
        }

        Ok(DB {
            inner: db,
            cfs: cf_map,
        })
    }

    pub fn destroy(opts: &Options, path: &str) -> Result<(), String> {
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
            return Err(error_message(err));
        }
        Ok(())
    }

    pub fn repair(opts: &Options, path: &str) -> Result<(), String> {
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
            return Err(error_message(err));
        }
        Ok(())
    }

    pub fn write_opt(&self,
                     batch: WriteBatch,
                     writeopts: &WriteOptions)
                     -> Result<(), String> {
        let mut err: *const i8 = 0 as *const i8;
        let err_ptr: *mut *const i8 = &mut err;
        unsafe {
            rocksdb_ffi::rocksdb_write(self.inner,
                                       writeopts.inner,
                                       batch.inner,
                                       err_ptr);
        }
        if !err.is_null() {
            return Err(error_message(err));
        }
        return Ok(());
    }

    pub fn write(&self, batch: WriteBatch) -> Result<(), String> {
        self.write_opt(batch, &WriteOptions::new())
    }

    pub fn get_opt(&self,
                   key: &[u8],
                   readopts: &ReadOptions)
                   -> Result<Option<DBVector>, String> {
        if readopts.inner.is_null() {
            return Err("Unable to create rocksdb read options.  This is a \
                        fairly trivial call, and its failure may be \
                        indicative of a mis-compiled or mis-loaded rocksdb \
                        library."
                           .to_string());
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
                return Err(error_message(err));
            }
            match val.is_null() {
                true => Ok(None),
                false => Ok(Some(DBVector::from_c(val, val_len))),
            }
        }
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<DBVector>, String> {
        self.get_opt(key, &ReadOptions::new())
    }

    pub fn get_cf_opt(&self,
                      cf: DBCFHandle,
                      key: &[u8],
                      readopts: &ReadOptions)
                      -> Result<Option<DBVector>, String> {
        if readopts.inner.is_null() {
            return Err("Unable to create rocksdb read options.  This is a \
                        fairly trivial call, and its failure may be \
                        indicative of a mis-compiled or mis-loaded rocksdb \
                        library."
                           .to_string());
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
                return Err(error_message(err));
            }
            match val.is_null() {
                true => Ok(None),
                false => Ok(Some(DBVector::from_c(val, val_len))),
            }
        }
    }

    pub fn get_cf(&self,
                  cf: DBCFHandle,
                  key: &[u8])
                  -> Result<Option<DBVector>, String> {
        self.get_cf_opt(cf, key, &ReadOptions::new())
    }

    pub fn create_cf(&mut self,
                     name: &str,
                     opts: &Options)
                     -> Result<DBCFHandle, String> {
        let cname = match CString::new(name.as_bytes()) {
            Ok(c) => c,
            Err(_) => {
                return Err("Failed to convert path to CString when opening \
                            rocksdb"
                               .to_string())
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
            return Err(error_message(err));
        }
        Ok(cf_handler)
    }

    pub fn drop_cf(&mut self, name: &str) -> Result<(), String> {
        let cf = self.cfs.get(name);
        if cf.is_none() {
            return Err(format!("Invalid column family: {}", name).to_string());
        }

        let mut err: *const i8 = 0 as *const i8;
        let err_ptr: *mut *const i8 = &mut err;
        unsafe {
            rocksdb_ffi::rocksdb_drop_column_family(self.inner,
                                                    *cf.unwrap(),
                                                    err_ptr);
        }
        if !err.is_null() {
            return Err(error_message(err));
        }

        Ok(())
    }

    pub fn cf_handle(&self, name: &str) -> Option<&DBCFHandle> {
        self.cfs.get(name)
    }

    pub fn iterator(&self, mode: IteratorMode) -> DBIterator {
        let opts = ReadOptions::new();
        DBIterator::new(&self, &opts, mode)
    }

    pub fn iterator_cf(&self,
                       cf_handle: DBCFHandle,
                       mode: IteratorMode)
                       -> Result<DBIterator, String> {
        let opts = ReadOptions::new();
        DBIterator::new_cf(&self, cf_handle, &opts, mode)
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot::new(self)
    }

    pub fn put_opt(&self,
                   key: &[u8],
                   value: &[u8],
                   writeopts: &WriteOptions)
                   -> Result<(), String> {
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
                return Err(error_message(err));
            }
            Ok(())
        }
    }

    pub fn put_cf_opt(&self,
                      cf: DBCFHandle,
                      key: &[u8],
                      value: &[u8],
                      writeopts: &WriteOptions)
                      -> Result<(), String> {
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
                return Err(error_message(err));
            }
            Ok(())
        }
    }
    pub fn merge_opt(&self,
                     key: &[u8],
                     value: &[u8],
                     writeopts: &WriteOptions)
                     -> Result<(), String> {
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
                return Err(error_message(err));
            }
            Ok(())
        }
    }
    fn merge_cf_opt(&self,
                    cf: DBCFHandle,
                    key: &[u8],
                    value: &[u8],
                    writeopts: &WriteOptions)
                    -> Result<(), String> {
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
                return Err(error_message(err));
            }
            Ok(())
        }
    }
    fn delete_opt(&self,
                  key: &[u8],
                  writeopts: &WriteOptions)
                  -> Result<(), String> {
        unsafe {
            let mut err: *const i8 = 0 as *const i8;
            let err_ptr: *mut *const i8 = &mut err;
            rocksdb_ffi::rocksdb_delete(self.inner,
                                        writeopts.inner,
                                        key.as_ptr(),
                                        key.len() as size_t,
                                        err_ptr);
            if !err.is_null() {
                return Err(error_message(err));
            }
            Ok(())
        }
    }
    fn delete_cf_opt(&self,
                     cf: DBCFHandle,
                     key: &[u8],
                     writeopts: &WriteOptions)
                     -> Result<(), String> {
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
                return Err(error_message(err));
            }
            Ok(())
        }
    }
}

impl Writable for DB {
    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), String> {
        self.put_opt(key, value, &WriteOptions::new())
    }

    fn put_cf(&self,
              cf: DBCFHandle,
              key: &[u8],
              value: &[u8])
              -> Result<(), String> {
        self.put_cf_opt(cf, key, value, &WriteOptions::new())
    }

    fn merge(&self, key: &[u8], value: &[u8]) -> Result<(), String> {
        self.merge_opt(key, value, &WriteOptions::new())
    }

    fn merge_cf(&self,
                cf: DBCFHandle,
                key: &[u8],
                value: &[u8])
                -> Result<(), String> {
        self.merge_cf_opt(cf, key, value, &WriteOptions::new())
    }

    fn delete(&self, key: &[u8]) -> Result<(), String> {
        self.delete_opt(key, &WriteOptions::new())
    }

    fn delete_cf(&self, cf: DBCFHandle, key: &[u8]) -> Result<(), String> {
        self.delete_cf_opt(cf, key, &WriteOptions::new())
    }
}

impl WriteBatch {
    pub fn new() -> WriteBatch {
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
            for (_, cf) in self.cfs.iter() {
                rocksdb_ffi::rocksdb_column_family_handle_destroy(*cf);
            }
            rocksdb_ffi::rocksdb_close(self.inner);
        }
    }
}

impl Writable for WriteBatch {
    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), String> {
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
              -> Result<(), String> {
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

    fn merge(&self, key: &[u8], value: &[u8]) -> Result<(), String> {
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
                -> Result<(), String> {
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

    fn delete(&self, key: &[u8]) -> Result<(), String> {
        unsafe {
            rocksdb_ffi::rocksdb_writebatch_delete(self.inner,
                                                   key.as_ptr(),
                                                   key.len() as size_t);
            Ok(())
        }
    }

    fn delete_cf(&self, cf: DBCFHandle, key: &[u8]) -> Result<(), String> {
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
    pub fn new() -> ReadOptions {
        unsafe {
            ReadOptions { inner: rocksdb_ffi::rocksdb_readoptions_create() }
        }
    }
    // TODO add snapshot setting here
    // TODO add snapshot wrapper structs with proper destructors;
    // that struct needs an "iterator" impl too.
    #[allow(dead_code)]
    fn fill_cache(&mut self, v: bool) {
        unsafe {
            rocksdb_ffi::rocksdb_readoptions_set_fill_cache(self.inner, v);
        }
    }

    pub fn set_snapshot(&mut self, snapshot: &Snapshot) {
        unsafe {
            rocksdb_ffi::rocksdb_readoptions_set_snapshot(self.inner,
                                                          snapshot.inner);
        }
    }
}

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

    pub fn to_utf8<'a>(&'a self) -> Option<&'a str> {
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
        let r: Result<Option<DBVector>, String> = db.get(b"k1");
        assert!(r.unwrap().unwrap().to_utf8().unwrap() == "v1111");
        assert!(db.delete(b"k1").is_ok());
        assert!(db.get(b"k1").unwrap().is_none());
    }
    let opts = Options::new();
    let result = DB::destroy(&opts, path);
    assert!(result.is_ok());
}

#[test]
fn errors_do_stuff() {
    let path = "_rust_rocksdb_error";
    let _db = DB::open_default(path).unwrap();
    let opts = Options::new();
    // The DB will still be open when we try to destroy and the lock should fail
    match DB::destroy(&opts, path) {
        Err(ref s) => {
            assert!(s ==
                    "IO error: lock _rust_rocksdb_error/LOCK: No locks \
                     available")
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
            let batch = WriteBatch::new();
            assert!(db.get(b"k1").unwrap().is_none());
            let _ = batch.put(b"k1", b"v1111");
            assert!(db.get(b"k1").unwrap().is_none());
            let p = db.write(batch);
            assert!(p.is_ok());
            let r: Result<Option<DBVector>, String> = db.get(b"k1");
            assert!(r.unwrap().unwrap().to_utf8().unwrap() == "v1111");
        }
        {
            // test delete
            let batch = WriteBatch::new();
            let _ = batch.delete(b"k1");
            let p = db.write(batch);
            assert!(p.is_ok());
            assert!(db.get(b"k1").unwrap().is_none());
        }
    }
    let opts = Options::new();
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
    let opts = Options::new();
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
        let r: Result<Option<DBVector>, String> = snap.get(b"k1");
        assert!(r.unwrap().unwrap().to_utf8().unwrap() == "v1111");

        let p = db.put(b"k2", b"v2222");
        assert!(p.is_ok());

        assert!(db.get(b"k2").unwrap().is_some());
        assert!(snap.get(b"k2").unwrap().is_none());
    }
    let opts = Options::new();
    assert!(DB::destroy(&opts, path).is_ok());
}

#[test]
fn options() {
    let mut opts = Options::new();
    assert!(opts.set_parsed_options("rate_limiter_bytes_per_sec=1024").is_ok());
}