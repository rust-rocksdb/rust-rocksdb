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


use {Db, DbOptions, Error, ReadOptions, Snapshot, WriteOptions};
use ffi;

use libc::{self, c_char, c_int, c_uchar, c_void, size_t};
use std::collections::BTreeMap;
use std::ffi::CString;
use std::fmt;
use std::fs;
use std::ops::Deref;
use std::path::Path;
use std::ptr;
use std::slice;
use std::str;

pub fn new_bloom_filter(bits: c_int) -> *mut ffi::rocksdb_filterpolicy_t {
    unsafe { ffi::rocksdb_filterpolicy_create_bloom(bits) }
}

unsafe impl Send for Db {}
unsafe impl Sync for Db {}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DbCompressionType {
    None = ffi::rocksdb_no_compression as isize,
    Snappy = ffi::rocksdb_snappy_compression as isize,
    Zlib = ffi::rocksdb_zlib_compression as isize,
    Bz2 = ffi::rocksdb_bz2_compression as isize,
    Lz4 = ffi::rocksdb_lz4_compression as isize,
    Lz4hc = ffi::rocksdb_lz4hc_compression as isize,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DbCompactionStyle {
    Level = ffi::rocksdb_level_compaction as isize,
    Universal = ffi::rocksdb_universal_compaction as isize,
    Fifo = ffi::rocksdb_fifo_compaction as isize,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DbRecoveryMode {
    TolerateCorruptedTailRecords = ffi::rocksdb_recovery_mode_tolerate_corrupted_tail_records as isize,
    AbsoluteConsistency = ffi::rocksdb_recovery_mode_absolute_consistency as isize,
    PointInTime = ffi::rocksdb_recovery_mode_point_in_time as isize,
    SkipAnyCorruptedRecord = ffi::rocksdb_recovery_mode_skip_any_corrupted_record as isize,
}

/// An atomic batch of write operations.
///
/// Making an atomic commit of several writes:
///
/// ```
/// use rocksdb::{Db, WriteBatch, WriteOptions};
///
/// let db = Db::open_default("path/for/rocksdb/storage1").unwrap();
/// {
///     let mut batch = WriteBatch::default();
///     batch.put(b"my key", b"my value");
///     batch.put(b"key2", b"value2");
///     batch.put(b"key3", b"value3");
///     db.write(batch, &WriteOptions::default()); // Atomically commits the batch
/// }
/// ```
pub struct WriteBatch {
    inner: *mut ffi::rocksdb_writebatch_t,
}

/// An iterator over a database or column family, with specifiable
/// ranges and direction.
///
/// ```
/// use rocksdb::{Db, Direction, IteratorMode, ReadOptions};
///
/// let mut db = Db::open_default("path/for/rocksdb/storage2").unwrap();
/// let mut iter = db.iterator(IteratorMode::Start, &ReadOptions::default()); // Always iterates forward
/// for (key, value) in iter {
///     println!("Saw {:?} {:?}", key, value);
/// }
/// iter = db.iterator(IteratorMode::End, &ReadOptions::default());  // Always iterates backward
/// for (key, value) in iter {
///     println!("Saw {:?} {:?}", key, value);
/// }
/// iter = db.iterator(IteratorMode::From(b"my key", Direction::Forward), &ReadOptions::default()); // From a key in Direction::{forward,reverse}
/// for (key, value) in iter {
///     println!("Saw {:?} {:?}", key, value);
/// }
///
/// // You can seek with an existing Iterator instance, too
/// iter = db.iterator(IteratorMode::Start, &ReadOptions::default());
/// iter.set_mode(IteratorMode::From(b"another key", Direction::Reverse));
/// for (key, value) in iter {
///     println!("Saw {:?} {:?}", key, value);
/// }
/// ```
pub struct DbIterator {
    inner: *mut ffi::rocksdb_iterator_t,
    direction: Direction,
    just_seeked: bool,
}

pub enum Direction {
    Forward,
    Reverse,
}

pub type KVBytes = (Box<[u8]>, Box<[u8]>);

impl Iterator for DbIterator {
    type Item = KVBytes;

    fn next(&mut self) -> Option<KVBytes> {
        if !self.just_seeked {
            match self.direction {
                Direction::Forward => unsafe { ffi::rocksdb_iter_next(self.inner) },
                Direction::Reverse => unsafe { ffi::rocksdb_iter_prev(self.inner) },
            }
        } else {
            self.just_seeked = false;
        }
        if self.valid() {
            let mut key_len: size_t = 0;
            let mut val_len: size_t = 0;
            let key_ptr =
                unsafe { ffi::rocksdb_iter_key(self.inner, &mut key_len) as *const c_uchar };
            let key = unsafe { slice::from_raw_parts(key_ptr, key_len as usize) };
            let val_ptr =
                unsafe { ffi::rocksdb_iter_value(self.inner, &mut val_len) as *const c_uchar };
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

impl DbIterator {
    fn new(db: &Db, opts: &ReadOptions, mode: IteratorMode) -> DbIterator {
        unsafe {
            let iterator = ffi::rocksdb_create_iterator(db.inner, opts.inner);

            let mut rv = DbIterator {
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
            // TODO: call `rocksdb_iter_get_error` after seeks
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
                    ffi::rocksdb_iter_seek(self.inner,
                                           key.as_ptr() as *const c_char,
                                           key.len() as size_t);
                    self.direction = dir;
                }
            };
            self.just_seeked = true;
        }
    }

    pub fn valid(&self) -> bool {
        unsafe { ffi::rocksdb_iter_valid(self.inner) != 0 }
    }

    fn new_cf(db: &Db,
              cf_handle: *mut ffi::rocksdb_column_family_handle_t,
              opts: &ReadOptions,
              mode: IteratorMode)
              -> Result<DbIterator, Error> {
        unsafe {
            let iterator = ffi::rocksdb_create_iterator_cf(db.inner, opts.inner, cf_handle);

            let mut rv = DbIterator {
                inner: iterator,
                direction: Direction::Forward, // blown away by call to `set_mode`
                just_seeked: false,
            };
            rv.set_mode(mode);
            Ok(rv)
        }
    }
}

impl Drop for DbIterator {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_iter_destroy(self.inner);
        }
    }
}

impl<'a> Snapshot<'a> {
    pub fn new(db: &Db) -> Snapshot {
        let snapshot = unsafe { ffi::rocksdb_create_snapshot(db.inner) };
        Snapshot {
            db: db,
            inner: snapshot,
        }
    }

    pub fn iterator(&self, mode: IteratorMode, opts: &mut ReadOptions) -> DbIterator {
        opts.set_snapshot(self);
        DbIterator::new(self.db, opts, mode)
    }

    pub fn iterator_cf(&self,
                       cf_handle: *mut ffi::rocksdb_column_family_handle_t,
                       mode: IteratorMode,
                       opts: &mut ReadOptions)
                       -> Result<DbIterator, Error> {
        opts.set_snapshot(self);
        DbIterator::new_cf(self.db, cf_handle, opts, mode)
    }

    pub fn get(&self, key: &[u8], opts: &mut ReadOptions) -> Result<Option<DbVector>, Error> {
        opts.set_snapshot(self);
        self.db.get(key, opts)
    }

    pub fn get_cf(&self,
                  cf: *mut ffi::rocksdb_column_family_handle_t,
                  key: &[u8],
                  opts: &mut ReadOptions)
                  -> Result<Option<DbVector>, Error> {
        opts.set_snapshot(self);
        self.db.get_cf(cf, key, opts)
    }
}

impl<'a> Drop for Snapshot<'a> {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_release_snapshot(self.db.inner, self.inner);
        }
    }
}

