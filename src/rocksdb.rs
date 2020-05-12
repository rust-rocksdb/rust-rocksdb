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

use crocksdb_ffi::{
    self, DBBackupEngine, DBCFHandle, DBCache, DBCompressionType, DBEnv, DBInstance, DBMapProperty,
    DBPinnableSlice, DBSequentialFile, DBStatisticsHistogramType, DBStatisticsTickerType,
    DBTablePropertiesCollection, DBTitanDBOptions, DBWriteBatch,
};
use libc::{self, c_char, c_int, c_void, size_t};
use librocksdb_sys::DBMemoryAllocator;
use metadata::ColumnFamilyMetaData;
use rocksdb_options::{
    CColumnFamilyDescriptor, ColumnFamilyDescriptor, ColumnFamilyOptions, CompactOptions,
    CompactionOptions, DBOptions, EnvOptions, FlushOptions, HistogramData,
    IngestExternalFileOptions, LRUCacheOptions, ReadOptions, RestoreOptions, UnsafeSnap,
    WriteOptions,
};
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::fmt::{self, Debug, Formatter};
use std::io;
use std::mem;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::str::from_utf8;
use std::sync::Arc;
use std::{fs, ptr, slice};

#[cfg(feature = "encryption")]
use encryption::{DBEncryptionKeyManager, EncryptionKeyManager};
use table_properties::{TableProperties, TablePropertiesCollection};
use table_properties_rc::TablePropertiesCollection as RcTablePropertiesCollection;
use titan::TitanDBOptions;

pub struct CFHandle {
    inner: *mut DBCFHandle,
}

impl CFHandle {
    pub fn id(&self) -> u32 {
        unsafe { crocksdb_ffi::crocksdb_column_family_handle_id(self.inner) }
    }
}

impl Drop for CFHandle {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_column_family_handle_destroy(self.inner);
        }
    }
}

fn ensure_default_cf_exists<'a>(
    list: &mut Vec<ColumnFamilyDescriptor<'a>>,
    ttls: &mut Vec<i32>,
    is_titan: bool,
) {
    let contains = list.iter().any(|ref cf| cf.is_default());
    if !contains {
        let mut desc = ColumnFamilyDescriptor::default();
        if is_titan {
            desc.options.set_titandb_options(&TitanDBOptions::new());
        }
        list.push(desc);
        if ttls.len() > 0 {
            ttls.push(0);
        }
    }
}

fn split_descriptors<'a>(
    list: Vec<ColumnFamilyDescriptor<'a>>,
    is_titan: bool,
) -> (Vec<&'a str>, Vec<ColumnFamilyOptions>) {
    let mut v1 = Vec::with_capacity(list.len());
    let mut v2 = Vec::with_capacity(list.len());
    for mut d in list {
        v1.push(d.name);
        if is_titan && d.options.titan_inner.is_null() {
            d.options.set_titandb_options(&TitanDBOptions::new());
        }
        v2.push(d.options);
    }
    (v1, v2)
}

fn build_cstring_list(str_list: &[&str]) -> Vec<CString> {
    str_list
        .iter()
        .map(|s| CString::new(s.as_bytes()).unwrap())
        .collect()
}

pub struct MapProperty {
    inner: *mut DBMapProperty,
}

impl Drop for MapProperty {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_destroy_map_property(self.inner);
        }
    }
}

impl MapProperty {
    pub fn new() -> MapProperty {
        unsafe {
            MapProperty {
                inner: crocksdb_ffi::crocksdb_create_map_property(),
            }
        }
    }

    pub fn get_property_value(&self, property: &str) -> String {
        let propname = CString::new(property.as_bytes()).unwrap();
        unsafe {
            let value = crocksdb_ffi::crocksdb_map_property_value(self.inner, propname.as_ptr());
            return CStr::from_ptr(value).to_str().unwrap().to_owned();
        }
    }

    pub fn get_property_int_value(&self, property: &str) -> u64 {
        let propname = CString::new(property.as_bytes()).unwrap();
        unsafe {
            let value =
                crocksdb_ffi::crocksdb_map_property_int_value(self.inner, propname.as_ptr());
            return value as u64;
        }
    }
}

pub struct DB {
    inner: *mut DBInstance,
    cfs: BTreeMap<String, CFHandle>,
    path: String,
    opts: DBOptions,
    _cf_opts: Vec<ColumnFamilyOptions>,
    readonly: bool,
}

impl Debug for DB {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Db [path={}]", self.path)
    }
}

unsafe impl Send for DB {}
unsafe impl Sync for DB {}

impl DB {
    pub fn is_titan(&self) -> bool {
        !self.opts.titan_inner.is_null()
    }
}

pub struct WriteBatch {
    inner: *mut DBWriteBatch,
}

unsafe impl Send for WriteBatch {}

pub struct Snapshot<D: Deref<Target = DB>> {
    db: D,
    snap: UnsafeSnap,
}

pub struct DBIterator<D> {
    _db: D,
    _readopts: ReadOptions,
    inner: *mut crocksdb_ffi::DBIterator,
}

pub enum SeekKey<'a> {
    Start,
    End,
    Key(&'a [u8]),
}

impl<'a> From<&'a [u8]> for SeekKey<'a> {
    fn from(bs: &'a [u8]) -> SeekKey {
        SeekKey::Key(bs)
    }
}

impl<D: Deref<Target = DB>> DBIterator<D> {
    pub fn new(db: D, readopts: ReadOptions) -> DBIterator<D> {
        unsafe {
            let iterator = if db.is_titan() {
                crocksdb_ffi::ctitandb_create_iterator(
                    db.inner,
                    readopts.get_inner(),
                    readopts.get_titan_inner(),
                )
            } else {
                crocksdb_ffi::crocksdb_create_iterator(db.inner, readopts.get_inner())
            };

            DBIterator {
                _db: db,
                _readopts: readopts,
                inner: iterator,
            }
        }
    }

    pub fn new_cf(db: D, cf_handle: &CFHandle, readopts: ReadOptions) -> DBIterator<D> {
        unsafe {
            let iterator = if db.is_titan() {
                crocksdb_ffi::ctitandb_create_iterator_cf(
                    db.inner,
                    readopts.get_inner(),
                    readopts.get_titan_inner(),
                    cf_handle.inner,
                )
            } else {
                crocksdb_ffi::crocksdb_create_iterator_cf(
                    db.inner,
                    readopts.get_inner(),
                    cf_handle.inner,
                )
            };
            DBIterator {
                _db: db,
                _readopts: readopts,
                inner: iterator,
            }
        }
    }
}

impl<D> DBIterator<D> {
    pub fn seek(&mut self, key: SeekKey) -> Result<bool, String> {
        unsafe {
            match key {
                SeekKey::Start => crocksdb_ffi::crocksdb_iter_seek_to_first(self.inner),
                SeekKey::End => crocksdb_ffi::crocksdb_iter_seek_to_last(self.inner),
                SeekKey::Key(key) => {
                    crocksdb_ffi::crocksdb_iter_seek(self.inner, key.as_ptr(), key.len() as size_t)
                }
            }
        }
        self.valid()
    }

    pub fn seek_for_prev(&mut self, key: SeekKey) -> Result<bool, String> {
        unsafe {
            match key {
                SeekKey::Start => crocksdb_ffi::crocksdb_iter_seek_to_first(self.inner),
                SeekKey::End => crocksdb_ffi::crocksdb_iter_seek_to_last(self.inner),
                SeekKey::Key(key) => crocksdb_ffi::crocksdb_iter_seek_for_prev(
                    self.inner,
                    key.as_ptr(),
                    key.len() as size_t,
                ),
            }
        }
        self.valid()
    }

    pub fn prev(&mut self) -> Result<bool, String> {
        unsafe {
            crocksdb_ffi::crocksdb_iter_prev(self.inner);
        }
        self.valid()
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<bool, String> {
        unsafe {
            crocksdb_ffi::crocksdb_iter_next(self.inner);
        }
        self.valid()
    }

    /// Get the key pointed by the iterator. Must be called when `self.valid() == Ok(true)`.
    pub fn key(&self) -> &[u8] {
        debug_assert_eq!(self.valid(), Ok(true));
        let mut key_len: size_t = 0;
        let key_len_ptr: *mut size_t = &mut key_len;
        unsafe {
            let key_ptr = crocksdb_ffi::crocksdb_iter_key(self.inner, key_len_ptr);
            slice::from_raw_parts(key_ptr, key_len as usize)
        }
    }

    /// Get the value pointed by the iterator. Must be called when `self.valid() == Ok(true)`.
    pub fn value(&self) -> &[u8] {
        debug_assert_eq!(self.valid(), Ok(true));
        let mut val_len: size_t = 0;
        let val_len_ptr: *mut size_t = &mut val_len;
        unsafe {
            let val_ptr = crocksdb_ffi::crocksdb_iter_value(self.inner, val_len_ptr);
            slice::from_raw_parts(val_ptr, val_len as usize)
        }
    }

    #[deprecated]
    pub fn kv(&self) -> Option<(Vec<u8>, Vec<u8>)> {
        if self.valid().unwrap() {
            Some((self.key().to_vec(), self.value().to_vec()))
        } else {
            None
        }
    }

    pub fn valid(&self) -> Result<bool, String> {
        let valid = unsafe { crocksdb_ffi::crocksdb_iter_valid(self.inner) };
        if !valid {
            self.status()?;
        }
        Ok(valid)
    }

    fn status(&self) -> Result<(), String> {
        unsafe {
            ffi_try!(crocksdb_iter_get_error(self.inner));
        }
        Ok(())
    }
}

#[deprecated]
pub type Kv = (Vec<u8>, Vec<u8>);

#[deprecated]
impl<'b, D> Iterator for &'b mut DBIterator<D> {
    #[allow(deprecated)]
    type Item = Kv;

    fn next(&mut self) -> Option<Self::Item> {
        match self.valid() {
            Ok(true) => {}
            Ok(false) => return None,
            Err(e) => panic!("invalid iterator: {}", e),
        }
        let k = self.key().to_vec();
        let v = self.value().to_vec();
        let _ = DBIterator::next(self);
        Some((k, v))
    }
}

impl<D> Drop for DBIterator<D> {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_iter_destroy(self.inner);
        }
    }
}

unsafe impl<D: Send> Send for DBIterator<D> {}

unsafe impl<D: Deref<Target = DB> + Send + Sync> Send for Snapshot<D> {}

unsafe impl<D: Deref<Target = DB> + Send + Sync> Sync for Snapshot<D> {}

impl<D: Deref<Target = DB> + Clone> Snapshot<D> {
    /// Create an iterator and clone the inner db.
    ///
    /// Please note that, the snapshot struct could be dropped before the iterator
    /// if use improperly, which seems safe though.
    pub fn iter_opt_clone(&self, mut opt: ReadOptions) -> DBIterator<D> {
        unsafe {
            opt.set_snapshot(&self.snap);
        }
        DBIterator::new(self.db.clone(), opt)
    }
}

impl<D: Deref<Target = DB>> Snapshot<D> {
    pub fn new(db: D) -> Snapshot<D> {
        unsafe {
            Snapshot {
                snap: db.unsafe_snap(),
                db: db,
            }
        }
    }

    pub fn iter(&self) -> DBIterator<&DB> {
        let readopts = ReadOptions::new();
        self.iter_opt(readopts)
    }

    pub fn iter_opt(&self, mut opt: ReadOptions) -> DBIterator<&DB> {
        unsafe {
            opt.set_snapshot(&self.snap);
        }
        DBIterator::new(&self.db, opt)
    }

    pub fn iter_cf(&self, cf_handle: &CFHandle, mut opt: ReadOptions) -> DBIterator<&DB> {
        unsafe {
            opt.set_snapshot(&self.snap);
        }
        DBIterator::new_cf(&self.db, cf_handle, opt)
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<DBVector>, String> {
        let mut readopts = ReadOptions::new();
        unsafe {
            readopts.set_snapshot(&self.snap);
        }
        self.db.get_opt(key, &readopts)
    }

    pub fn get_cf(&self, cf: &CFHandle, key: &[u8]) -> Result<Option<DBVector>, String> {
        let mut readopts = ReadOptions::new();
        unsafe {
            readopts.set_snapshot(&self.snap);
        }
        self.db.get_cf_opt(cf, key, &readopts)
    }

    /// Get the snapshot's sequence number.
    pub fn get_sequence_number(&self) -> u64 {
        unsafe { crocksdb_ffi::crocksdb_get_snapshot_sequence_number(self.snap.get_inner()) }
    }
}

impl<D: Deref<Target = DB>> Drop for Snapshot<D> {
    fn drop(&mut self) {
        unsafe { self.db.release_snap(&self.snap) }
    }
}

// This is for the DB and write batches to share the same API
pub trait Writable {
    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), String>;
    fn put_cf(&self, cf: &CFHandle, key: &[u8], value: &[u8]) -> Result<(), String>;
    fn merge(&self, key: &[u8], value: &[u8]) -> Result<(), String>;
    fn merge_cf(&self, cf: &CFHandle, key: &[u8], value: &[u8]) -> Result<(), String>;
    fn delete(&self, key: &[u8]) -> Result<(), String>;
    fn delete_cf(&self, cf: &CFHandle, key: &[u8]) -> Result<(), String>;
    fn single_delete(&self, key: &[u8]) -> Result<(), String>;
    fn single_delete_cf(&self, cf: &CFHandle, key: &[u8]) -> Result<(), String>;
    fn delete_range(&self, begin_key: &[u8], end_key: &[u8]) -> Result<(), String>;
    fn delete_range_cf(
        &self,
        cf: &CFHandle,
        begin_key: &[u8],
        end_key: &[u8],
    ) -> Result<(), String>;
}