impl Db {
    /// Open a database with default options.
    pub fn open_default<P: AsRef<Path>>(path: P) -> Result<Db, Error> {
        let mut opts = DbOptions::default();
        opts.create_if_missing(true);
        Db::open(path, opts)
    }

    /// Open the database with the specified options.
    pub fn open<P: AsRef<Path>>(path: P, opts: DbOptions) -> Result<Db, Error> {
        Db::open_cf(path, &[], opts)
    }

    /// Open a database with specified options and column family.
    ///
    /// A column family must be created first by calling `Db::create_cf`.
    ///
    /// # Panics
    ///
    /// * Panics if the column family doesn't exist.
    pub fn open_cf<P: AsRef<Path>>(path: P,
                                   cfs: &[&str],
                                   mut opts: DbOptions)
                                   -> Result<Db, Error> {
        let path = path.as_ref();
        let cpath = match CString::new(path.to_string_lossy().as_bytes()) {
            Ok(c) => c,
            Err(_) => {
                return Err(Error::new("Failed to convert path to CString \
                                       when opening DB."
                    .to_owned()))
            }
        };

        if let Err(e) = fs::create_dir_all(&path) {
            return Err(Error::new(format!("Failed to create RocksDB\
                                           directory: `{:?}`.",
                                          e)));
        }

        let db: *mut ffi::rocksdb_t;
        let mut cf_map = BTreeMap::new();

        if cfs.len() == 0 {
            unsafe {
                db = ffi_try!(ffi::rocksdb_open(opts.inner, cpath.as_ptr() as *const _));
            }
        } else {
            let mut cfs_v = cfs.to_vec();
            // Always open the default column family.
            if !cfs_v.contains(&"default") {
                cfs_v.push("default");
            }

            // We need to store the CStrings in an intermediate vector
            // so that their pointers remain valid.
            let c_cfs: Vec<CString> = cfs_v.iter()
                .map(|cf| CString::new(cf.as_bytes()).unwrap())
                .collect();

            let cfnames: Vec<_> = c_cfs.iter().map(|cf| cf.as_ptr()).collect();

            // These handles will be populated by DB.
            let mut cfhandles: Vec<_> = cfs_v.iter().map(|_| ptr::null_mut()).collect();

            // TODO(tyler) Allow options to be passed in.
            let cfopts: Vec<_> = cfs_v.iter()
                .map(|_| unsafe { ffi::rocksdb_options_create() as *const _ })
                .collect();

            unsafe {
                db = ffi_try!(ffi::rocksdb_open_column_families(opts.inner,
                                                                cpath.as_ptr() as *const _,
                                                                cfs_v.len() as c_int,
                                                                cfnames.as_ptr() as *const _,
                                                                cfopts.as_ptr(),
                                                                cfhandles.as_mut_ptr()));
            }

            for handle in &cfhandles {
                if handle.is_null() {
                    return Err(Error::new("Received null column family \
                                           handle from DB."
                        .to_owned()));
                }
            }

            for (n, h) in cfs_v.iter().zip(cfhandles) {
                cf_map.insert(n.to_string(), h);
            }
        }

        if db.is_null() {
            return Err(Error::new("Could not initialize database.".to_owned()));
        }

        Ok(Db {
            inner: db,
            cfs: cf_map,
            path: path.to_path_buf(),
            comparator: opts.comparator.take(),
            prefix_extractor: opts.prefix_extractor.take(),
        })
    }

    pub fn destroy<P: AsRef<Path>>(path: P, opts: DbOptions) -> Result<(), Error> {
        let cpath = CString::new(path.as_ref().to_string_lossy().as_bytes()).unwrap();
        unsafe {
            ffi_try!(ffi::rocksdb_destroy_db(opts.inner, cpath.as_ptr()));
        }
        Ok(())
    }

    pub fn repair<P: AsRef<Path>>(path: P, opts: DbOptions) -> Result<(), Error> {
        let cpath = CString::new(path.as_ref().to_string_lossy().as_bytes()).unwrap();
        unsafe {
            ffi_try!(ffi::rocksdb_repair_db(opts.inner, cpath.as_ptr()));
        }
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path.as_path()
    }

    pub fn write(&self, batch: WriteBatch, opts: &WriteOptions) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_write(self.inner, opts.inner, batch.inner));
        }
        Ok(())
    }

    pub fn write_without_wal(&self, batch: WriteBatch) -> Result<(), Error> {
        let mut wo = WriteOptions::new();
        wo.disable_wal(true);
        self.write(batch, &wo)
    }

    /// Return the bytes associated with a key value
    pub fn get(&self, key: &[u8], opts: &ReadOptions) -> Result<Option<DbVector>, Error> {
        if opts.inner.is_null() {
            return Err(Error::new("Unable to create RocksDB read options. \
                                   This is a fairly trivial call, and its \
                                   failure may be indicative of a \
                                   mis-compiled or mis-loaded RocksDB \
                                   library."
                .to_owned()));
        }

        unsafe {
            let mut val_len: size_t = 0;
            let val = ffi_try!(ffi::rocksdb_get(self.inner,
                                                opts.inner,
                                                key.as_ptr() as *const c_char,
                                                key.len() as size_t,
                                                &mut val_len)) as *mut u8;
            if val.is_null() {
                Ok(None)
            } else {
                Ok(Some(DbVector::from_c(val, val_len)))
            }
        }
    }

    pub fn get_cf(&self,
                  cf: *mut ffi::rocksdb_column_family_handle_t,
                  key: &[u8],
                  opts: &ReadOptions)
                  -> Result<Option<DbVector>, Error> {
        if opts.inner.is_null() {
            return Err(Error::new("Unable to create RocksDB read options. \
                                   This is a fairly trivial call, and its \
                                   failure may be indicative of a \
                                   mis-compiled or mis-loaded RocksDB \
                                   library."
                .to_owned()));
        }

        unsafe {
            let mut val_len: size_t = 0;
            let val = ffi_try!(ffi::rocksdb_get_cf(self.inner,
                                                   opts.inner,
                                                   cf,
                                                   key.as_ptr() as *const c_char,
                                                   key.len() as size_t,
                                                   &mut val_len)) as *mut u8;
            if val.is_null() {
                Ok(None)
            } else {
                Ok(Some(DbVector::from_c(val, val_len)))
            }
        }
    }

    pub fn create_cf(&mut self,
                     name: &str,
                     opts: &DbOptions)
                     -> Result<*mut ffi::rocksdb_column_family_handle_t, Error> {
        let cname = match CString::new(name.as_bytes()) {
            Ok(c) => c,
            Err(_) => {
                return Err(Error::new("Failed to convert path to CString \
                                       when opening rocksdb"
                    .to_owned()))
            }
        };
        let cf_handler = unsafe {
            let cf_handler =
                ffi_try!(ffi::rocksdb_create_column_family(self.inner, opts.inner, cname.as_ptr()));
            self.cfs.insert(name.to_string(), cf_handler);
            cf_handler
        };
        Ok(cf_handler)
    }

    pub fn drop_cf(&mut self, name: &str) -> Result<(), Error> {
        let cf = self.cfs.get(name);
        if cf.is_none() {
            return Err(Error::new(format!("Invalid column family: {}", name).to_owned()));
        }
        unsafe {
            ffi_try!(ffi::rocksdb_drop_column_family(self.inner, *cf.unwrap()));
        }
        Ok(())
    }

    /// Return the underlying column family handle.
    pub fn cf_handle(&self, name: &str) -> Option<&*mut ffi::rocksdb_column_family_handle_t> {
        self.cfs.get(name)
    }

    pub fn iterator(&self, mode: IteratorMode, opts: &ReadOptions) -> DbIterator {
        DbIterator::new(self, opts, mode)
    }

    pub fn iterator_cf(&self,
                       cf_handle: *mut ffi::rocksdb_column_family_handle_t,
                       mode: IteratorMode,
                       opts: &ReadOptions)
                       -> Result<DbIterator, Error> {
        DbIterator::new_cf(self, cf_handle, opts, mode)
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot::new(self)
    }

    pub fn put(&self, key: &[u8], value: &[u8], opts: &WriteOptions) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_put(self.inner,
                                      opts.inner,
                                      key.as_ptr() as *const c_char,
                                      key.len() as size_t,
                                      value.as_ptr() as *const c_char,
                                      value.len() as size_t));
            Ok(())
        }
    }

    pub fn put_cf(&self,
                  cf: *mut ffi::rocksdb_column_family_handle_t,
                  key: &[u8],
                  value: &[u8],
                  opts: &WriteOptions)
                  -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_put_cf(self.inner,
                                         opts.inner,
                                         cf,
                                         key.as_ptr() as *const c_char,
                                         key.len() as size_t,
                                         value.as_ptr() as *const c_char,
                                         value.len() as size_t));
            Ok(())
        }
    }

    pub fn merge(&self, key: &[u8], value: &[u8], opts: &WriteOptions) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_merge(self.inner,
                                        opts.inner,
                                        key.as_ptr() as *const c_char,
                                        key.len() as size_t,
                                        value.as_ptr() as *const c_char,
                                        value.len() as size_t));
            Ok(())
        }
    }

    pub fn merge_cf(&self,
                    cf: *mut ffi::rocksdb_column_family_handle_t,
                    key: &[u8],
                    value: &[u8],
                    opts: &WriteOptions)
                    -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_merge_cf(self.inner,
                                           opts.inner,
                                           cf,
                                           key.as_ptr() as *const i8,
                                           key.len() as size_t,
                                           value.as_ptr() as *const i8,
                                           value.len() as size_t));
            Ok(())
        }
    }

    pub fn delete(&self, key: &[u8], opts: &WriteOptions) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_delete(self.inner,
                                         opts.inner,
                                         key.as_ptr() as *const c_char,
                                         key.len() as size_t));
            Ok(())
        }
    }

    pub fn delete_cf(&self,
                     cf: *mut ffi::rocksdb_column_family_handle_t,
                     key: &[u8],
                     opts: &WriteOptions)
                     -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_delete_cf(self.inner,
                                            opts.inner,
                                            cf,
                                            key.as_ptr() as *const c_char,
                                            key.len() as size_t));
            Ok(())
        }
    }
}