/// A range of keys, `start_key` is included, but not `end_key`.
///
/// You should make sure `end_key` is not less than `start_key`.
pub struct Range<'a> {
    start_key: &'a [u8],
    end_key: &'a [u8],
}

impl<'a> Range<'a> {
    pub fn new(start_key: &'a [u8], end_key: &'a [u8]) -> Range<'a> {
        Range {
            start_key: start_key,
            end_key: end_key,
        }
    }
}

pub struct KeyVersion {
    pub key: String,
    pub value: String,
    pub seq: u64,
    pub value_type: c_int,
}

impl DB {
    pub fn open_default(path: &str) -> Result<DB, String> {
        let mut opts = DBOptions::new();
        opts.create_if_missing(true);
        DB::open(opts, path)
    }

    pub fn open(opts: DBOptions, path: &str) -> Result<DB, String> {
        let cfds: Vec<&str> = vec![];
        DB::open_cf(opts, path, cfds)
    }

    pub fn open_with_ttl(opts: DBOptions, path: &str, ttls: &[i32]) -> Result<DB, String> {
        let cfds: Vec<&str> = vec![];
        if ttls.len() == 0 {
            return Err("ttls is empty in with_ttl function".to_owned());
        }
        DB::open_cf_with_ttl(opts, path, cfds, ttls)
    }

    pub fn open_cf<'a, T>(opts: DBOptions, path: &str, cfds: Vec<T>) -> Result<DB, String>
    where
        T: Into<ColumnFamilyDescriptor<'a>>,
    {
        DB::open_cf_internal(opts, path, cfds, &[], None)
    }

    pub fn open_cf_with_ttl<'a, T>(
        opts: DBOptions,
        path: &str,
        cfds: Vec<T>,
        ttls: &[i32],
    ) -> Result<DB, String>
    where
        T: Into<ColumnFamilyDescriptor<'a>>,
    {
        if ttls.len() == 0 {
            return Err("ttls is empty in with_ttl function".to_owned());
        }
        DB::open_cf_internal(opts, path, cfds, ttls, None)
    }

    pub fn open_for_read_only(
        opts: DBOptions,
        path: &str,
        error_if_log_file_exist: bool,
    ) -> Result<DB, String> {
        let cfds: Vec<&str> = vec![];
        DB::open_cf_for_read_only(opts, path, cfds, error_if_log_file_exist)
    }

    pub fn open_cf_for_read_only<'a, T>(
        opts: DBOptions,
        path: &str,
        cfds: Vec<T>,
        error_if_log_file_exist: bool,
    ) -> Result<DB, String>
    where
        T: Into<ColumnFamilyDescriptor<'a>>,
    {
        DB::open_cf_internal(opts, path, cfds, &[], Some(error_if_log_file_exist))
    }

    fn open_cf_internal<'a, T>(
        mut opts: DBOptions,
        path: &str,
        cfds: Vec<T>,
        ttls: &[i32],
        // if none, open for read write mode.
        // otherwise, open for read only.
        error_if_log_file_exist: Option<bool>,
    ) -> Result<DB, String>
    where
        T: Into<ColumnFamilyDescriptor<'a>>,
    {
        const ERR_CONVERT_PATH: &str = "Failed to convert path to CString when opening rocksdb";
        const ERR_NULL_DB_ONINIT: &str = "Could not initialize database";
        const ERR_NULL_CF_HANDLE: &str = "Received null column family handle from DB";

        let cpath = CString::new(path.as_bytes()).map_err(|_| ERR_CONVERT_PATH.to_owned())?;
        fs::create_dir_all(&Path::new(path)).map_err(|e| {
            format!(
                "Failed to create rocksdb directory: \
                 src/rocksdb.rs:                              \
                 {:?}",
                e
            )
        })?;

        let mut descs = cfds.into_iter().map(|t| t.into()).collect();
        let mut ttls_vec = ttls.to_vec();
        ensure_default_cf_exists(&mut descs, &mut ttls_vec, !opts.titan_inner.is_null());

        let (names, options) = split_descriptors(descs, !opts.titan_inner.is_null());
        let cstrings = build_cstring_list(&names);

        let cf_names: Vec<*const _> = cstrings.iter().map(|cs| cs.as_ptr()).collect();
        let cf_handles: Vec<_> = vec![ptr::null_mut(); cf_names.len()];
        let cf_options: Vec<_> = options
            .iter()
            .map(|x| x.inner as *const crocksdb_ffi::Options)
            .collect();
        let titan_cf_options: Vec<_> = options
            .iter()
            .map(|x| {
                if !x.titan_inner.is_null() {
                    unsafe {
                        crocksdb_ffi::ctitandb_options_set_rocksdb_options(x.titan_inner, x.inner);
                    }
                }
                x.titan_inner as *const crocksdb_ffi::DBTitanDBOptions
            })
            .collect();

        let readonly = error_if_log_file_exist.is_some();

        let with_ttl = if ttls_vec.len() > 0 {
            if ttls_vec.len() == cf_names.len() {
                true
            } else {
                return Err("the length of ttls not equal to length of cfs".to_owned());
            }
        } else {
            false
        };

        let db = {
            let db_options = opts.inner;
            let db_path = cpath.as_ptr();
            let db_cfs_count = cf_names.len() as c_int;
            let db_cf_ptrs = cf_names.as_ptr();
            let db_cf_opts = cf_options.as_ptr();
            let titan_cf_opts = titan_cf_options.as_ptr();
            let db_cf_handles = cf_handles.as_ptr();

            let titan_options = opts.titan_inner;
            if !titan_options.is_null() {
                unsafe {
                    crocksdb_ffi::ctitandb_options_set_rocksdb_options(titan_options, db_options);
                }
                if error_if_log_file_exist.is_some() {
                    return Err("TitanDB doesn't support read only mode.".to_owned());
                } else if with_ttl {
                    return Err("TitanDB doesn't support ttl.".to_owned());
                }
            }

            if !with_ttl {
                if let Some(flag) = error_if_log_file_exist {
                    unsafe {
                        ffi_try!(crocksdb_open_for_read_only_column_families(
                            db_options,
                            db_path,
                            db_cfs_count,
                            db_cf_ptrs,
                            db_cf_opts,
                            db_cf_handles,
                            flag
                        ))
                    }
                } else if titan_options.is_null() {
                    unsafe {
                        ffi_try!(crocksdb_open_column_families(
                            db_options,
                            db_path,
                            db_cfs_count,
                            db_cf_ptrs,
                            db_cf_opts,
                            db_cf_handles
                        ))
                    }
                } else {
                    unsafe {
                        ffi_try!(ctitandb_open_column_families(
                            db_path,
                            titan_options,
                            db_cfs_count,
                            db_cf_ptrs,
                            titan_cf_opts,
                            db_cf_handles
                        ))
                    }
                }
            } else {
                let ttl_array = ttls_vec.as_ptr() as *const c_int;

                unsafe {
                    ffi_try!(crocksdb_open_column_families_with_ttl(
                        db_options,
                        db_path,
                        db_cfs_count,
                        db_cf_ptrs,
                        db_cf_opts,
                        ttl_array,
                        readonly,
                        db_cf_handles
                    ))
                }
            }
        };

        if cf_handles.iter().any(|h| h.is_null()) {
            return Err(ERR_NULL_CF_HANDLE.to_owned());
        }
        if db.is_null() {
            return Err(ERR_NULL_DB_ONINIT.to_owned());
        }

        unsafe {
            // the options provided when opening db may sanitized, so get the latest options.
            crocksdb_ffi::crocksdb_options_destroy(opts.inner);
            opts.inner = crocksdb_ffi::crocksdb_get_db_options(db);
            if !opts.titan_inner.is_null() {
                crocksdb_ffi::ctitandb_options_destroy(opts.titan_inner);
                opts.titan_inner = crocksdb_ffi::ctitandb_get_titan_db_options(db);
            }
        }

        let cfs = names
            .into_iter()
            .zip(cf_handles)
            .map(|(s, h)| (s.to_owned(), CFHandle { inner: h }))
            .collect();

        Ok(DB {
            inner: db,
            cfs: cfs,
            path: path.to_owned(),
            opts: opts,
            _cf_opts: options,
            readonly: readonly,
        })
    }

    pub fn destroy(opts: &DBOptions, path: &str) -> Result<(), String> {
        let cpath = CString::new(path.as_bytes()).unwrap();
        unsafe {
            ffi_try!(crocksdb_destroy_db(opts.inner, cpath.as_ptr()));
        }
        Ok(())
    }

    pub fn repair(opts: DBOptions, path: &str) -> Result<(), String> {
        let cpath = CString::new(path.as_bytes()).unwrap();
        unsafe {
            ffi_try!(crocksdb_repair_db(opts.inner, cpath.as_ptr()));
        }
        Ok(())
    }

    pub fn list_column_families(opts: &DBOptions, path: &str) -> Result<Vec<String>, String> {
        let cpath = match CString::new(path.as_bytes()) {
            Ok(c) => c,
            Err(_) => {
                return Err("Failed to convert path to CString when list \
                            column families"
                    .to_owned());
            }
        };

        let mut cfs: Vec<String> = vec![];
        unsafe {
            let mut lencf: size_t = 0;
            let list = ffi_try!(crocksdb_list_column_families(
                opts.inner,
                cpath.as_ptr(),
                &mut lencf
            ));
            let list_cfs = slice::from_raw_parts(list, lencf);
            for &cf_name in list_cfs {
                let cf = match CStr::from_ptr(cf_name).to_owned().into_string() {
                    Ok(s) => s,
                    Err(e) => return Err(format!("invalid utf8 bytes: {:?}", e)),
                };
                cfs.push(cf);
            }
            crocksdb_ffi::crocksdb_list_column_families_destroy(list, lencf);
        }

        Ok(cfs)
    }

    pub fn env(&self) -> Option<Arc<Env>> {
        self.opts.env()
    }

    pub fn pause_bg_work(&self) {
        unsafe {
            crocksdb_ffi::crocksdb_pause_bg_work(self.inner);
        }
    }

    pub fn continue_bg_work(&self) {
        unsafe {
            crocksdb_ffi::crocksdb_continue_bg_work(self.inner);
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn write_opt(&self, batch: &WriteBatch, writeopts: &WriteOptions) -> Result<(), String> {
        unsafe {
            ffi_try!(crocksdb_write(self.inner, writeopts.inner, batch.inner));
        }
        Ok(())
    }

    pub fn multi_batch_write(
        &self,
        batches: &[WriteBatch],
        writeopts: &WriteOptions,
    ) -> Result<(), String> {
        unsafe {
            let b: Vec<*mut DBWriteBatch> = batches.iter().map(|w| w.inner).collect();
            if !b.is_empty() {
                ffi_try!(crocksdb_write_multi_batch(
                    self.inner,
                    writeopts.inner,
                    b.as_ptr(),
                    b.len()
                ));
            }
        }
        Ok(())
    }

    pub fn write(&self, batch: &WriteBatch) -> Result<(), String> {
        self.write_opt(batch, &WriteOptions::new())
    }

    pub fn write_without_wal(&self, batch: &WriteBatch) -> Result<(), String> {
        let mut wo = WriteOptions::new();
        wo.disable_wal(true);
        self.write_opt(batch, &wo)
    }

    pub fn get_opt(&self, key: &[u8], readopts: &ReadOptions) -> Result<Option<DBVector>, String> {
        unsafe {
            let val = ffi_try!(crocksdb_get_pinned(
                self.inner,
                readopts.get_inner(),
                key.as_ptr(),
                key.len() as size_t
            ));
            if val.is_null() {
                Ok(None)
            } else {
                Ok(Some(DBVector::from_pinned_slice(val)))
            }
        }
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<DBVector>, String> {
        self.get_opt(key, &ReadOptions::new())
    }

    pub fn get_cf_opt(
        &self,
        cf: &CFHandle,
        key: &[u8],
        readopts: &ReadOptions,
    ) -> Result<Option<DBVector>, String> {
        unsafe {
            let val = ffi_try!(crocksdb_get_pinned_cf(
                self.inner,
                readopts.get_inner(),
                cf.inner,
                key.as_ptr(),
                key.len() as size_t
            ));
            if val.is_null() {
                Ok(None)
            } else {
                Ok(Some(DBVector::from_pinned_slice(val)))
            }
        }
    }

    pub fn get_cf(&self, cf: &CFHandle, key: &[u8]) -> Result<Option<DBVector>, String> {
        self.get_cf_opt(cf, key, &ReadOptions::new())
    }

    pub fn create_cf<'a, T>(&mut self, cfd: T) -> Result<&CFHandle, String>
    where
        T: Into<ColumnFamilyDescriptor<'a>>,
    {
        let mut cfd = cfd.into();
        let cname = match CString::new(cfd.name.as_bytes()) {
            Ok(c) => c,
            Err(_) => {
                return Err("Failed to convert path to CString when opening rocksdb".to_owned());
            }
        };
        let cname_ptr = cname.as_ptr();
        unsafe {
            let cf_handler = if !self.is_titan() {
                ffi_try!(crocksdb_create_column_family(
                    self.inner,
                    cfd.options.inner,
                    cname_ptr
                ))
            } else {
                if cfd.options.titan_inner.is_null() {
                    cfd.options.set_titandb_options(&TitanDBOptions::new());
                }
                crocksdb_ffi::ctitandb_options_set_rocksdb_options(
                    cfd.options.titan_inner,
                    cfd.options.inner,
                );
                ffi_try!(ctitandb_create_column_family(
                    self.inner,
                    cfd.options.titan_inner,
                    cname_ptr
                ))
            };
            let handle = CFHandle { inner: cf_handler };
            self._cf_opts.push(cfd.options);
            Ok(match self.cfs.entry(cfd.name.to_owned()) {
                Entry::Occupied(mut e) => {
                    e.insert(handle);
                    e.into_mut()
                }
                Entry::Vacant(e) => e.insert(handle),
            })
        }
    }

    pub fn drop_cf(&mut self, name: &str) -> Result<(), String> {
        let cf = self.cfs.remove(name);
        if cf.is_none() {
            return Err(format!("Invalid column family: {}", name));
        }

        unsafe {
            ffi_try!(crocksdb_drop_column_family(self.inner, cf.unwrap().inner));
        }

        Ok(())
    }

    pub fn cf_handle(&self, name: &str) -> Option<&CFHandle> {
        self.cfs.get(name)
    }

    /// get all column family names, including 'default'.
    pub fn cf_names(&self) -> Vec<&str> {
        self.cfs.iter().map(|(k, _)| k.as_str()).collect()
    }

    pub fn iter(&self) -> DBIterator<&DB> {
        let opts = ReadOptions::new();
        self.iter_opt(opts)
    }

    pub fn iter_opt(&self, opt: ReadOptions) -> DBIterator<&DB> {
        DBIterator::new(&self, opt)
    }

    pub fn iter_cf(&self, cf_handle: &CFHandle) -> DBIterator<&DB> {
        let opts = ReadOptions::new();
        DBIterator::new_cf(self, cf_handle, opts)
    }

    pub fn iter_cf_opt(&self, cf_handle: &CFHandle, opts: ReadOptions) -> DBIterator<&DB> {
        DBIterator::new_cf(self, cf_handle, opts)
    }

    pub fn snapshot(&self) -> Snapshot<&DB> {
        Snapshot::new(self)
    }

    pub unsafe fn unsafe_snap(&self) -> UnsafeSnap {
        UnsafeSnap::new(self.inner)
    }

    pub unsafe fn release_snap(&self, snap: &UnsafeSnap) {
        crocksdb_ffi::crocksdb_release_snapshot(self.inner, snap.get_inner())
    }

    pub fn put_opt(
        &self,
        key: &[u8],
        value: &[u8],
        writeopts: &WriteOptions,
    ) -> Result<(), String> {
        unsafe {
            ffi_try!(crocksdb_put(
                self.inner,
                writeopts.inner,
                key.as_ptr(),
                key.len() as size_t,
                value.as_ptr(),
                value.len() as size_t
            ));
            Ok(())
        }
    }

    pub fn put_cf_opt(
        &self,
        cf: &CFHandle,
        key: &[u8],
        value: &[u8],
        writeopts: &WriteOptions,
    ) -> Result<(), String> {
        unsafe {
            ffi_try!(crocksdb_put_cf(
                self.inner,
                writeopts.inner,
                cf.inner,
                key.as_ptr(),
                key.len() as size_t,
                value.as_ptr(),
                value.len() as size_t
            ));
            Ok(())
        }
    }
    pub fn merge_opt(
        &self,
        key: &[u8],
        value: &[u8],
        writeopts: &WriteOptions,
    ) -> Result<(), String> {
        unsafe {
            ffi_try!(crocksdb_merge(
                self.inner,
                writeopts.inner,
                key.as_ptr(),
                key.len() as size_t,
                value.as_ptr(),
                value.len() as size_t
            ));
            Ok(())
        }
    }
    fn merge_cf_opt(
        &self,
        cf: &CFHandle,
        key: &[u8],
        value: &[u8],
        writeopts: &WriteOptions,
    ) -> Result<(), String> {
        unsafe {
            ffi_try!(crocksdb_merge_cf(
                self.inner,
                writeopts.inner,
                cf.inner,
                key.as_ptr(),
                key.len() as size_t,
                value.as_ptr(),
                value.len() as size_t
            ));
            Ok(())
        }
    }
    fn delete_opt(&self, key: &[u8], writeopts: &WriteOptions) -> Result<(), String> {
        unsafe {
            ffi_try!(crocksdb_delete(
                self.inner,
                writeopts.inner,
                key.as_ptr(),
                key.len() as size_t
            ));
            Ok(())
        }
    }

    fn delete_cf_opt(
        &self,
        cf: &CFHandle,
        key: &[u8],
        writeopts: &WriteOptions,
    ) -> Result<(), String> {
        unsafe {
            ffi_try!(crocksdb_delete_cf(
                self.inner,
                writeopts.inner,
                cf.inner,
                key.as_ptr(),
                key.len() as size_t
            ));
            Ok(())
        }
    }

    fn single_delete_opt(&self, key: &[u8], writeopts: &WriteOptions) -> Result<(), String> {
        unsafe {
            ffi_try!(crocksdb_single_delete(
                self.inner,
                writeopts.inner,
                key.as_ptr(),
                key.len() as size_t
            ));
            Ok(())
        }
    }

    fn single_delete_cf_opt(
        &self,
        cf: &CFHandle,
        key: &[u8],
        writeopts: &WriteOptions,
    ) -> Result<(), String> {
        unsafe {
            ffi_try!(crocksdb_single_delete_cf(
                self.inner,
                writeopts.inner,
                cf.inner,
                key.as_ptr(),
                key.len() as size_t
            ));
            Ok(())
        }
    }

    fn delete_range_cf_opt(
        &self,
        cf: &CFHandle,
        begin_key: &[u8],
        end_key: &[u8],
        writeopts: &WriteOptions,
    ) -> Result<(), String> {
        unsafe {
            ffi_try!(crocksdb_delete_range_cf(
                self.inner,
                writeopts.inner,
                cf.inner,
                begin_key.as_ptr(),
                begin_key.len() as size_t,
                end_key.as_ptr(),
                end_key.len() as size_t
            ));
            Ok(())
        }
    }

    /// Flush all memtable data.
    /// If wait, the flush will wait until the flush is done.
    pub fn flush(&self, wait: bool) -> Result<(), String> {
        unsafe {
            let mut opts = FlushOptions::new();
            opts.set_wait(wait);
            ffi_try!(crocksdb_flush(self.inner, opts.inner));
            Ok(())
        }
    }

    /// Flush all memtable data for specified cf.
    /// If wait, the flush will wait until the flush is done.
    pub fn flush_cf(&self, cf: &CFHandle, wait: bool) -> Result<(), String> {
        unsafe {
            let mut opts = FlushOptions::new();
            opts.set_wait(wait);
            ffi_try!(crocksdb_flush_cf(self.inner, cf.inner, opts.inner));
            Ok(())
        }
    }

    /// Flushes multiple column families.
    /// If atomic flush is not enabled, flush_cfs is equivalent to
    /// calling flush_cf multiple times.
    /// If atomic flush is enabled, flush_cfs will flush all column families
    /// specified in `cfs` up to the latest sequence number at the time
    /// when flush is requested.
    pub fn flush_cfs(&self, cfs: &[&CFHandle], wait: bool) -> Result<(), String> {
        unsafe {
            let cfs: Vec<*mut _> = cfs.iter().map(|cf| cf.inner).collect();
            let mut opts = FlushOptions::new();
            opts.set_wait(wait);
            ffi_try!(crocksdb_flush_cfs(
                self.inner,
                cfs.as_ptr(),
                cfs.len(),
                opts.inner
            ));
            Ok(())
        }
    }

    /// Flush the WAL memory buffer to the file. If sync is true, it calls SyncWAL
    /// afterwards.
    pub fn flush_wal(&self, sync: bool) -> Result<(), String> {
        unsafe {
            ffi_try!(crocksdb_flush_wal(self.inner, sync));
            Ok(())
        }
    }

    /// Sync the wal. Note that Write() followed by SyncWAL() is not exactly the
    /// same as Write() with sync=true: in the latter case the changes won't be
    /// visible until the sync is done.
    /// Currently only works if allow_mmap_writes = false in Options.
    pub fn sync_wal(&self) -> Result<(), String> {
        unsafe {
            ffi_try!(crocksdb_sync_wal(self.inner));
            Ok(())
        }
    }

    /// Get the sequence number of the most recent transaction.
    pub fn get_latest_sequence_number(&self) -> u64 {
        unsafe { crocksdb_ffi::crocksdb_get_latest_sequence_number(self.inner) }
    }

    /// Return the approximate file system space used by keys in each ranges.
    ///
    /// Note that the returned sizes measure file system space usage, so
    /// if the user data compresses by a factor of ten, the returned
    /// sizes will be one-tenth the size of the corresponding user data size.
    ///
    /// Due to lack of abi, only data flushed to disk is taken into account.
    pub fn get_approximate_sizes(&self, ranges: &[Range]) -> Vec<u64> {
        self.get_approximate_sizes_cfopt(None, ranges)
    }

    pub fn get_approximate_sizes_cf(&self, cf: &CFHandle, ranges: &[Range]) -> Vec<u64> {
        self.get_approximate_sizes_cfopt(Some(cf), ranges)
    }

    fn get_approximate_sizes_cfopt(&self, cf: Option<&CFHandle>, ranges: &[Range]) -> Vec<u64> {
        let start_keys: Vec<*const u8> = ranges.iter().map(|x| x.start_key.as_ptr()).collect();
        let start_key_lens: Vec<_> = ranges.iter().map(|x| x.start_key.len()).collect();
        let end_keys: Vec<*const u8> = ranges.iter().map(|x| x.end_key.as_ptr()).collect();
        let end_key_lens: Vec<_> = ranges.iter().map(|x| x.end_key.len()).collect();
        let mut sizes: Vec<u64> = vec![0; ranges.len()];
        let (n, start_key_ptr, start_key_len_ptr, end_key_ptr, end_key_len_ptr, size_ptr) = (
            ranges.len() as i32,
            start_keys.as_ptr(),
            start_key_lens.as_ptr(),
            end_keys.as_ptr(),
            end_key_lens.as_ptr(),
            sizes.as_mut_ptr(),
        );
        match cf {
            None => unsafe {
                crocksdb_ffi::crocksdb_approximate_sizes(
                    self.inner,
                    n,
                    start_key_ptr,
                    start_key_len_ptr,
                    end_key_ptr,
                    end_key_len_ptr,
                    size_ptr,
                )
            },
            Some(cf) => unsafe {
                crocksdb_ffi::crocksdb_approximate_sizes_cf(
                    self.inner,
                    cf.inner,
                    n,
                    start_key_ptr,
                    start_key_len_ptr,
                    end_key_ptr,
                    end_key_len_ptr,
                    size_ptr,
                )
            },
        }
        sizes
    }

    // Return the approximate number of records and size in the range of memtables.
    pub fn get_approximate_memtable_stats(&self, range: &Range) -> (u64, u64) {
        let (mut count, mut size) = (0, 0);
        unsafe {
            crocksdb_ffi::crocksdb_approximate_memtable_stats(
                self.inner,
                range.start_key.as_ptr(),
                range.start_key.len(),
                range.end_key.as_ptr(),
                range.end_key.len(),
                &mut count,
                &mut size,
            );
        }
        (count, size)
    }

    // Return the approximate number of records and size in the range of memtables of the cf.
    pub fn get_approximate_memtable_stats_cf(&self, cf: &CFHandle, range: &Range) -> (u64, u64) {
        let (mut count, mut size) = (0, 0);
        unsafe {
            crocksdb_ffi::crocksdb_approximate_memtable_stats_cf(
                self.inner,
                cf.inner,
                range.start_key.as_ptr(),
                range.start_key.len(),
                range.end_key.as_ptr(),
                range.end_key.len(),
                &mut count,
                &mut size,
            );
        }
        (count, size)
    }

    pub fn compact_range(&self, start_key: Option<&[u8]>, end_key: Option<&[u8]>) {
        unsafe {
            let (start, s_len) = start_key.map_or((ptr::null(), 0), |k| (k.as_ptr(), k.len()));
            let (end, e_len) = end_key.map_or((ptr::null(), 0), |k| (k.as_ptr(), k.len()));
            crocksdb_ffi::crocksdb_compact_range(self.inner, start, s_len, end, e_len);
        }
    }

    pub fn compact_range_cf(
        &self,
        cf: &CFHandle,
        start_key: Option<&[u8]>,
        end_key: Option<&[u8]>,
    ) {
        unsafe {
            let (start, s_len) = start_key.map_or((ptr::null(), 0), |k| (k.as_ptr(), k.len()));
            let (end, e_len) = end_key.map_or((ptr::null(), 0), |k| (k.as_ptr(), k.len()));
            crocksdb_ffi::crocksdb_compact_range_cf(self.inner, cf.inner, start, s_len, end, e_len);
        }
    }

    pub fn compact_range_cf_opt(
        &self,
        cf: &CFHandle,
        compact_options: &CompactOptions,
        start_key: Option<&[u8]>,
        end_key: Option<&[u8]>,
    ) {
        unsafe {
            let (start, s_len) = start_key.map_or((ptr::null(), 0), |k| (k.as_ptr(), k.len()));
            let (end, e_len) = end_key.map_or((ptr::null(), 0), |k| (k.as_ptr(), k.len()));
            crocksdb_ffi::crocksdb_compact_range_cf_opt(
                self.inner,
                cf.inner,
                compact_options.inner,
                start,
                s_len,
                end,
                e_len,
            );
        }
    }

    pub fn delete_files_in_range(
        &self,
        start_key: &[u8],
        end_key: &[u8],
        include_end: bool,
    ) -> Result<(), String> {
        unsafe {
            if self.is_titan() {
                ffi_try!(ctitandb_delete_files_in_range(
                    self.inner,
                    start_key.as_ptr(),
                    start_key.len() as size_t,
                    end_key.as_ptr(),
                    end_key.len() as size_t,
                    include_end
                ));
            } else {
                ffi_try!(crocksdb_delete_files_in_range(
                    self.inner,
                    start_key.as_ptr(),
                    start_key.len() as size_t,
                    end_key.as_ptr(),
                    end_key.len() as size_t,
                    include_end
                ));
            }
            Ok(())
        }
    }

    pub fn delete_files_in_range_cf(
        &self,
        cf: &CFHandle,
        start_key: &[u8],
        end_key: &[u8],
        include_end: bool,
    ) -> Result<(), String> {
        unsafe {
            if self.is_titan() {
                ffi_try!(ctitandb_delete_files_in_range_cf(
                    self.inner,
                    cf.inner,
                    start_key.as_ptr(),
                    start_key.len() as size_t,
                    end_key.as_ptr(),
                    end_key.len() as size_t,
                    include_end
                ));
            } else {
                ffi_try!(crocksdb_delete_files_in_range_cf(
                    self.inner,
                    cf.inner,
                    start_key.as_ptr(),
                    start_key.len() as size_t,
                    end_key.as_ptr(),
                    end_key.len() as size_t,
                    include_end
                ));
            }
            Ok(())
        }
    }

    pub fn delete_files_in_ranges_cf(
        &self,
        cf: &CFHandle,
        ranges: &[Range],
        include_end: bool,
    ) -> Result<(), String> {
        let start_keys: Vec<*const u8> = ranges.iter().map(|x| x.start_key.as_ptr()).collect();
        let start_keys_lens: Vec<_> = ranges.iter().map(|x| x.start_key.len()).collect();
        let limit_keys: Vec<*const u8> = ranges.iter().map(|x| x.end_key.as_ptr()).collect();
        let limit_keys_lens: Vec<_> = ranges.iter().map(|x| x.end_key.len()).collect();
        unsafe {
            if self.is_titan() {
                ffi_try!(ctitandb_delete_files_in_ranges_cf(
                    self.inner,
                    cf.inner,
                    start_keys.as_ptr(),
                    start_keys_lens.as_ptr(),
                    limit_keys.as_ptr(),
                    limit_keys_lens.as_ptr(),
                    ranges.len(),
                    include_end
                ));
            } else {
                ffi_try!(crocksdb_delete_files_in_ranges_cf(
                    self.inner,
                    cf.inner,
                    start_keys.as_ptr(),
                    start_keys_lens.as_ptr(),
                    limit_keys.as_ptr(),
                    limit_keys_lens.as_ptr(),
                    ranges.len(),
                    include_end
                ));
            }
        }
        Ok(())
    }

    pub fn get_property_value(&self, name: &str) -> Option<String> {
        self.get_property_value_cf_opt(None, name)
    }

    pub fn get_property_value_cf(&self, cf: &CFHandle, name: &str) -> Option<String> {
        self.get_property_value_cf_opt(Some(cf), name)
    }

    /// Return the int property in rocksdb.
    /// Return None if the property not exists or not int type.
    pub fn get_property_int(&self, name: &str) -> Option<u64> {
        self.get_property_int_cf_opt(None, name)
    }

    pub fn get_property_int_cf(&self, cf: &CFHandle, name: &str) -> Option<u64> {
        self.get_property_int_cf_opt(Some(cf), name)
    }

    fn get_property_value_cf_opt(&self, cf: Option<&CFHandle>, name: &str) -> Option<String> {
        unsafe {
            let prop_name = CString::new(name).unwrap();

            let value = match cf {
                None => crocksdb_ffi::crocksdb_property_value(self.inner, prop_name.as_ptr()),
                Some(cf) => crocksdb_ffi::crocksdb_property_value_cf(
                    self.inner,
                    cf.inner,
                    prop_name.as_ptr(),
                ),
            };

            if value.is_null() {
                return None;
            }

            // Must valid UTF-8 format.
            let s = CStr::from_ptr(value).to_str().unwrap().to_owned();
            libc::free(value as *mut c_void);
            Some(s)
        }
    }

    fn get_property_int_cf_opt(&self, cf: Option<&CFHandle>, name: &str) -> Option<u64> {
        // Rocksdb guarantees that the return property int
        // value is u64 if exists.
        if let Some(value) = self.get_property_value_cf_opt(cf, name) {
            if let Ok(num) = value.as_str().parse::<u64>() {
                return Some(num);
            }
        }

        None
    }

    pub fn get_statistics(&self) -> Option<String> {
        self.opts.get_statistics()
    }

    pub fn reset_statistics(&self) {
        self.opts.reset_statistics();
    }

    pub fn get_statistics_ticker_count(&self, ticker_type: DBStatisticsTickerType) -> u64 {
        self.opts.get_statistics_ticker_count(ticker_type)
    }

    pub fn get_and_reset_statistics_ticker_count(
        &self,
        ticker_type: DBStatisticsTickerType,
    ) -> u64 {
        self.opts.get_and_reset_statistics_ticker_count(ticker_type)
    }

    pub fn get_statistics_histogram_string(
        &self,
        hist_type: DBStatisticsHistogramType,
    ) -> Option<String> {
        self.opts.get_statistics_histogram_string(hist_type)
    }

    pub fn get_statistics_histogram(
        &self,
        hist_type: DBStatisticsHistogramType,
    ) -> Option<HistogramData> {
        self.opts.get_statistics_histogram(hist_type)
    }

    pub fn get_db_options(&self) -> DBOptions {
        unsafe {
            let inner = crocksdb_ffi::crocksdb_get_db_options(self.inner);
            DBOptions::from_raw(inner)
        }
    }

    pub fn get_map_property_cf(&self, cf: &CFHandle, name: &str) -> Option<MapProperty> {
        unsafe {
            let info = MapProperty::new();
            let cname = CString::new(name.as_bytes()).unwrap();
            if !crocksdb_ffi::crocksdb_get_map_property_cf(
                self.inner,
                cf.inner,
                cname.as_ptr(),
                info.inner,
            ) {
                return None;
            }
            Some(info)
        }
    }

    pub fn set_db_options(&self, options: &[(&str, &str)]) -> Result<(), String> {
        unsafe {
            let name_strs: Vec<_> = options
                .iter()
                .map(|&(n, _)| CString::new(n.as_bytes()).unwrap())
                .collect();
            let name_ptrs: Vec<_> = name_strs.iter().map(|s| s.as_ptr()).collect();
            let value_strs: Vec<_> = options
                .iter()
                .map(|&(_, v)| CString::new(v.as_bytes()).unwrap())
                .collect();
            let value_ptrs: Vec<_> = value_strs.iter().map(|s| s.as_ptr()).collect();
            ffi_try!(crocksdb_set_db_options(
                self.inner,
                name_ptrs.as_ptr() as *const *const c_char,
                value_ptrs.as_ptr() as *const *const c_char,
                options.len() as size_t
            ));
            Ok(())
        }
    }

    pub fn get_options(&self) -> ColumnFamilyOptions {
        let cf = self.cf_handle("default").unwrap();
        unsafe {
            let inner = crocksdb_ffi::crocksdb_get_options_cf(self.inner, cf.inner);
            let titan_inner = if self.is_titan() {
                crocksdb_ffi::ctitandb_get_titan_options_cf(self.inner, cf.inner)
            } else {
                ptr::null_mut::<DBTitanDBOptions>()
            };
            ColumnFamilyOptions::from_raw(inner, titan_inner)
        }
    }

    pub fn get_options_cf(&self, cf: &CFHandle) -> ColumnFamilyOptions {
        unsafe {
            let inner = crocksdb_ffi::crocksdb_get_options_cf(self.inner, cf.inner);
            let titan_inner = if self.is_titan() {
                crocksdb_ffi::ctitandb_get_titan_options_cf(self.inner, cf.inner)
            } else {
                ptr::null_mut::<DBTitanDBOptions>()
            };
            ColumnFamilyOptions::from_raw(inner, titan_inner)
        }
    }

    pub fn set_options_cf(&self, cf: &CFHandle, options: &[(&str, &str)]) -> Result<(), String> {
        unsafe {
            let name_strs: Vec<_> = options
                .iter()
                .map(|&(n, _)| CString::new(n.as_bytes()).unwrap())
                .collect();
            let name_ptrs: Vec<_> = name_strs.iter().map(|s| s.as_ptr()).collect();
            let value_strs: Vec<_> = options
                .iter()
                .map(|&(_, v)| CString::new(v.as_bytes()).unwrap())
                .collect();
            let value_ptrs: Vec<_> = value_strs.iter().map(|s| s.as_ptr()).collect();
            ffi_try!(crocksdb_set_options_cf(
                self.inner,
                cf.inner,
                name_ptrs.as_ptr() as *const *const c_char,
                value_ptrs.as_ptr() as *const *const c_char,
                options.len() as size_t
            ));
            Ok(())
        }
    }

    pub fn ingest_external_file(
        &self,
        opt: &IngestExternalFileOptions,
        files: &[&str],
    ) -> Result<(), String> {
        let c_files = build_cstring_list(files);
        let c_files_ptrs: Vec<*const _> = c_files.iter().map(|s| s.as_ptr()).collect();
        unsafe {
            ffi_try!(crocksdb_ingest_external_file(
                self.inner,
                c_files_ptrs.as_ptr(),
                c_files.len(),
                opt.inner
            ));
        }
        Ok(())
    }

    pub fn ingest_external_file_cf(
        &self,
        cf: &CFHandle,
        opt: &IngestExternalFileOptions,
        files: &[&str],
    ) -> Result<(), String> {
        let c_files = build_cstring_list(files);
        let c_files_ptrs: Vec<*const _> = c_files.iter().map(|s| s.as_ptr()).collect();
        unsafe {
            ffi_try!(crocksdb_ingest_external_file_cf(
                self.inner,
                cf.inner,
                c_files_ptrs.as_ptr(),
                c_files_ptrs.len(),
                opt.inner
            ));
        }
        Ok(())
    }

    /// An optimized version of `ingest_external_file_cf`. It will
    /// first try to ingest files without blocking and fallback to a
    /// blocking ingestion if the optimization fails.
    /// Returns true if a memtable is flushed without blocking.
    pub fn ingest_external_file_optimized(
        &self,
        cf: &CFHandle,
        opt: &IngestExternalFileOptions,
        files: &[&str],
    ) -> Result<bool, String> {
        let c_files = build_cstring_list(files);
        let c_files_ptrs: Vec<*const _> = c_files.iter().map(|s| s.as_ptr()).collect();
        let has_flush = unsafe {
            ffi_try!(crocksdb_ingest_external_file_optimized(
                self.inner,
                cf.inner,
                c_files_ptrs.as_ptr(),
                c_files_ptrs.len(),
                opt.inner
            ))
        };
        Ok(has_flush)
    }

    pub fn backup_at(&self, path: &str) -> Result<BackupEngine, String> {
        let backup_engine = BackupEngine::open(DBOptions::new(), path).unwrap();
        unsafe {
            ffi_try!(crocksdb_backup_engine_create_new_backup(
                backup_engine.inner,
                self.inner
            ))
        }
        Ok(backup_engine)
    }

    pub fn restore_from(
        backup_engine: &BackupEngine,
        restore_db_path: &str,
        restore_wal_path: &str,
        ropts: &RestoreOptions,
    ) -> Result<DB, String> {
        let c_db_path = match CString::new(restore_db_path.as_bytes()) {
            Ok(c) => c,
            Err(_) => {
                return Err(
                    "Failed to convert restore_db_path to CString when restoring rocksdb"
                        .to_owned(),
                );
            }
        };

        let c_wal_path = match CString::new(restore_wal_path.as_bytes()) {
            Ok(c) => c,
            Err(_) => {
                return Err(
                    "Failed to convert restore_wal_path to CString when restoring rocksdb"
                        .to_owned(),
                );
            }
        };

        unsafe {
            ffi_try!(crocksdb_backup_engine_restore_db_from_latest_backup(
                backup_engine.inner,
                c_db_path.as_ptr(),
                c_wal_path.as_ptr(),
                ropts.inner
            ))
        };

        DB::open_default(restore_db_path)
    }

    pub fn get_block_cache_usage(&self) -> u64 {
        self.get_options().get_block_cache_usage()
    }

    pub fn get_block_cache_usage_cf(&self, cf: &CFHandle) -> u64 {
        self.get_options_cf(cf).get_block_cache_usage()
    }

    pub fn get_blob_cache_usage(&self) -> u64 {
        self.get_options().get_blob_cache_usage()
    }

    pub fn get_blob_cache_usage_cf(&self, cf: &CFHandle) -> u64 {
        self.get_options_cf(cf).get_blob_cache_usage()
    }

    pub fn get_properties_of_all_tables(&self) -> Result<TablePropertiesCollection, String> {
        unsafe {
            let props = ffi_try!(crocksdb_get_properties_of_all_tables(self.inner));
            Ok(TablePropertiesCollection::from_raw(props))
        }
    }

    pub fn get_properties_of_all_tables_rc(&self) -> Result<RcTablePropertiesCollection, String> {
        unsafe {
            let props = ffi_try!(crocksdb_get_properties_of_all_tables(self.inner));
            Ok(RcTablePropertiesCollection::new(props))
        }
    }

    pub fn get_properties_of_all_tables_cf(
        &self,
        cf: &CFHandle,
    ) -> Result<TablePropertiesCollection, String> {
        unsafe {
            let props = ffi_try!(crocksdb_get_properties_of_all_tables_cf(
                self.inner, cf.inner
            ));
            Ok(TablePropertiesCollection::from_raw(props))
        }
    }

    pub fn get_properties_of_tables_in_range(
        &self,
        cf: &CFHandle,
        ranges: &[Range],
    ) -> Result<TablePropertiesCollection, String> {
        // Safety: transfers ownership of new non-null pointer
        unsafe {
            let props = self.get_properties_of_tables_in_range_common(cf, ranges)?;
            Ok(TablePropertiesCollection::from_raw(props))
        }
    }

    /// Like `get_properties_of_table_in_range` but the returned family
    /// of types don't contain any lifetimes. This is suitable for wrapping
    /// in further abstractions without needing abstract associated lifetime
    /// parameters. Used by tikv's `engine_rocks`.
    pub fn get_properties_of_tables_in_range_rc(
        &self,
        cf: &CFHandle,
        ranges: &[Range],
    ) -> Result<RcTablePropertiesCollection, String> {
        // Safety: transfers ownership of new non-null pointer
        unsafe {
            let props = self.get_properties_of_tables_in_range_common(cf, ranges)?;
            Ok(RcTablePropertiesCollection::new(props))
        }
    }

    fn get_properties_of_tables_in_range_common(
        &self,
        cf: &CFHandle,
        ranges: &[Range],
    ) -> Result<*mut DBTablePropertiesCollection, String> {
        let start_keys: Vec<*const u8> = ranges.iter().map(|x| x.start_key.as_ptr()).collect();
        let start_keys_lens: Vec<_> = ranges.iter().map(|x| x.start_key.len()).collect();
        let limit_keys: Vec<*const u8> = ranges.iter().map(|x| x.end_key.as_ptr()).collect();
        let limit_keys_lens: Vec<_> = ranges.iter().map(|x| x.end_key.len()).collect();
        unsafe {
            let props = ffi_try!(crocksdb_get_properties_of_tables_in_range(
                self.inner,
                cf.inner,
                ranges.len() as i32,
                start_keys.as_ptr(),
                start_keys_lens.as_ptr(),
                limit_keys.as_ptr(),
                limit_keys_lens.as_ptr()
            ));
            Ok(props)
        }
    }

    pub fn get_all_key_versions(
        &self,
        start_key: &[u8],
        end_key: &[u8],
    ) -> Result<Vec<KeyVersion>, String> {
        unsafe {
            let kvs = ffi_try!(crocksdb_get_all_key_versions(
                self.inner,
                start_key.as_ptr(),
                start_key.len() as size_t,
                end_key.as_ptr(),
                end_key.len() as size_t
            ));
            let size = crocksdb_ffi::crocksdb_keyversions_count(kvs) as usize;
            let mut key_versions = Vec::with_capacity(size);
            for i in 0..size {
                key_versions.push(KeyVersion {
                    key: CStr::from_ptr(crocksdb_ffi::crocksdb_keyversions_key(kvs, i))
                        .to_string_lossy()
                        .into_owned(),
                    value: CStr::from_ptr(crocksdb_ffi::crocksdb_keyversions_value(kvs, i))
                        .to_string_lossy()
                        .into_owned(),
                    seq: crocksdb_ffi::crocksdb_keyversions_seq(kvs, i),
                    value_type: crocksdb_ffi::crocksdb_keyversions_type(kvs, i),
                })
            }
            crocksdb_ffi::crocksdb_keyversions_destroy(kvs);
            Ok(key_versions)
        }
    }

    pub fn get_column_family_meta_data(&self, cf: &CFHandle) -> ColumnFamilyMetaData {
        unsafe {
            let inner = crocksdb_ffi::crocksdb_column_family_meta_data_create();
            crocksdb_ffi::crocksdb_get_column_family_meta_data(self.inner, cf.inner, inner);
            ColumnFamilyMetaData::from_ptr(inner)
        }
    }

    pub fn compact_files_cf(
        &self,
        cf: &CFHandle,
        opts: &CompactionOptions,
        input_files: &[String],
        output_level: i32,
    ) -> Result<(), String> {
        unsafe {
            let input_file_cstrs: Vec<_> = input_files
                .iter()
                .map(|s| CString::new(s.as_bytes()).unwrap())
                .collect();
            let input_file_names: Vec<_> = input_file_cstrs.iter().map(|s| s.as_ptr()).collect();
            ffi_try!(crocksdb_compact_files_cf(
                self.inner,
                cf.inner,
                opts.inner,
                input_file_names.as_ptr() as *const *const c_char,
                input_file_names.len(),
                output_level
            ));
            Ok(())
        }
    }
}

impl Writable for DB {
    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), String> {
        self.put_opt(key, value, &WriteOptions::new())
    }

    fn put_cf(&self, cf: &CFHandle, key: &[u8], value: &[u8]) -> Result<(), String> {
        self.put_cf_opt(cf, key, value, &WriteOptions::new())
    }

    fn merge(&self, key: &[u8], value: &[u8]) -> Result<(), String> {
        self.merge_opt(key, value, &WriteOptions::new())
    }

    fn merge_cf(&self, cf: &CFHandle, key: &[u8], value: &[u8]) -> Result<(), String> {
        self.merge_cf_opt(cf, key, value, &WriteOptions::new())
    }

    fn delete(&self, key: &[u8]) -> Result<(), String> {
        self.delete_opt(key, &WriteOptions::new())
    }

    fn delete_cf(&self, cf: &CFHandle, key: &[u8]) -> Result<(), String> {
        self.delete_cf_opt(cf, key, &WriteOptions::new())
    }

    fn single_delete(&self, key: &[u8]) -> Result<(), String> {
        self.single_delete_opt(key, &WriteOptions::new())
    }

    fn single_delete_cf(&self, cf: &CFHandle, key: &[u8]) -> Result<(), String> {
        self.single_delete_cf_opt(cf, key, &WriteOptions::new())
    }

    fn delete_range(&self, begin_key: &[u8], end_key: &[u8]) -> Result<(), String> {
        let handle = self.cf_handle("default").unwrap();
        self.delete_range_cf(handle, begin_key, end_key)
    }

    fn delete_range_cf(
        &self,
        cf: &CFHandle,
        begin_key: &[u8],
        end_key: &[u8],
    ) -> Result<(), String> {
        self.delete_range_cf_opt(cf, begin_key, end_key, &WriteOptions::new())
    }
}

impl Default for WriteBatch {
    fn default() -> WriteBatch {
        WriteBatch {
            inner: unsafe { crocksdb_ffi::crocksdb_writebatch_create() },
        }
    }
}

impl WriteBatch {
    pub fn new() -> WriteBatch {
        WriteBatch::default()
    }

    pub fn with_capacity(cap: usize) -> WriteBatch {
        WriteBatch {
            inner: unsafe { crocksdb_ffi::crocksdb_writebatch_create_with_capacity(cap) },
        }
    }

    pub fn count(&self) -> usize {
        unsafe { crocksdb_ffi::crocksdb_writebatch_count(self.inner) as usize }
    }

    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    pub fn data_size(&self) -> usize {
        unsafe {
            let mut data_size: usize = 0;
            let _ = crocksdb_ffi::crocksdb_writebatch_data(self.inner, &mut data_size);
            return data_size;
        }
    }

    pub fn clear(&self) {
        unsafe {
            crocksdb_ffi::crocksdb_writebatch_clear(self.inner);
        }
    }

    pub fn set_save_point(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_writebatch_set_save_point(self.inner);
        }
    }

    pub fn rollback_to_save_point(&mut self) -> Result<(), String> {
        unsafe {
            ffi_try!(crocksdb_writebatch_rollback_to_save_point(self.inner));
        }
        Ok(())
    }

    pub fn pop_save_point(&mut self) -> Result<(), String> {
        unsafe {
            ffi_try!(crocksdb_writebatch_pop_save_point(self.inner));
        }
        Ok(())
    }
}