impl WriteBatch {
    pub fn new() -> Self {
        unsafe { WriteBatch { inner: ffi::rocksdb_writebatch_create() } }
    }

    pub fn len(&self) -> usize {
        unsafe { ffi::rocksdb_writebatch_count(self.inner) as usize }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Insert a value into the database under the given key.
    pub fn put(&mut self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        unsafe {
            ffi::rocksdb_writebatch_put(self.inner,
                                        key.as_ptr() as *const i8,
                                        key.len() as size_t,
                                        value.as_ptr() as *const i8,
                                        value.len() as size_t);
            Ok(())
        }
    }

    pub fn put_cf(&mut self,
                  cf: *mut ffi::rocksdb_column_family_handle_t,
                  key: &[u8],
                  value: &[u8])
                  -> Result<(), Error> {
        unsafe {
            ffi::rocksdb_writebatch_put_cf(self.inner,
                                           cf,
                                           key.as_ptr() as *const i8,
                                           key.len() as size_t,
                                           value.as_ptr() as *const i8,
                                           value.len() as size_t);
            Ok(())
        }
    }

    pub fn merge(&mut self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        unsafe {
            ffi::rocksdb_writebatch_merge(self.inner,
                                          key.as_ptr() as *const i8,
                                          key.len() as size_t,
                                          value.as_ptr() as *const i8,
                                          value.len() as size_t);
            Ok(())
        }
    }

    pub fn merge_cf(&mut self,
                    cf: *mut ffi::rocksdb_column_family_handle_t,
                    key: &[u8],
                    value: &[u8])
                    -> Result<(), Error> {
        unsafe {
            ffi::rocksdb_writebatch_merge_cf(self.inner,
                                             cf,
                                             key.as_ptr() as *const i8,
                                             key.len() as size_t,
                                             value.as_ptr() as *const i8,
                                             value.len() as size_t);
            Ok(())
        }
    }

    /// Remove the database entry for key.
    ///
    /// Returns an error if the key was not found.
    pub fn delete(&mut self, key: &[u8]) -> Result<(), Error> {
        unsafe {
            ffi::rocksdb_writebatch_delete(self.inner,
                                           key.as_ptr() as *const i8,
                                           key.len() as size_t);
            Ok(())
        }
    }

    pub fn delete_cf(&mut self,
                     cf: *mut ffi::rocksdb_column_family_handle_t,
                     key: &[u8])
                     -> Result<(), Error> {
        unsafe {
            ffi::rocksdb_writebatch_delete_cf(self.inner,
                                              cf,
                                              key.as_ptr() as *const i8,
                                              key.len() as size_t);
            Ok(())
        }
    }
}

impl Default for WriteBatch {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WriteBatch {
    fn drop(&mut self) {
        unsafe { ffi::rocksdb_writebatch_destroy(self.inner) }
    }
}

impl Drop for Db {
    fn drop(&mut self) {
        unsafe {
            for cf in self.cfs.values() {
                ffi::rocksdb_column_family_handle_destroy(*cf);
            }
            ffi::rocksdb_close(self.inner);
        }
    }
}

impl fmt::Debug for Db {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RocksDB {{ path: {:?} }}", self.path())
    }
}

/// Vector of bytes stored in the database.
pub struct DbVector {
    base: *mut u8,
    len: usize,
}

impl Deref for DbVector {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.base, self.len) }
    }
}