impl Drop for WriteBatch {
    fn drop(&mut self) {
        unsafe { crocksdb_ffi::crocksdb_writebatch_destroy(self.inner) }
    }
}

impl Drop for DB {
    fn drop(&mut self) {
        // SyncWAL before call close.
        if !self.readonly {
            // DB::SyncWal requires writable file support thread safe sync, but
            // not all types of env can create writable file that support thread
            // safe sync. eg, MemEnv.
            self.sync_wal().unwrap_or_else(|_| {});
        }
        unsafe {
            self.cfs.clear();
            crocksdb_ffi::crocksdb_close(self.inner);
        }
    }
}

impl Writable for WriteBatch {
    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), String> {
        unsafe {
            crocksdb_ffi::crocksdb_writebatch_put(
                self.inner,
                key.as_ptr(),
                key.len() as size_t,
                value.as_ptr(),
                value.len() as size_t,
            );
            Ok(())
        }
    }

    fn put_cf(&self, cf: &CFHandle, key: &[u8], value: &[u8]) -> Result<(), String> {
        unsafe {
            crocksdb_ffi::crocksdb_writebatch_put_cf(
                self.inner,
                cf.inner,
                key.as_ptr(),
                key.len() as size_t,
                value.as_ptr(),
                value.len() as size_t,
            );
            Ok(())
        }
    }

    fn merge(&self, key: &[u8], value: &[u8]) -> Result<(), String> {
        unsafe {
            crocksdb_ffi::crocksdb_writebatch_merge(
                self.inner,
                key.as_ptr(),
                key.len() as size_t,
                value.as_ptr(),
                value.len() as size_t,
            );
            Ok(())
        }
    }

    fn merge_cf(&self, cf: &CFHandle, key: &[u8], value: &[u8]) -> Result<(), String> {
        unsafe {
            crocksdb_ffi::crocksdb_writebatch_merge_cf(
                self.inner,
                cf.inner,
                key.as_ptr(),
                key.len() as size_t,
                value.as_ptr(),
                value.len() as size_t,
            );
            Ok(())
        }
    }

    fn delete(&self, key: &[u8]) -> Result<(), String> {
        unsafe {
            crocksdb_ffi::crocksdb_writebatch_delete(self.inner, key.as_ptr(), key.len() as size_t);
            Ok(())
        }
    }

    fn delete_cf(&self, cf: &CFHandle, key: &[u8]) -> Result<(), String> {
        unsafe {
            crocksdb_ffi::crocksdb_writebatch_delete_cf(
                self.inner,
                cf.inner,
                key.as_ptr(),
                key.len() as size_t,
            );
            Ok(())
        }
    }

    fn single_delete(&self, key: &[u8]) -> Result<(), String> {
        unsafe {
            crocksdb_ffi::crocksdb_writebatch_single_delete(
                self.inner,
                key.as_ptr(),
                key.len() as size_t,
            );
            Ok(())
        }
    }

    fn single_delete_cf(&self, cf: &CFHandle, key: &[u8]) -> Result<(), String> {
        unsafe {
            crocksdb_ffi::crocksdb_writebatch_single_delete_cf(
                self.inner,
                cf.inner,
                key.as_ptr(),
                key.len() as size_t,
            );
            Ok(())
        }
    }

    fn delete_range(&self, begin_key: &[u8], end_key: &[u8]) -> Result<(), String> {
        unsafe {
            crocksdb_ffi::crocksdb_writebatch_delete_range(
                self.inner,
                begin_key.as_ptr(),
                begin_key.len(),
                end_key.as_ptr(),
                end_key.len(),
            );
            Ok(())
        }
    }

    fn delete_range_cf(
        &self,
        cf: &CFHandle,
        begin_key: &[u8],
        end_key: &[u8],
    ) -> Result<(), String> {
        unsafe {
            crocksdb_ffi::crocksdb_writebatch_delete_range_cf(
                self.inner,
                cf.inner,
                begin_key.as_ptr(),
                begin_key.len(),
                end_key.as_ptr(),
                end_key.len(),
            );
            Ok(())
        }
    }
}

pub struct DBVector {
    pinned_slice: *mut DBPinnableSlice,
}

impl Debug for DBVector {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "{:?}", &**self)
    }
}

impl<'a> PartialEq<&'a [u8]> for DBVector {
    fn eq(&self, rhs: &&[u8]) -> bool {
        **rhs == **self
    }
}

impl Deref for DBVector {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        let mut val_len: size_t = 0;
        let val_len_ptr = &mut val_len as *mut size_t;
        unsafe {
            let val = crocksdb_ffi::crocksdb_pinnableslice_value(self.pinned_slice, val_len_ptr);
            slice::from_raw_parts(val, val_len)
        }
    }
}

impl Drop for DBVector {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_pinnableslice_destroy(self.pinned_slice);
        }
    }
}

impl DBVector {
    pub fn from_pinned_slice(s: *mut DBPinnableSlice) -> DBVector {
        DBVector { pinned_slice: s }
    }

    pub fn to_utf8(&self) -> Option<&str> {
        from_utf8(self.deref()).ok()
    }
}

pub struct BackupEngine {
    inner: *mut DBBackupEngine,
}

impl BackupEngine {
    pub fn open(opts: DBOptions, path: &str) -> Result<BackupEngine, String> {
        let cpath = match CString::new(path.as_bytes()) {
            Ok(c) => c,
            Err(_) => {
                return Err(
                    "Failed to convert path to CString when opening rocksdb backup engine"
                        .to_owned(),
                );
            }
        };

        if let Err(e) = fs::create_dir_all(path) {
            return Err(format!(
                "Failed to create rocksdb backup directory: {:?}",
                e
            ));
        }

        let backup_engine =
            unsafe { ffi_try!(crocksdb_backup_engine_open(opts.inner, cpath.as_ptr())) };

        Ok(BackupEngine {
            inner: backup_engine,
        })
    }
}