impl Drop for DbVector {
    fn drop(&mut self) {
        unsafe {
            libc::free(self.base as *mut c_void);
        }
    }
}

impl DbVector {
    pub fn from_c(val: *mut u8, val_len: size_t) -> DbVector {
        DbVector {
            base: val,
            len: val_len as usize,
        }
    }

    pub fn to_utf8(&self) -> Option<&str> {
        str::from_utf8(self.deref()).ok()
    }
}

#[cfg(test)]
mod tests {
    use std::str;
    use super::*;

    #[test]
    fn external() {
        let path = "_rust_rocksdb_externaltest";
        {
            let db = Db::open_default(path).unwrap();
            let p = db.put(b"k1", b"v1111", &WriteOptions::default());
            assert!(p.is_ok());
            let r: Result<Option<DbVector>, Error> = db.get(b"k1", &ReadOptions::default());
            assert!(r.unwrap().unwrap().to_utf8().unwrap() == "v1111");
            assert!(db.delete(b"k1", &WriteOptions::default()).is_ok());
            assert!(db.get(b"k1", &ReadOptions::default()).unwrap().is_none());
        }
        let opts = DbOptions::default();
        let result = Db::destroy(path, opts);
        assert!(result.is_ok());
    }

    #[test]
    fn errors_do_stuff() {
        let path = "_rust_rocksdb_error";
        let _db = Db::open_default(path).unwrap();
        let opts = DbOptions::default();
        // The DB will still be open when we try to destroy it and the lock should fail.
        match Db::destroy(path, opts) {
            Err(s) => {
                let message = s.to_string();
                assert!(message.find("IO error:").is_some());
                assert!(message.find("_rust_rocksdb_error/LOCK:").is_some());
            }
            Ok(_) => panic!("should fail"),
        }
    }