impl Drop for BackupEngine {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_backup_engine_close(self.inner);
        }
    }
}

// SstFileReader is used to read sst files that are generated by DB or
// SstFileWriter.
pub struct SstFileReader {
    inner: *mut crocksdb_ffi::SstFileReader,
    _opt: ColumnFamilyOptions,
}

unsafe impl Send for SstFileReader {}

impl SstFileReader {
    /// Creates a new SstFileReader.
    pub fn new(opt: ColumnFamilyOptions) -> Self {
        unsafe {
            Self {
                inner: crocksdb_ffi::crocksdb_sstfilereader_create(opt.inner),
                _opt: opt,
            }
        }
    }

    /// Opens a local SST file for reading.
    pub fn open(&mut self, name: &str) -> Result<(), String> {
        let path =
            CString::new(name.to_owned()).map_err(|e| format!("invalid path {}: {:?}", name, e))?;
        unsafe {
            ffi_try!(crocksdb_sstfilereader_open(self.inner, path.as_ptr()));
        }
        Ok(())
    }

    pub fn iter(&self) -> DBIterator<&Self> {
        self.iter_opt(ReadOptions::new())
    }

    pub fn iter_opt(&self, readopts: ReadOptions) -> DBIterator<&Self> {
        unsafe {
            DBIterator {
                inner: crocksdb_ffi::crocksdb_sstfilereader_new_iterator(
                    self.inner,
                    readopts.get_inner(),
                ),
                _db: self,
                _readopts: readopts,
            }
        }
    }

    /// Create an iterator out of a reference-counted SstFileReader.
    ///
    /// This exists due to restrictions on lifetimes in associated types.
    /// See RocksSstIterator in TiKV's engine_rocks.
    pub fn iter_rc(this: Rc<Self>) -> DBIterator<Rc<Self>> {
        Self::iter_opt_rc(this, ReadOptions::new())
    }

    pub fn iter_opt_rc(this: Rc<Self>, readopts: ReadOptions) -> DBIterator<Rc<Self>> {
        unsafe {
            DBIterator {
                inner: crocksdb_ffi::crocksdb_sstfilereader_new_iterator(
                    this.inner,
                    readopts.get_inner(),
                ),
                _db: this,
                _readopts: readopts,
            }
        }
    }

    pub fn read_table_properties<F: FnOnce(&TableProperties)>(&self, mut action: F) {
        extern "C" fn callback<F: FnOnce(&TableProperties)>(
            ctx: *mut c_void,
            ptr: *const crocksdb_ffi::DBTableProperties,
        ) {
            unsafe {
                let caller = ptr::read(ctx as *mut F);
                caller(TableProperties::from_ptr(ptr));
            }
        }

        unsafe {
            crocksdb_ffi::crocksdb_sstfilereader_read_table_properties(
                self.inner,
                &mut action as *mut F as *mut c_void,
                callback::<F>,
            );
            mem::forget(action);
        }
    }

    pub fn verify_checksum(&self) -> Result<(), String> {
        unsafe { ffi_try!(crocksdb_sstfilereader_verify_checksum(self.inner)) };
        Ok(())
    }
}

impl Drop for SstFileReader {
    fn drop(&mut self) {
        unsafe { crocksdb_ffi::crocksdb_sstfilereader_destroy(self.inner) }
    }
}

/// SstFileWriter is used to create sst files that can be added to database later
/// All keys in files generated by SstFileWriter will have sequence number = 0
pub struct SstFileWriter {
    inner: *mut crocksdb_ffi::SstFileWriter,
    _env_opt: EnvOptions,
    _opt: ColumnFamilyOptions,
}

unsafe impl Send for SstFileWriter {}

impl SstFileWriter {
    pub fn new(env_opt: EnvOptions, opt: ColumnFamilyOptions) -> SstFileWriter {
        unsafe {
            SstFileWriter {
                inner: crocksdb_ffi::crocksdb_sstfilewriter_create(env_opt.inner, opt.inner),
                _env_opt: env_opt,
                _opt: opt,
            }
        }
    }

    pub fn new_cf(env_opt: EnvOptions, opt: ColumnFamilyOptions, cf: &CFHandle) -> SstFileWriter {
        unsafe {
            SstFileWriter {
                inner: crocksdb_ffi::crocksdb_sstfilewriter_create_cf(
                    env_opt.inner,
                    opt.inner,
                    cf.inner,
                ),
                _env_opt: env_opt,
                _opt: opt,
            }
        }
    }

    /// Prepare SstFileWriter to write into file located at "file_path".
    pub fn open(&mut self, name: &str) -> Result<(), String> {
        let path = match CString::new(name.to_owned()) {
            Err(e) => return Err(format!("invalid path {}: {:?}", name, e)),
            Ok(p) => p,
        };
        unsafe {
            ffi_try!(crocksdb_sstfilewriter_open(self.inner, path.as_ptr()));
        }
        Ok(())
    }

    /// Add key, value to currently opened file
    /// REQUIRES: key is after any previously added key according to comparator.
    pub fn put(&mut self, key: &[u8], val: &[u8]) -> Result<(), String> {
        unsafe {
            ffi_try!(crocksdb_sstfilewriter_put(
                self.inner,
                key.as_ptr(),
                key.len(),
                val.as_ptr(),
                val.len()
            ));
            Ok(())
        }
    }

    pub fn merge(&mut self, key: &[u8], val: &[u8]) -> Result<(), String> {
        unsafe {
            ffi_try!(crocksdb_sstfilewriter_merge(
                self.inner,
                key.as_ptr(),
                key.len(),
                val.as_ptr(),
                val.len()
            ));
            Ok(())
        }
    }

    pub fn delete(&mut self, key: &[u8]) -> Result<(), String> {
        unsafe {
            ffi_try!(crocksdb_sstfilewriter_delete(
                self.inner,
                key.as_ptr(),
                key.len()
            ));
            Ok(())
        }
    }

    pub fn delete_range(&mut self, begin_key: &[u8], end_key: &[u8]) -> Result<(), String> {
        unsafe {
            ffi_try!(crocksdb_sstfilewriter_delete_range(
                self.inner,
                begin_key.as_ptr(),
                begin_key.len(),
                end_key.as_ptr(),
                end_key.len()
            ));
            Ok(())
        }
    }

    /// Finalize writing to sst file and close file.
    pub fn finish(&mut self) -> Result<ExternalSstFileInfo, String> {
        let info = ExternalSstFileInfo::new();
        unsafe {
            ffi_try!(crocksdb_sstfilewriter_finish(self.inner, info.inner));
        }
        Ok(info)
    }

    pub fn file_size(&mut self) -> u64 {
        unsafe { crocksdb_ffi::crocksdb_sstfilewriter_file_size(self.inner) as u64 }
    }
}

impl Drop for SstFileWriter {
    fn drop(&mut self) {
        unsafe { crocksdb_ffi::crocksdb_sstfilewriter_destroy(self.inner) }
    }
}

pub struct ExternalSstFileInfo {
    inner: *mut crocksdb_ffi::ExternalSstFileInfo,
}

impl ExternalSstFileInfo {
    pub fn new() -> ExternalSstFileInfo {
        unsafe {
            ExternalSstFileInfo {
                inner: crocksdb_ffi::crocksdb_externalsstfileinfo_create(),
            }
        }
    }

    pub fn file_path(&self) -> PathBuf {
        let mut len: size_t = 0;
        unsafe {
            let ptr = crocksdb_ffi::crocksdb_externalsstfileinfo_file_path(self.inner, &mut len);
            let bytes = slice::from_raw_parts(ptr, len as usize);
            PathBuf::from(String::from_utf8(bytes.to_owned()).unwrap())
        }
    }

    pub fn smallest_key(&self) -> &[u8] {
        let mut len: size_t = 0;
        unsafe {
            let ptr = crocksdb_ffi::crocksdb_externalsstfileinfo_smallest_key(self.inner, &mut len);
            slice::from_raw_parts(ptr, len as usize)
        }
    }

    pub fn largest_key(&self) -> &[u8] {
        let mut len: size_t = 0;
        unsafe {
            let ptr = crocksdb_ffi::crocksdb_externalsstfileinfo_largest_key(self.inner, &mut len);
            slice::from_raw_parts(ptr, len as usize)
        }
    }

    pub fn sequence_number(&self) -> u64 {
        unsafe { crocksdb_ffi::crocksdb_externalsstfileinfo_sequence_number(self.inner) as u64 }
    }

    pub fn file_size(&self) -> u64 {
        unsafe { crocksdb_ffi::crocksdb_externalsstfileinfo_file_size(self.inner) as u64 }
    }

    pub fn num_entries(&self) -> u64 {
        unsafe { crocksdb_ffi::crocksdb_externalsstfileinfo_num_entries(self.inner) as u64 }
    }
}

impl Drop for ExternalSstFileInfo {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_externalsstfileinfo_destroy(self.inner);
        }
    }
}

pub fn supported_compression() -> Vec<DBCompressionType> {
    unsafe {
        let size = crocksdb_ffi::crocksdb_get_supported_compression_number() as usize;
        let mut v: Vec<DBCompressionType> = Vec::with_capacity(size);
        let pv = v.as_mut_ptr();
        crocksdb_ffi::crocksdb_get_supported_compression(pv, size as size_t);
        v.set_len(size);
        v
    }
}

pub struct Env {
    pub inner: *mut DBEnv,
    #[allow(dead_code)]
    base: Option<Arc<Env>>,
}

unsafe impl Send for Env {}

unsafe impl Sync for Env {}

impl Default for Env {
    fn default() -> Env {
        unsafe {
            Env {
                inner: crocksdb_ffi::crocksdb_default_env_create(),
                base: None,
            }
        }
    }
}

impl Env {
    pub fn new_mem() -> Env {
        unsafe {
            Env {
                inner: crocksdb_ffi::crocksdb_mem_env_create(),
                base: None,
            }
        }
    }

    // Create a ctr encrypted env with a given base env and a given ciper text.
    // The length of ciper text must be 2^n, and must be less or equal to 2048.
    // The recommanded block size are 1024, 512 and 256.
    pub fn new_ctr_encrypted_env(base_env: Arc<Env>, ciphertext: &[u8]) -> Result<Env, String> {
        let len = ciphertext.len();
        if len > 2048 || !len.is_power_of_two() {
            return Err(
                "ciphertext length must be less or equal to 2048, and must be power of 2"
                    .to_owned(),
            );
        }
        let env = unsafe {
            crocksdb_ffi::crocksdb_ctr_encrypted_env_create(
                base_env.inner,
                ciphertext.as_ptr() as *const c_char,
                len,
            )
        };
        Ok(Env {
            inner: env,
            base: Some(base_env),
        })
    }

    // Create a ctr encrypted env with the default env
    pub fn new_default_ctr_encrypted_env(ciphertext: &[u8]) -> Result<Env, String> {
        Env::new_ctr_encrypted_env(Arc::new(Env::default()), ciphertext)
    }

    // Create an encrypted env that accepts an external key manager.
    #[cfg(feature = "encryption")]
    pub fn new_key_managed_encrypted_env(
        base_env: Arc<Env>,
        key_manager: Arc<dyn EncryptionKeyManager>,
    ) -> Result<Env, String> {
        let db_key_manager = DBEncryptionKeyManager::new(key_manager);
        let env = unsafe {
            crocksdb_ffi::crocksdb_key_managed_encrypted_env_create(
                base_env.inner,
                db_key_manager.inner,
            )
        };
        Ok(Env {
            inner: env,
            base: Some(base_env),
        })
    }

    pub fn new_sequential_file(
        &self,
        path: &str,
        opts: EnvOptions,
    ) -> Result<SequentialFile, String> {
        unsafe {
            let file_path = CString::new(path).unwrap();
            let file = ffi_try!(crocksdb_sequential_file_create(
                self.inner,
                file_path.as_ptr(),
                opts.inner
            ));
            Ok(SequentialFile::new(file))
        }
    }

    pub fn file_exists(&self, path: &str) -> Result<(), String> {
        unsafe {
            let file_path = CString::new(path).unwrap();
            ffi_try!(crocksdb_env_file_exists(self.inner, file_path.as_ptr()));
            Ok(())
        }
    }

    pub fn delete_file(&self, path: &str) -> Result<(), String> {
        unsafe {
            let file_path = CString::new(path).unwrap();
            ffi_try!(crocksdb_env_delete_file(self.inner, file_path.as_ptr()));
            Ok(())
        }
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_env_destroy(self.inner);
        }
    }
}

pub struct SequentialFile {
    inner: *mut DBSequentialFile,
}

impl SequentialFile {
    fn new(inner: *mut DBSequentialFile) -> SequentialFile {
        SequentialFile { inner: inner }
    }

    pub fn skip(&mut self, n: usize) -> Result<(), String> {
        unsafe {
            ffi_try!(crocksdb_sequential_file_skip(self.inner, n as size_t));
            Ok(())
        }
    }
}

unsafe impl Send for SequentialFile {}

impl io::Read for SequentialFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        unsafe {
            let mut err = ptr::null_mut();
            let size = crocksdb_ffi::crocksdb_sequential_file_read(
                self.inner,
                buf.len() as size_t,
                buf.as_mut_ptr(),
                &mut err,
            );
            if !err.is_null() {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    crocksdb_ffi::error_message(err),
                ));
            }
            Ok(size as usize)
        }
    }
}

impl Drop for SequentialFile {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_sequential_file_destroy(self.inner);
        }
    }
}

pub struct Cache {
    pub inner: *mut DBCache,
}

impl Cache {
    pub fn new_lru_cache(opt: LRUCacheOptions) -> Cache {
        // This is ok because LRUCacheOptions always contains a valid pointer
        unsafe {
            Cache {
                inner: crocksdb_ffi::new_lru_cache(opt.inner),
            }
        }
    }
}

impl Drop for Cache {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_cache_destroy(self.inner);
        }
    }
}

pub struct MemoryAllocator {
    pub inner: *mut DBMemoryAllocator,
}

impl MemoryAllocator {
    #[cfg(feature = "jemalloc")]
    pub fn new_jemalloc_memory_allocator() -> Result<MemoryAllocator, String> {
        unsafe {
            let allocator = MemoryAllocator {
                inner: ffi_try!(crocksdb_jemalloc_nodump_allocator_create()),
            };
            Ok(allocator)
        }
    }
}

impl Drop for MemoryAllocator {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_memory_allocator_destroy(self.inner);
        }
    }
}

pub fn set_external_sst_file_global_seq_no(
    db: &DB,
    cf: &CFHandle,
    file: &str,
    seq_no: u64,
) -> Result<u64, String> {
    let cfile = CString::new(file).unwrap();
    unsafe {
        let pre_seq_no = ffi_try!(crocksdb_set_external_sst_file_global_seq_no(
            db.inner,
            cf.inner,
            cfile.as_ptr(),
            seq_no
        ));
        Ok(pre_seq_no)
    }
}

pub fn load_latest_options(
    dbpath: &str,
    env: &Env,
    ignore_unknown_options: bool,
) -> Result<Option<(DBOptions, Vec<CColumnFamilyDescriptor>)>, String> {
    const ERR_CONVERT_PATH: &str = "Failed to convert path to CString when load latest options";

    let dbpath = CString::new(dbpath.as_bytes()).map_err(|_| ERR_CONVERT_PATH.to_owned())?;
    let db_options = DBOptions::new();
    unsafe {
        let raw_cf_descs: *mut *mut crocksdb_ffi::ColumnFamilyDescriptor = ptr::null_mut();
        let mut cf_descs_len: size_t = 0;

        let ok = ffi_try!(crocksdb_load_latest_options(
            dbpath.as_ptr(),
            env.inner,
            db_options.inner,
            &raw_cf_descs,
            &mut cf_descs_len,
            ignore_unknown_options
        ));
        if !ok {
            return Ok(None);
        }
        let cf_descs_list = slice::from_raw_parts(raw_cf_descs, cf_descs_len);
        let cf_descs = cf_descs_list
            .iter()
            .map(|raw_cf_desc| CColumnFamilyDescriptor::from_raw(*raw_cf_desc))
            .collect();

        libc::free(raw_cf_descs as *mut c_void);

        Ok(Some((db_options, cf_descs)))
    }
}

pub fn run_ldb_tool(ldb_args: &[String], opts: &DBOptions) {
    unsafe {
        let ldb_args_cstrs: Vec<_> = ldb_args
            .iter()
            .map(|s| CString::new(s.as_bytes()).unwrap())
            .collect();
        let args: Vec<_> = ldb_args_cstrs.iter().map(|s| s.as_ptr()).collect();
        crocksdb_ffi::crocksdb_run_ldb_tool(
            args.len() as i32,
            args.as_ptr() as *const *const c_char,
            opts.inner,
        );
    }
}

pub fn run_sst_dump_tool(sst_dump_args: &[String], opts: &DBOptions) {
    unsafe {
        let sst_dump_args_cstrs: Vec<_> = sst_dump_args
            .iter()
            .map(|s| CString::new(s.as_bytes()).unwrap())
            .collect();
        let args: Vec<_> = sst_dump_args_cstrs.iter().map(|s| s.as_ptr()).collect();
        crocksdb_ffi::crocksdb_run_sst_dump_tool(
            args.len() as i32,
            args.as_ptr() as *const *const c_char,
            opts.inner,
        );
    }
}

#[cfg(test)]
mod test {
    use std::fs;
    use std::path::Path;
    use std::str;
    use std::string::String;
    use std::thread;

    use super::*;
    use crate::tempdir_with_prefix;

    #[test]
    fn external() {
        let path = tempdir_with_prefix("_rust_rocksdb_externaltest");
        let db = DB::open_default(path.path().to_str().unwrap()).unwrap();
        let p = db.put(b"k1", b"v1111");
        assert!(p.is_ok());
        let r: Result<Option<DBVector>, String> = db.get(b"k1");
        assert!(r.unwrap().unwrap().to_utf8().unwrap() == "v1111");
        assert!(db.delete(b"k1").is_ok());
        assert!(db.get(b"k1").unwrap().is_none());
    }

    #[allow(unused_variables)]
    #[test]
    fn errors_do_stuff() {
        let path = tempdir_with_prefix("_rust_rocksdb_error");
        let path_str = path.path().to_str().unwrap();
        let db = DB::open_default(path_str).unwrap();
        let opts = DBOptions::new();
        // The DB will still be open when we try to destroy and the lock should fail
        match DB::destroy(&opts, path_str) {
            Err(ref s) => {
                assert!(
                    s.contains("IO error: ") && s.contains("lock"),
                    "expect lock fail, but got {}",
                    s
                );
            }
            Ok(_) => panic!("should fail"),
        }
    }

    #[test]
    fn writebatch_works() {
        let path = tempdir_with_prefix("_rust_rocksdb_writebacktest");
        let db = DB::open_default(path.path().to_str().unwrap()).unwrap();

        // test put
        let batch = WriteBatch::new();
        assert!(db.get(b"k1").unwrap().is_none());
        assert_eq!(batch.count(), 0);
        assert!(batch.is_empty());
        let _ = batch.put(b"k1", b"v1111");
        assert_eq!(batch.count(), 1);
        assert!(!batch.is_empty());
        assert!(db.get(b"k1").unwrap().is_none());
        let p = db.write(&batch);
        assert!(p.is_ok());
        let r = db.get(b"k1");
        assert_eq!(r.unwrap().unwrap(), b"v1111");

        // test delete
        let batch = WriteBatch::new();
        let _ = batch.delete(b"k1");
        assert_eq!(batch.count(), 1);
        assert!(!batch.is_empty());
        let p = db.write(&batch);
        assert!(p.is_ok());
        assert!(db.get(b"k1").unwrap().is_none());

        let batch = WriteBatch::new();
        let prev_size = batch.data_size();
        let _ = batch.delete(b"k1");
        assert!(batch.data_size() > prev_size);
        batch.clear();
        assert_eq!(batch.data_size(), prev_size);

        // test save point
        let mut batch = WriteBatch::new();
        batch.set_save_point();
        batch.pop_save_point().unwrap();
        batch.put(b"k10", b"v10").unwrap();
        batch.set_save_point();
        batch.put(b"k11", b"v11").unwrap();
        batch.set_save_point();
        batch.put(b"k12", b"v12").unwrap();
        batch.set_save_point();
        batch.put(b"k13", b"v13").unwrap();
        batch.rollback_to_save_point().unwrap();
        batch.rollback_to_save_point().unwrap();
        let p = db.write(&batch);
        assert!(p.is_ok());
        let r = db.get(b"k10");
        assert_eq!(r.unwrap().unwrap(), b"v10");
        let r = db.get(b"k11");
        assert_eq!(r.unwrap().unwrap(), b"v11");
        let r = db.get(b"k12");
        assert!(r.unwrap().is_none());
        let r = db.get(b"k13");
        assert!(r.unwrap().is_none());

        // test with capacity
        let batch = WriteBatch::with_capacity(1024);
        batch.put(b"kc1", b"v1").unwrap();
        batch.put(b"kc2", b"v2").unwrap();
        let p = db.write(&batch);
        assert!(p.is_ok());
        let r = db.get(b"kc1");
        assert!(r.unwrap().is_some());
        let r = db.get(b"kc2");
        assert!(r.unwrap().is_some());
    }

    #[test]
    fn iterator_test() {
        let path = tempdir_with_prefix("_rust_rocksdb_iteratortest");

        let db = DB::open_default(path.path().to_str().unwrap()).unwrap();
        db.put(b"k1", b"v1111").expect("");
        db.put(b"k2", b"v2222").expect("");
        db.put(b"k3", b"v3333").expect("");
        let mut iter = db.iter();
        iter.seek(SeekKey::Start).unwrap();
        for (k, v) in &mut iter {
            println!(
                "Hello {}: {}",
                str::from_utf8(&*k).unwrap(),
                str::from_utf8(&*v).unwrap()
            );
        }
    }

    #[test]
    fn approximate_size_test() {
        let path = tempdir_with_prefix("_rust_rocksdb_iteratortest");
        let db = DB::open_default(path.path().to_str().unwrap()).unwrap();
        for i in 1..8000 {
            db.put(
                format!("{:04}", i).as_bytes(),
                format!("{:04}", i).as_bytes(),
            )
            .expect("");
        }
        db.flush(true).expect("");
        assert!(db.get(b"0001").expect("").is_some());
        db.flush(true).expect("");
        let sizes = db.get_approximate_sizes(&[
            Range::new(b"0000", b"2000"),
            Range::new(b"2000", b"4000"),
            Range::new(b"4000", b"6000"),
            Range::new(b"6000", b"8000"),
            Range::new(b"8000", b"9999"),
        ]);
        assert_eq!(sizes.len(), 5);
        for s in &sizes[0..4] {
            assert!(*s > 0);
        }
        assert_eq!(sizes[4], 0);
    }

    #[test]
    fn property_test() {
        let path = tempdir_with_prefix("_rust_rocksdb_propertytest");
        let db = DB::open_default(path.path().to_str().unwrap()).unwrap();
        db.put(b"a1", b"v1").unwrap();
        db.flush(true).unwrap();
        let prop_name = "rocksdb.total-sst-files-size";
        let st1 = db.get_property_int(prop_name).unwrap();
        assert!(st1 > 0);
        db.put(b"a2", b"v2").unwrap();
        db.flush(true).unwrap();
        let st2 = db.get_property_int(prop_name).unwrap();
        assert!(st2 > st1);
    }

    #[test]
    fn list_column_families_test() {
        let path = tempdir_with_prefix("_rust_rocksdb_list_column_families_test");
        let mut cfs = ["default", "cf1", "cf2", "cf3"];
        {
            let mut cfs_opts = vec![];
            for _ in 0..cfs.len() {
                cfs_opts.push(ColumnFamilyOptions::new());
            }

            let mut opts = DBOptions::new();
            opts.create_if_missing(true);
            let mut db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
            for (cf, cf_opts) in cfs.iter().zip(cfs_opts) {
                if *cf == "default" {
                    continue;
                }
                db.create_cf((*cf, cf_opts)).unwrap();
            }
        }
        let opts_list_cfs = DBOptions::new();
        let mut cfs_vec =
            DB::list_column_families(&opts_list_cfs, path.path().to_str().unwrap()).unwrap();
        cfs_vec.sort();
        cfs.sort();
        assert_eq!(cfs_vec, cfs);
    }

    #[test]
    fn backup_db_test() {
        let key = b"foo";
        let value = b"bar";

        let db_dir = tempdir_with_prefix("_rust_rocksdb_backuptest");
        let db = DB::open_default(db_dir.path().to_str().unwrap()).unwrap();
        let p = db.put(key, value);
        assert!(p.is_ok());

        // Make a backup.
        let backup_dir = tempdir_with_prefix("_rust_rocksdb_backuptest_backup");
        let backup_engine = db.backup_at(backup_dir.path().to_str().unwrap()).unwrap();

        // Restore it.
        let ropt1 = RestoreOptions::new();
        let mut ropt2 = RestoreOptions::new();
        ropt2.set_keep_log_files(true);
        let ropts = [ropt1, ropt2];
        for ropt in &ropts {
            let restore_dir = tempdir_with_prefix("_rust_rocksdb_backuptest_restore");
            let restored_db = DB::restore_from(
                &backup_engine,
                restore_dir.path().to_str().unwrap(),
                restore_dir.path().to_str().unwrap(),
                &ropt,
            )
            .unwrap();

            let r = restored_db.get(key);
            assert!(r.unwrap().unwrap().to_utf8().unwrap() == str::from_utf8(value).unwrap());
        }
    }