    #[test]
    fn writebatch_works() {
        let path = "_rust_rocksdb_writebacktest";
        {
            let db = Db::open_default(path).unwrap();
            {
                // test put
                let mut batch = WriteBatch::default();
                assert!(db.get(b"k1", &ReadOptions::default()).unwrap().is_none());
                assert_eq!(batch.len(), 0);
                assert!(batch.is_empty());
                let _ = batch.put(b"k1", b"v1111");
                assert_eq!(batch.len(), 1);
                assert!(!batch.is_empty());
                assert!(db.get(b"k1", &ReadOptions::default()).unwrap().is_none());
                let p = db.write(batch, &WriteOptions::default());
                assert!(p.is_ok());
                let r: Result<Option<DbVector>, Error> = db.get(b"k1", &ReadOptions::default());
                assert!(r.unwrap().unwrap().to_utf8().unwrap() == "v1111");
            }
            {
                // test delete
                let mut batch = WriteBatch::default();
                let _ = batch.delete(b"k1");
                assert_eq!(batch.len(), 1);
                assert!(!batch.is_empty());
                let p = db.write(batch, &WriteOptions::default());
                assert!(p.is_ok());
                assert!(db.get(b"k1", &ReadOptions::default()).unwrap().is_none());
            }
        }
        let opts = DbOptions::default();
        assert!(Db::destroy(path, opts).is_ok());
    }

    #[test]
    fn iterator_test() {
        let path = "_rust_rocksdb_iteratortest";
        {
            let db = Db::open_default(path).unwrap();
            let p = db.put(b"k1", b"v1111", &WriteOptions::default());
            assert!(p.is_ok());
            let p = db.put(b"k2", b"v2222", &WriteOptions::default());
            assert!(p.is_ok());
            let p = db.put(b"k3", b"v3333", &WriteOptions::default());
            assert!(p.is_ok());
            let iter = db.iterator(IteratorMode::Start, &ReadOptions::default());
            for (k, v) in iter {
                println!("Hello {}: {}",
                         str::from_utf8(&*k).unwrap(),
                         str::from_utf8(&*v).unwrap());
            }
        }
        let opts = DbOptions::default();
        assert!(Db::destroy(path, opts).is_ok());
    }

    #[test]
    fn snapshot_test() {
        let path = "_rust_rocksdb_snapshottest";
        {
            let db = Db::open_default(path).unwrap();
            let p = db.put(b"k1", b"v1111", &WriteOptions::default());
            assert!(p.is_ok());

            let snap = db.snapshot();
            let r: Result<Option<DbVector>, Error> = snap.get(b"k1", &mut ReadOptions::default());
            assert!(r.unwrap().unwrap().to_utf8().unwrap() == "v1111");

            let p = db.put(b"k2", b"v2222", &WriteOptions::default());
            assert!(p.is_ok());

            assert!(db.get(b"k2", &ReadOptions::default()).unwrap().is_some());
            assert!(snap.get(b"k2", &mut ReadOptions::default()).unwrap().is_none());
        }
        let opts = DbOptions::default();
        assert!(Db::destroy(path, opts).is_ok());
    }
}