    #[test]
    fn log_dir_test() {
        let db_dir = tempdir_with_prefix("_rust_rocksdb_logdirtest");
        let db_path = db_dir.path().to_str().unwrap();
        let log_path = format!("{}", Path::new(&db_path).join("log_path").display());
        fs::create_dir_all(&log_path).unwrap();

        let mut opts = DBOptions::new();
        opts.create_if_missing(true);
        opts.set_db_log_dir(&log_path);

        DB::open(opts, db_path).unwrap();

        // Check LOG file.
        let mut read_dir = fs::read_dir(&log_path).unwrap();
        let entry = read_dir.next().unwrap().unwrap();
        let name = entry.file_name();
        name.to_str().unwrap().find("LOG").unwrap();

        for entry in fs::read_dir(&db_path).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name();
            assert!(name.to_str().unwrap().find("LOG").is_none());
        }
    }

    #[test]
    fn single_delete_test() {
        let path = tempdir_with_prefix("_rust_rocksdb_singledeletetest");
        let db = DB::open_default(path.path().to_str().unwrap()).unwrap();

        db.put(b"a", b"v1").unwrap();
        let a = db.get(b"a");
        assert_eq!(a.unwrap().unwrap().to_utf8().unwrap(), "v1");
        db.single_delete(b"a").unwrap();
        let a = db.get(b"a");
        assert!(a.unwrap().is_none());

        db.put(b"a", b"v2").unwrap();
        let a = db.get(b"a");
        assert_eq!(a.unwrap().unwrap().to_utf8().unwrap(), "v2");
        db.single_delete(b"a").unwrap();
        let a = db.get(b"a");
        assert!(a.unwrap().is_none());

        let cf_handle = db.cf_handle("default").unwrap();
        db.put_cf(cf_handle, b"a", b"v3").unwrap();
        let a = db.get_cf(cf_handle, b"a");
        assert_eq!(a.unwrap().unwrap().to_utf8().unwrap(), "v3");
        db.single_delete_cf(cf_handle, b"a").unwrap();
        let a = db.get_cf(cf_handle, b"a");
        assert!(a.unwrap().is_none());

        db.put_cf(cf_handle, b"a", b"v4").unwrap();
        let a = db.get_cf(cf_handle, b"a");
        assert_eq!(a.unwrap().unwrap().to_utf8().unwrap(), "v4");
        db.single_delete_cf(cf_handle, b"a").unwrap();
        let a = db.get_cf(cf_handle, b"a");
        assert!(a.unwrap().is_none());
    }

    #[test]
    fn test_pause_bg_work() {
        let path = tempdir_with_prefix("_rust_rocksdb_pause_bg_work");
        let db = DB::open_default(path.path().to_str().unwrap()).unwrap();
        let db = Arc::new(db);
        let db1 = db.clone();
        let builder = thread::Builder::new().name(String::from("put-thread"));
        let h = builder
            .spawn(move || {
                db1.put(b"k1", b"v1").unwrap();
                db1.put(b"k2", b"v2").unwrap();
                db1.flush(true).unwrap();
                db1.compact_range(None, None);
            })
            .unwrap();
        // Wait until all currently running background processes finish.
        db.pause_bg_work();
        assert_eq!(
            db.get_property_int("rocksdb.num-running-compactions")
                .unwrap(),
            0
        );
        assert_eq!(
            db.get_property_int("rocksdb.num-running-flushes").unwrap(),
            0
        );
        db.continue_bg_work();
        h.join().unwrap();
    }

    #[test]
    fn snapshot_test() {
        let path = "_rust_rocksdb_snapshottest";
        {
            let db = DB::open_default(path).unwrap();
            let p = db.put(b"k1", b"v1111");
            assert!(p.is_ok());

            let snap = db.snapshot();
            let mut r: Result<Option<DBVector>, String> = snap.get(b"k1");
            assert!(r.unwrap().unwrap().to_utf8().unwrap() == "v1111");

            r = db.get(b"k1");
            assert!(r.unwrap().unwrap().to_utf8().unwrap() == "v1111");

            let p = db.put(b"k2", b"v2222");
            assert!(p.is_ok());

            assert!(db.get(b"k2").unwrap().is_some());
            assert!(snap.get(b"k2").unwrap().is_none());
        }
        let opts = DBOptions::new();
        assert!(DB::destroy(&opts, path).is_ok());
    }

    #[test]
    fn block_cache_usage() {
        let path = tempdir_with_prefix("_rust_rocksdb_block_cache_usage");
        let db = DB::open_default(path.path().to_str().unwrap()).unwrap();

        for i in 0..200 {
            db.put(format!("k_{}", i).as_bytes(), b"v").unwrap();
        }
        db.flush(true).unwrap();
        for i in 0..200 {
            db.get(format!("k_{}", i).as_bytes()).unwrap();
        }

        assert!(db.get_block_cache_usage() > 0);
        let cf_handle = db.cf_handle("default").unwrap();
        assert!(db.get_block_cache_usage_cf(cf_handle) > 0);
    }

    #[test]
    fn flush_cf() {
        let path = tempdir_with_prefix("_rust_rocksdb_flush_cf");
        let mut opts = DBOptions::new();
        opts.create_if_missing(true);
        let mut db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
        db.create_cf("cf").unwrap();

        let cf_handle = db.cf_handle("cf").unwrap();
        for i in 0..200 {
            db.put_cf(cf_handle, format!("k_{}", i).as_bytes(), b"v")
                .unwrap();
        }
        db.flush_cf(cf_handle, true).unwrap();

        let total_sst_files_size = db
            .get_property_int_cf(cf_handle, "rocksdb.total-sst-files-size")
            .unwrap();
        assert!(total_sst_files_size > 0);
    }

    #[test]
    fn test_supported_compression() {
        let mut com = supported_compression();
        let len_before = com.len();
        assert!(com.len() != 0);
        com.dedup();
        assert_eq!(len_before, com.len());
        for c in com {
            println!("{:?}", c);
            println!("{}", c as u32);
            match c as u32 {
                0..=5 | 7 | 0x40 => assert!(true),
                _ => assert!(false),
            }
        }
    }

    #[test]
    fn test_get_all_key_versions() {
        let mut opts = DBOptions::new();
        opts.create_if_missing(true);
        let path = tempdir_with_prefix("_rust_rocksdb_get_all_key_version_test");
        let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();

        let samples = vec![
            (b"key1".to_vec(), b"value1".to_vec()),
            (b"key2".to_vec(), b"value2".to_vec()),
            (b"key3".to_vec(), b"value3".to_vec()),
            (b"key4".to_vec(), b"value4".to_vec()),
        ];

        // Put 4 keys.
        for &(ref k, ref v) in &samples {
            db.put(k, v).unwrap();
            assert_eq!(v.as_slice(), &*db.get(k).unwrap().unwrap());
        }
        db.flush(true).unwrap();
        let key_versions = db.get_all_key_versions(b"key2", b"key4").unwrap();
        assert_eq!(key_versions[1].key, "key3");
        assert_eq!(key_versions[1].value, "value3");
        assert_eq!(key_versions[1].seq, 3);
    }

    #[test]
    fn test_get_approximate_memtable_stats() {
        let mut opts = DBOptions::new();
        opts.create_if_missing(true);
        let path = tempdir_with_prefix("_rust_rocksdb_get_approximate_memtable_stats");
        let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();

        let samples = [
            (b"key1", b"value1"),
            (b"key2", b"value2"),
            (b"key3", b"value3"),
            (b"key4", b"value4"),
        ];

        for &(k, v) in &samples {
            db.put(k, v).unwrap();
        }

        let range = Range::new(b"a", b"z");

        let (count, size) = db.get_approximate_memtable_stats(&range);
        assert!(count > 0);
        assert!(size > 0);

        let cf = db.cf_handle("default").unwrap();
        let (count, size) = db.get_approximate_memtable_stats_cf(cf, &range);
        assert!(count > 0);
        assert!(size > 0);
    }

    #[test]
    fn test_set_options() {
        let mut opts = DBOptions::new();
        opts.create_if_missing(true);
        let path = tempdir_with_prefix("_rust_rocksdb_set_option");

        let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
        let cf = db.cf_handle("default").unwrap();

        let db_opts = db.get_db_options();
        assert_eq!(db_opts.get_max_background_jobs(), 2);
        db.set_db_options(&[("max_background_jobs", "8")]).unwrap();
        let db_opts = db.get_db_options();
        assert_eq!(db_opts.get_max_background_jobs(), 8);

        let cf_opts = db.get_options_cf(cf);
        assert_eq!(cf_opts.get_disable_auto_compactions(), false);
        db.set_options_cf(cf, &[("disable_auto_compactions", "true")])
            .unwrap();
        let cf_opts = db.get_options_cf(cf);
        assert_eq!(cf_opts.get_disable_auto_compactions(), true);
    }

    #[test]
    fn test_load_latest_options() {
        let path = tempdir_with_prefix("_rust_rocksdb_load_latest_option");
        let dbpath = path.path().to_str().unwrap().clone();
        let cf_name: &str = "cf_dynamic_level_bytes";

        // test when options not exist
        assert!(load_latest_options(dbpath, &Env::default(), false)
            .unwrap()
            .is_none());

        let mut opts = DBOptions::new();
        opts.create_if_missing(true);
        let mut db = DB::open(opts, dbpath).unwrap();

        let mut cf_opts = ColumnFamilyOptions::new();
        cf_opts.set_level_compaction_dynamic_level_bytes(true);
        db.create_cf((cf_name.clone(), cf_opts)).unwrap();
        let cf_handle = db.cf_handle(cf_name.clone()).unwrap();
        let cf_opts = db.get_options_cf(cf_handle);
        assert!(cf_opts.get_level_compaction_dynamic_level_bytes());

        let (_, cf_descs) = load_latest_options(dbpath, &Env::default(), false)
            .unwrap()
            .unwrap();

        for cf_desc in cf_descs {
            if cf_desc.name() == cf_name {
                assert!(cf_desc.options().get_level_compaction_dynamic_level_bytes());
            } else {
                assert!(!cf_desc.options().get_level_compaction_dynamic_level_bytes());
            }
        }
    }

    #[test]
    fn test_sequence_number() {
        let path = tempdir_with_prefix("_rust_rocksdb_sequence_number");

        let mut opts = DBOptions::new();
        opts.create_if_missing(true);
        let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();

        let snap = db.snapshot();
        let snap_seq = snap.get_sequence_number();
        let seq1 = db.get_latest_sequence_number();
        assert_eq!(snap_seq, seq1);

        db.put(b"key", b"value").unwrap();
        let seq2 = db.get_latest_sequence_number();
        assert!(seq2 > seq1);
    }

    #[test]
    fn test_atomic_flush() {
        let path = tempdir_with_prefix("_rust_rocksdb_test_atomic_flush");
        let cfs = ["default", "cf1", "cf2", "cf3"];
        let mut cfs_opts = vec![];
        for _ in 0..cfs.len() {
            cfs_opts.push(ColumnFamilyOptions::new());
        }

        {
            let mut opts = DBOptions::new();
            opts.create_if_missing(true);
            opts.set_atomic_flush(true);
            let mut db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
            let wb = WriteBatch::new();
            for (cf, cf_opts) in cfs.iter().zip(cfs_opts.iter().cloned()) {
                if *cf != "default" {
                    db.create_cf((*cf, cf_opts)).unwrap();
                }
                let handle = db.cf_handle(cf).unwrap();
                wb.put_cf(handle, b"k", cf.as_bytes()).unwrap();
            }
            let mut options = WriteOptions::new();
            options.disable_wal(true);
            db.write_opt(&wb, &options).unwrap();
            let handles: Vec<_> = cfs.iter().map(|name| db.cf_handle(name).unwrap()).collect();
            db.flush_cfs(&handles, true).unwrap();
        }

        let opts = DBOptions::new();
        let db = DB::open_cf(
            opts,
            path.path().to_str().unwrap(),
            cfs.iter().map(|cf| *cf).zip(cfs_opts).collect(),
        )
        .unwrap();
        for cf in &cfs {
            let handle = db.cf_handle(cf).unwrap();
            assert_eq!(db.get_cf(handle, b"k").unwrap().unwrap(), cf.as_bytes());
        }
    }

    #[test]
    fn test_map_property() {
        let path = tempdir_with_prefix("_rust_rocksdb_get_map_property");
        let dbpath = path.path().to_str().unwrap().clone();

        let mut opts = DBOptions::new();
        opts.create_if_missing(true);
        let db = DB::open(opts, dbpath).unwrap();

        let cf_handle = db.cf_handle("default").unwrap();
        let mp = db.get_map_property_cf(cf_handle, "rocksdb.cfstats");
        assert!(mp.is_some());
    }

    #[test]
    fn test_multi_batch_write() {
        let mut opts = DBOptions::new();
        opts.create_if_missing(true);
        opts.enable_multi_batch_write(true);
        let path = tempdir_with_prefix("_rust_rocksdb_multi_batch");

        let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
        let cf = db.cf_handle("default").unwrap();
        let mut data = Vec::new();
        for s in &[b"ab", b"cd", b"ef"] {
            let w = WriteBatch::new();
            w.put_cf(cf, s.to_vec().as_slice(), b"a").unwrap();
            data.push(w);
        }
        db.multi_batch_write(&data, &WriteOptions::new()).unwrap();
        for s in &[b"ab", b"cd", b"ef"] {
            let v = db.get_cf(cf, s.to_vec().as_slice()).unwrap();
            assert!(v.is_some());
            assert_eq!(v.unwrap().to_utf8().unwrap(), "a");
        }
    }

    #[test]
    fn test_get_db_path_from_option() {
        let mut opts = DBOptions::new();
        opts.create_if_missing(true);
        let dir = tempdir_with_prefix("_rust_rocksdb_get_db_path_from_option");
        let path = dir.path().to_str().unwrap();
        let db = DB::open(opts, path).unwrap();
        let path_num = db.get_db_options().get_db_paths_num();
        assert_eq!(1, path_num);
        let first_path = db.get_db_options().get_db_path(0).unwrap();
        assert_eq!(path, first_path.as_str());
    }
}
