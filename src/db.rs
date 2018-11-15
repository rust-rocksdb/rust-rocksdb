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


use {DB, Error, Options, WriteOptions, ColumnFamily, ColumnFamilyDescriptor};
use ffi;
use ffi_util::opt_bytes_to_ptr;

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
use std::ffi::CStr;

pub fn new_bloom_filter(bits: c_int) -> *mut ffi::rocksdb_filterpolicy_t {
    unsafe { ffi::rocksdb_filterpolicy_create_bloom(bits) }
}

unsafe impl Send for DB {}
unsafe impl Sync for DB {}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DBCompressionType {
    None = ffi::rocksdb_no_compression as isize,
    Snappy = ffi::rocksdb_snappy_compression as isize,
    Zlib = ffi::rocksdb_zlib_compression as isize,
    Bz2 = ffi::rocksdb_bz2_compression as isize,
    Lz4 = ffi::rocksdb_lz4_compression as isize,
    Lz4hc = ffi::rocksdb_lz4hc_compression as isize,
    Zstd = ffi::rocksdb_zstd_compression as isize,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DBCompactionStyle {
    Level = ffi::rocksdb_level_compaction as isize,
    Universal = ffi::rocksdb_universal_compaction as isize,
    Fifo = ffi::rocksdb_fifo_compaction as isize,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DBRecoveryMode {
    TolerateCorruptedTailRecords = ffi::rocksdb_tolerate_corrupted_tail_records_recovery as isize,
    AbsoluteConsistency = ffi::rocksdb_absolute_consistency_recovery as isize,
    PointInTime = ffi::rocksdb_point_in_time_recovery as isize,
    SkipAnyCorruptedRecord = ffi::rocksdb_skip_any_corrupted_records_recovery as isize,
}

/// An atomic batch of write operations.
///
/// Making an atomic commit of several writes:
///
/// ```
/// use rocksdb::{DB, WriteBatch};
///
/// let db = DB::open_default("path/for/rocksdb/storage1").unwrap();
/// {
///     let mut batch = WriteBatch::default();
///     batch.put(b"my key", b"my value");
///     batch.put(b"key2", b"value2");
///     batch.put(b"key3", b"value3");
///     db.write(batch); // Atomically commits the batch
/// }
/// ```
pub struct WriteBatch {
    inner: *mut ffi::rocksdb_writebatch_t,
}

pub struct ReadOptions {
    inner: *mut ffi::rocksdb_readoptions_t,
}

/// A consistent view of the database at the point of creation.
///
/// ```
/// use rocksdb::{DB, IteratorMode};
///
/// let db = DB::open_default("path/for/rocksdb/storage3").unwrap();
/// let snapshot = db.snapshot(); // Creates a longer-term snapshot of the DB, but closed when goes out of scope
/// let mut iter = snapshot.iterator(IteratorMode::Start); // Make as many iterators as you'd like from one snapshot
/// ```
///
pub struct Snapshot<'a> {
    db: &'a DB,
    inner: *const ffi::rocksdb_snapshot_t,
}

/// An iterator over a database or column family, with specifiable
/// ranges and direction.
///
/// This iterator is different to the standard ``DBIterator`` as it aims Into
/// replicate the underlying iterator API within RocksDB itself. This should
/// give access to more performance and flexibility but departs from the
/// widely recognised Rust idioms.
///
/// ```
/// use rocksdb::DB;
///
/// let mut db = DB::open_default("path/for/rocksdb/storage4").unwrap();
/// let mut iter = db.raw_iterator();
///
/// // Forwards iteration
/// iter.seek_to_first();
/// while iter.valid() {
///     println!("Saw {:?} {:?}", iter.key(), iter.value());
///     iter.next();
/// }
///
/// // Reverse iteration
/// iter.seek_to_last();
/// while iter.valid() {
///     println!("Saw {:?} {:?}", iter.key(), iter.value());
///     iter.prev();
/// }
///
/// // Seeking
/// iter.seek(b"my key");
/// while iter.valid() {
///     println!("Saw {:?} {:?}", iter.key(), iter.value());
///     iter.next();
/// }
///
/// // Reverse iteration from key
/// // Note, use seek_for_prev when reversing because if this key doesn't exist,
/// // this will make the iterator start from the previous key rather than the next.
/// iter.seek_for_prev(b"my key");
/// while iter.valid() {
///     println!("Saw {:?} {:?}", iter.key(), iter.value());
///     iter.prev();
/// }
/// ```
pub struct DBRawIterator {
    inner: *mut ffi::rocksdb_iterator_t,
}


/// An iterator over a database or column family, with specifiable
/// ranges and direction.
///
/// ```
/// use rocksdb::{DB, Direction, IteratorMode};
///
/// let mut db = DB::open_default("path/for/rocksdb/storage2").unwrap();
/// let mut iter = db.iterator(IteratorMode::Start); // Always iterates forward
/// for (key, value) in iter {
///     println!("Saw {:?} {:?}", key, value);
/// }
/// iter = db.iterator(IteratorMode::End);  // Always iterates backward
/// for (key, value) in iter {
///     println!("Saw {:?} {:?}", key, value);
/// }
/// iter = db.iterator(IteratorMode::From(b"my key", Direction::Forward)); // From a key in Direction::{forward,reverse}
/// for (key, value) in iter {
///     println!("Saw {:?} {:?}", key, value);
/// }
///
/// // You can seek with an existing Iterator instance, too
/// iter = db.iterator(IteratorMode::Start);
/// iter.set_mode(IteratorMode::From(b"another key", Direction::Reverse));
/// for (key, value) in iter {
///     println!("Saw {:?} {:?}", key, value);
/// }
/// ```
pub struct DBIterator {
    raw: DBRawIterator,
    direction: Direction,
    just_seeked: bool,
}

unsafe impl Send for DBIterator {}
unsafe impl Sync for DBIterator {}

pub enum Direction {
    Forward,
    Reverse,
}

pub type KVBytes = (Box<[u8]>, Box<[u8]>);

pub enum IteratorMode<'a> {
    Start,
    End,
    From(&'a [u8], Direction),
}

impl DBRawIterator {
    fn new(db: &DB, readopts: &ReadOptions) -> DBRawIterator {
        unsafe { DBRawIterator { inner: ffi::rocksdb_create_iterator(db.inner, readopts.inner) } }
    }

    fn new_cf(
        db: &DB,
        cf_handle: ColumnFamily,
        readopts: &ReadOptions,
    ) -> Result<DBRawIterator, Error> {
        unsafe {
            Ok(DBRawIterator {
                inner: ffi::rocksdb_create_iterator_cf(db.inner, readopts.inner, cf_handle.inner),
            })
        }
    }

    /// Returns true if the iterator is valid.
    pub fn valid(&self) -> bool {
        unsafe { ffi::rocksdb_iter_valid(self.inner) != 0 }
    }

    /// Seeks to the first key in the database.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rocksdb::DB;
    ///
    /// let mut db = DB::open_default("path/for/rocksdb/storage5").unwrap();
    /// let mut iter = db.raw_iterator();
    ///
    /// // Iterate all keys from the start in lexicographic order
    ///
    /// iter.seek_to_first();
    ///
    /// while iter.valid() {
    ///    println!("{:?} {:?}", iter.key(), iter.value());
    ///
    ///    iter.next();
    /// }
    ///
    /// // Read just the first key
    ///
    /// iter.seek_to_first();
    ///
    /// if iter.valid() {
    ///    println!("{:?} {:?}", iter.key(), iter.value());
    /// } else {
    ///    // There are no keys in the database
    /// }
    /// ```
    pub fn seek_to_first(&mut self) {
        unsafe {
            ffi::rocksdb_iter_seek_to_first(self.inner);
        }
    }

    /// Seeks to the last key in the database.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rocksdb::DB;
    ///
    /// let mut db = DB::open_default("path/for/rocksdb/storage6").unwrap();
    /// let mut iter = db.raw_iterator();
    ///
    /// // Iterate all keys from the end in reverse lexicographic order
    ///
    /// iter.seek_to_last();
    ///
    /// while iter.valid() {
    ///    println!("{:?} {:?}", iter.key(), iter.value());
    ///
    ///    iter.prev();
    /// }
    ///
    /// // Read just the last key
    ///
    /// iter.seek_to_last();
    ///
    /// if iter.valid() {
    ///    println!("{:?} {:?}", iter.key(), iter.value());
    /// } else {
    ///    // There are no keys in the database
    /// }
    /// ```
    pub fn seek_to_last(&mut self) {
        unsafe {
            ffi::rocksdb_iter_seek_to_last(self.inner);
        }
    }

    /// Seeks to the specified key or the first key that lexicographically follows it.
    ///
    /// This method will attempt to seek to the specified key. If that key does not exist, it will
    /// find and seek to the key that lexicographically follows it instead.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rocksdb::DB;
    ///
    /// let mut db = DB::open_default("path/for/rocksdb/storage7").unwrap();
    /// let mut iter = db.raw_iterator();
    ///
    /// // Read the first key that starts with 'a'
    ///
    /// iter.seek(b"a");
    ///
    /// if iter.valid() {
    ///    println!("{:?} {:?}", iter.key(), iter.value());
    /// } else {
    ///    // There are no keys in the database
    /// }
    /// ```
    pub fn seek(&mut self, key: &[u8]) {
        unsafe {
            ffi::rocksdb_iter_seek(
                self.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
            );
        }
    }

    /// Seeks to the specified key, or the first key that lexicographically precedes it.
    ///
    /// Like ``.seek()`` this method will attempt to seek to the specified key.
    /// The difference with ``.seek()`` is that if the specified key do not exist, this method will
    /// seek to key that lexicographically precedes it instead.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rocksdb::DB;
    ///
    /// let mut db = DB::open_default("path/for/rocksdb/storage8").unwrap();
    /// let mut iter = db.raw_iterator();
    ///
    /// // Read the last key that starts with 'a'
    ///
    /// iter.seek_for_prev(b"b");
    ///
    /// if iter.valid() {
    ///    println!("{:?} {:?}", iter.key(), iter.value());
    /// } else {
    ///    // There are no keys in the database
    /// }
    pub fn seek_for_prev(&mut self, key: &[u8]) {
        unsafe {
            ffi::rocksdb_iter_seek_for_prev(
                self.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
            );
        }
    }

    /// Seeks to the next key.
    ///
    /// Returns true if the iterator is valid after this operation.
    pub fn next(&mut self) {
        unsafe {
            ffi::rocksdb_iter_next(self.inner);
        }
    }

    /// Seeks to the previous key.
    ///
    /// Returns true if the iterator is valid after this operation.
    pub fn prev(&mut self) {
        unsafe {
            ffi::rocksdb_iter_prev(self.inner);
        }
    }

    /// Returns a slice to the internal buffer storing the current key.
    ///
    /// This may be slightly more performant to use than the standard ``.key()`` method
    /// as it does not copy the key. However, you must be careful to not use the buffer
    /// if the iterator's seek position is ever moved by any of the seek commands or the
    /// ``.next()`` and ``.previous()`` methods as the underlying buffer may be reused
    /// for something else or freed entirely.
    pub unsafe fn key_inner<'a>(&'a self) -> Option<&'a [u8]> {
        if self.valid() {
            let mut key_len: size_t = 0;
            let key_len_ptr: *mut size_t = &mut key_len;
            let key_ptr = ffi::rocksdb_iter_key(self.inner, key_len_ptr) as *const c_uchar;

            Some(slice::from_raw_parts(key_ptr, key_len as usize))
        } else {
            None
        }
    }

    /// Returns a copy of the current key.
    pub fn key(&self) -> Option<Vec<u8>> {
        unsafe { self.key_inner().map(|key| key.to_vec()) }
    }

    /// Returns a slice to the internal buffer storing the current value.
    ///
    /// This may be slightly more performant to use than the standard ``.value()`` method
    /// as it does not copy the value. However, you must be careful to not use the buffer
    /// if the iterator's seek position is ever moved by any of the seek commands or the
    /// ``.next()`` and ``.previous()`` methods as the underlying buffer may be reused
    /// for something else or freed entirely.
    pub unsafe fn value_inner<'a>(&'a self) -> Option<&'a [u8]> {
        if self.valid() {
            let mut val_len: size_t = 0;
            let val_len_ptr: *mut size_t = &mut val_len;
            let val_ptr = ffi::rocksdb_iter_value(self.inner, val_len_ptr) as *const c_uchar;

            Some(slice::from_raw_parts(val_ptr, val_len as usize))
        } else {
            None
        }
    }

    /// Returns a copy of the current value.
    pub fn value(&self) -> Option<Vec<u8>> {
        unsafe { self.value_inner().map(|value| value.to_vec()) }
    }
}

impl Drop for DBRawIterator {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_iter_destroy(self.inner);
        }
    }
}

impl DBIterator {
    fn new(db: &DB, readopts: &ReadOptions, mode: IteratorMode) -> DBIterator {
        let mut rv = DBIterator {
            raw: DBRawIterator::new(db, readopts),
            direction: Direction::Forward, // blown away by set_mode()
            just_seeked: false,
        };
        rv.set_mode(mode);
        rv
    }

    fn new_cf(
        db: &DB,
        cf_handle: ColumnFamily,
        readopts: &ReadOptions,
        mode: IteratorMode,
    ) -> Result<DBIterator, Error> {
        let mut rv = DBIterator {
            raw: try!(DBRawIterator::new_cf(db, cf_handle, readopts)),
            direction: Direction::Forward, // blown away by set_mode()
            just_seeked: false,
        };
        rv.set_mode(mode);
        Ok(rv)
    }

    pub fn set_mode(&mut self, mode: IteratorMode) {
        match mode {
            IteratorMode::Start => {
                self.raw.seek_to_first();
                self.direction = Direction::Forward;
            }
            IteratorMode::End => {
                self.raw.seek_to_last();
                self.direction = Direction::Reverse;
            }
            IteratorMode::From(key, Direction::Forward) => {
                self.raw.seek(key);
                self.direction = Direction::Forward;
            }
            IteratorMode::From(key, Direction::Reverse) => {
                self.raw.seek_for_prev(key);
                self.direction = Direction::Reverse;
            }
        };

        self.just_seeked = true;
    }

    pub fn valid(&self) -> bool {
        self.raw.valid()
    }
}

impl Iterator for DBIterator {
    type Item = KVBytes;

    fn next(&mut self) -> Option<KVBytes> {
        // Initial call to next() after seeking should not move the iterator
        // or the first item will not be returned
        if !self.just_seeked {
            match self.direction {
                Direction::Forward => self.raw.next(),
                Direction::Reverse => self.raw.prev(),
            }
        } else {
            self.just_seeked = false;
        }

        if self.raw.valid() {
            // .key() and .value() only ever return None if valid == false, which we've just cheked
            Some((
                self.raw.key().unwrap().into_boxed_slice(),
                self.raw.value().unwrap().into_boxed_slice(),
            ))
        } else {
            None
        }
    }
}

impl Into<DBRawIterator> for DBIterator {
    fn into(self) -> DBRawIterator {
        self.raw
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

    pub fn iterator_cf(
        &self,
        cf_handle: ColumnFamily,
        mode: IteratorMode,
    ) -> Result<DBIterator, Error> {
        let mut readopts = ReadOptions::default();
        readopts.set_snapshot(self);
        DBIterator::new_cf(self.db, cf_handle, &readopts, mode)
    }

    pub fn raw_iterator(&self) -> DBRawIterator {
        let mut readopts = ReadOptions::default();
        readopts.set_snapshot(self);
        DBRawIterator::new(self.db, &readopts)
    }

    pub fn raw_iterator_cf(&self, cf_handle: ColumnFamily) -> Result<DBRawIterator, Error> {
        let mut readopts = ReadOptions::default();
        readopts.set_snapshot(self);
        DBRawIterator::new_cf(self.db, cf_handle, &readopts)
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<DBVector>, Error> {
        let mut readopts = ReadOptions::default();
        readopts.set_snapshot(self);
        self.db.get_opt(key, &readopts)
    }

    pub fn get_cf(&self, cf: ColumnFamily, key: &[u8]) -> Result<Option<DBVector>, Error> {
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

impl ColumnFamilyDescriptor {
    // Create a new column family descriptor with the specified name and options.
    pub fn new<S>(name: S, options: Options) -> Self where S: Into<String> {
        ColumnFamilyDescriptor {
            name: name.into(),
            options
        }
    }
}

impl DB {
    /// Open a database with default options.
    pub fn open_default<P: AsRef<Path>>(path: P) -> Result<DB, Error> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        DB::open(&opts, path)
    }

    /// Open the database with the specified options.
    pub fn open<P: AsRef<Path>>(opts: &Options, path: P) -> Result<DB, Error> {
        DB::open_cf(opts, path, &[])
    }

    /// Open a database with the given database options and column family names.
    ///
    /// Column families opened using this function will be created with default `Options`.
    pub fn open_cf<P: AsRef<Path>>(opts: &Options, path: P, cfs: &[&str]) -> Result<DB, Error> {
        let cfs_v = cfs.to_vec().iter().map(|name| ColumnFamilyDescriptor::new(*name, Options::default())).collect();

        DB::open_cf_descriptors(opts, path, cfs_v)
    }

    /// Open a database with the given database options and column family names/options.
    pub fn open_cf_descriptors<P: AsRef<Path>>(opts: &Options, path: P, cfs: Vec<ColumnFamilyDescriptor>) -> Result<DB, Error> {
        let path = path.as_ref();
        let cpath = match CString::new(path.to_string_lossy().as_bytes()) {
            Ok(c) => c,
            Err(_) => {
                return Err(Error::new(
                    "Failed to convert path to CString \
                                       when opening DB."
                        .to_owned(),
                ))
            }
        };

        if let Err(e) = fs::create_dir_all(&path) {
            return Err(Error::new(format!(
                "Failed to create RocksDB\
                                           directory: `{:?}`.",
                e
            )));
        }

        let db: *mut ffi::rocksdb_t;
        let mut cf_map = BTreeMap::new();

        if cfs.len() == 0 {
            unsafe {
                db = ffi_try!(ffi::rocksdb_open(opts.inner, cpath.as_ptr() as *const _,));
            }
        } else {
            let mut cfs_v = cfs;
            // Always open the default column family.
            if !cfs_v.iter().any(|cf| cf.name == "default") {
                cfs_v.push(ColumnFamilyDescriptor {
                    name: String::from("default"),
                    options: Options::default()
                });
            }
            // We need to store our CStrings in an intermediate vector
            // so that their pointers remain valid.
            let c_cfs: Vec<CString> = cfs_v
                .iter()
                .map(|cf| CString::new(cf.name.as_bytes()).unwrap())
                .collect();

            let mut cfnames: Vec<_> = c_cfs.iter().map(|cf| cf.as_ptr()).collect();

            // These handles will be populated by DB.
            let mut cfhandles: Vec<_> = cfs_v.iter().map(|_| ptr::null_mut()).collect();

            let mut cfopts: Vec<_> = cfs_v.iter()
                .map(|cf| cf.options.inner as *const _)
                .collect();

            unsafe {
                db = ffi_try!(ffi::rocksdb_open_column_families(
                    opts.inner,
                    cpath.as_ptr(),
                    cfs_v.len() as c_int,
                    cfnames.as_mut_ptr(),
                    cfopts.as_mut_ptr(),
                    cfhandles.as_mut_ptr(),));
            }

            for handle in &cfhandles {
                if handle.is_null() {
                    return Err(Error::new(
                        "Received null column family \
                                           handle from DB."
                            .to_owned(),
                    ));
                }
            }

            for (n, h) in cfs_v.iter().zip(cfhandles) {
                cf_map.insert(n.name.clone(), ColumnFamily { inner: h });
            }
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

    pub fn list_cf<P: AsRef<Path>>(opts: &Options, path: P) -> Result<Vec<String>, Error> {
        let cpath = match CString::new(path.as_ref().to_string_lossy().as_bytes()) {
            Ok(c) => c,
            Err(_) => {
                return Err(Error::new(
                    "Failed to convert path to CString \
                                       when opening DB."
                        .to_owned(),
                ))
            }
        };

        let mut length = 0;

        unsafe {
            let ptr = ffi_try!(ffi::rocksdb_list_column_families(
                opts.inner,
                cpath.as_ptr() as *const _,
                &mut length,
            ));

            let vec = slice::from_raw_parts(ptr, length)
                .iter()
                .map(|ptr| CStr::from_ptr(*ptr).to_string_lossy().into_owned())
                .collect();
            ffi::rocksdb_list_column_families_destroy(ptr, length);
            Ok(vec)
        }
    }


    pub fn destroy<P: AsRef<Path>>(opts: &Options, path: P) -> Result<(), Error> {
        let cpath = CString::new(path.as_ref().to_string_lossy().as_bytes()).unwrap();
        unsafe {
            ffi_try!(ffi::rocksdb_destroy_db(opts.inner, cpath.as_ptr(),));
        }
        Ok(())
    }

    pub fn repair<P: AsRef<Path>>(opts: Options, path: P) -> Result<(), Error> {
        let cpath = CString::new(path.as_ref().to_string_lossy().as_bytes()).unwrap();
        unsafe {
            ffi_try!(ffi::rocksdb_repair_db(opts.inner, cpath.as_ptr(),));
        }
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path.as_path()
    }

    pub fn write_opt(&self, batch: WriteBatch, writeopts: &WriteOptions) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_write(self.inner, writeopts.inner, batch.inner,));
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

    pub fn get_opt(&self, key: &[u8], readopts: &ReadOptions) -> Result<Option<DBVector>, Error> {
        if readopts.inner.is_null() {
            return Err(Error::new(
                "Unable to create RocksDB read options. \
                                   This is a fairly trivial call, and its \
                                   failure may be indicative of a \
                                   mis-compiled or mis-loaded RocksDB \
                                   library."
                    .to_owned(),
            ));
        }

        unsafe {
            let mut val_len: size_t = 0;
            let val = ffi_try!(ffi::rocksdb_get(
                self.inner,
                readopts.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
                &mut val_len,
            )) as *mut u8;
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

    pub fn get_cf_opt(
        &self,
        cf: ColumnFamily,
        key: &[u8],
        readopts: &ReadOptions,
    ) -> Result<Option<DBVector>, Error> {
        if readopts.inner.is_null() {
            return Err(Error::new(
                "Unable to create RocksDB read options. \
                                   This is a fairly trivial call, and its \
                                   failure may be indicative of a \
                                   mis-compiled or mis-loaded RocksDB \
                                   library."
                    .to_owned(),
            ));
        }

        unsafe {
            let mut val_len: size_t = 0;
            let val = ffi_try!(ffi::rocksdb_get_cf(
                self.inner,
                readopts.inner,
                cf.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
                &mut val_len,
            )) as *mut u8;
            if val.is_null() {
                Ok(None)
            } else {
                Ok(Some(DBVector::from_c(val, val_len)))
            }
        }
    }

    pub fn get_cf(&self, cf: ColumnFamily, key: &[u8]) -> Result<Option<DBVector>, Error> {
        self.get_cf_opt(cf, key, &ReadOptions::default())
    }

    pub fn create_cf(&mut self, name: &str, opts: &Options) -> Result<ColumnFamily, Error> {
        let cname = match CString::new(name.as_bytes()) {
            Ok(c) => c,
            Err(_) => {
                return Err(Error::new(
                    "Failed to convert path to CString \
                                       when opening rocksdb"
                        .to_owned(),
                ))
            }
        };
        let cf = unsafe {
            let cf_handler = ffi_try!(ffi::rocksdb_create_column_family(
                self.inner,
                opts.inner,
                cname.as_ptr(),
            ));
            let cf = ColumnFamily { inner: cf_handler };
            self.cfs.insert(name.to_string(), cf);
            cf
        };
        Ok(cf)
    }

    pub fn drop_cf(&mut self, name: &str) -> Result<(), Error> {
        let cf = self.cfs.get(name);
        if cf.is_none() {
            return Err(Error::new(
                format!("Invalid column family: {}", name).to_owned(),
            ));
        }
        unsafe {
            ffi_try!(ffi::rocksdb_drop_column_family(
                self.inner,
                cf.unwrap().inner,
            ));
        }
        Ok(())
    }

    /// Return the underlying column family handle.
    pub fn cf_handle(&self, name: &str) -> Option<ColumnFamily> {
        self.cfs.get(name).cloned()
    }

    pub fn iterator(&self, mode: IteratorMode) -> DBIterator {
        let opts = ReadOptions::default();
        DBIterator::new(self, &opts, mode)
    }

    /// Opens an interator with `set_total_order_seek` enabled.
    /// This must be used to iterate across prefixes when `set_memtable_factory` has been called
    /// with a Hash-based implementation.
    pub fn full_iterator(&self, mode: IteratorMode) -> DBIterator {
        let mut opts = ReadOptions::default();
        opts.set_total_order_seek(true);
        DBIterator::new(self, &opts, mode)
    }

    pub fn prefix_iterator<'a>(&self, prefix: &'a [u8]) -> DBIterator {
        let mut opts = ReadOptions::default();
        opts.set_prefix_same_as_start(true);
        DBIterator::new(self, &opts, IteratorMode::From(prefix, Direction::Forward))
    }

    pub fn iterator_cf(
        &self,
        cf_handle: ColumnFamily,
        mode: IteratorMode,
    ) -> Result<DBIterator, Error> {
        let opts = ReadOptions::default();
        DBIterator::new_cf(self, cf_handle, &opts, mode)
    }

    pub fn full_iterator_cf(
        &self,
        cf_handle: ColumnFamily,
        mode: IteratorMode,
    ) -> Result<DBIterator, Error> {
        let mut opts = ReadOptions::default();
        opts.set_total_order_seek(true);
        DBIterator::new_cf(self, cf_handle, &opts, mode)
    }

    pub fn prefix_iterator_cf<'a>(
        &self,
        cf_handle: ColumnFamily,
        prefix: &'a [u8]
    ) -> Result<DBIterator, Error> {
        let mut opts = ReadOptions::default();
        opts.set_prefix_same_as_start(true);
        DBIterator::new_cf(self, cf_handle, &opts, IteratorMode::From(prefix, Direction::Forward))
    }

    pub fn raw_iterator(&self) -> DBRawIterator {
        let opts = ReadOptions::default();
        DBRawIterator::new(self, &opts)
    }

    pub fn raw_iterator_cf(&self, cf_handle: ColumnFamily) -> Result<DBRawIterator, Error> {
        let opts = ReadOptions::default();
        DBRawIterator::new_cf(self, cf_handle, &opts)
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot::new(self)
    }

    pub fn put_opt(&self, key: &[u8], value: &[u8], writeopts: &WriteOptions) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_put(
                self.inner,
                writeopts.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
                value.as_ptr() as *const c_char,
                value.len() as size_t,
            ));
            Ok(())
        }
    }

    pub fn put_cf_opt(
        &self,
        cf: ColumnFamily,
        key: &[u8],
        value: &[u8],
        writeopts: &WriteOptions,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_put_cf(
                self.inner,
                writeopts.inner,
                cf.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
                value.as_ptr() as *const c_char,
                value.len() as size_t,
            ));
            Ok(())
        }
    }

    pub fn merge_opt(
        &self,
        key: &[u8],
        value: &[u8],
        writeopts: &WriteOptions,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_merge(
                self.inner,
                writeopts.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
                value.as_ptr() as *const c_char,
                value.len() as size_t,
            ));
            Ok(())
        }
    }

    pub fn merge_cf_opt(
        &self,
        cf: ColumnFamily,
        key: &[u8],
        value: &[u8],
        writeopts: &WriteOptions,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_merge_cf(
                self.inner,
                writeopts.inner,
                cf.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
                value.as_ptr() as *const c_char,
                value.len() as size_t,
            ));
            Ok(())
        }
    }

    pub fn delete_opt(&self, key: &[u8], writeopts: &WriteOptions) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_delete(
                self.inner,
                writeopts.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
            ));
            Ok(())
        }
    }

    pub fn delete_cf_opt(
        &self,
        cf: ColumnFamily,
        key: &[u8],
        writeopts: &WriteOptions,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_delete_cf(
                self.inner,
                writeopts.inner,
                cf.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
            ));
            Ok(())
        }
    }

    pub fn put(&self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        self.put_opt(key, value, &WriteOptions::default())
    }

    pub fn put_cf(&self, cf: ColumnFamily, key: &[u8], value: &[u8]) -> Result<(), Error> {
        self.put_cf_opt(cf, key, value, &WriteOptions::default())
    }

    pub fn merge(&self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        self.merge_opt(key, value, &WriteOptions::default())
    }

    pub fn merge_cf(&self, cf: ColumnFamily, key: &[u8], value: &[u8]) -> Result<(), Error> {
        self.merge_cf_opt(cf, key, value, &WriteOptions::default())
    }

    pub fn delete(&self, key: &[u8]) -> Result<(), Error> {
        self.delete_opt(key, &WriteOptions::default())
    }

    pub fn delete_cf(&self, cf: ColumnFamily, key: &[u8]) -> Result<(), Error> {
        self.delete_cf_opt(cf, key, &WriteOptions::default())
    }

    pub fn compact_range(&self, start: Option<&[u8]>, end: Option<&[u8]>) {
        unsafe {
            ffi::rocksdb_compact_range(
                self.inner,
                opt_bytes_to_ptr(start),
                start.map_or(0, |s| s.len()) as size_t,
                opt_bytes_to_ptr(end),
                end.map_or(0, |e| e.len()) as size_t,
            );
        }
    }

    pub fn compact_range_cf(&self, cf: ColumnFamily, start: Option<&[u8]>, end: Option<&[u8]>) {
        unsafe {
            ffi::rocksdb_compact_range_cf(
                self.inner,
                cf.inner,
                opt_bytes_to_ptr(start),
                start.map_or(0, |s| s.len()) as size_t,
                opt_bytes_to_ptr(end),
                end.map_or(0, |e| e.len()) as size_t,
            );
        }
    }
}

impl WriteBatch {
    pub fn len(&self) -> usize {
        unsafe { ffi::rocksdb_writebatch_count(self.inner) as usize }
    }

    /// Return WriteBatch serialized size (in bytes).
    pub fn size_in_bytes(&self) -> usize {
        unsafe {
            let mut batch_size: size_t = 0;
            ffi::rocksdb_writebatch_data(self.inner, &mut batch_size);
            batch_size as usize
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Insert a value into the database under the given key.
    pub fn put(&mut self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        unsafe {
            ffi::rocksdb_writebatch_put(
                self.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
                value.as_ptr() as *const c_char,
                value.len() as size_t,
            );
            Ok(())
        }
    }

    pub fn put_cf(&mut self, cf: ColumnFamily, key: &[u8], value: &[u8]) -> Result<(), Error> {
        unsafe {
            ffi::rocksdb_writebatch_put_cf(
                self.inner,
                cf.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
                value.as_ptr() as *const c_char,
                value.len() as size_t,
            );
            Ok(())
        }
    }

    pub fn merge(&mut self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        unsafe {
            ffi::rocksdb_writebatch_merge(
                self.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
                value.as_ptr() as *const c_char,
                value.len() as size_t,
            );
            Ok(())
        }
    }

    pub fn merge_cf(&mut self, cf: ColumnFamily, key: &[u8], value: &[u8]) -> Result<(), Error> {
        unsafe {
            ffi::rocksdb_writebatch_merge_cf(
                self.inner,
                cf.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
                value.as_ptr() as *const c_char,
                value.len() as size_t,
            );
            Ok(())
        }
    }

    /// Remove the database entry for key.
    ///
    /// Returns an error if the key was not found.
    pub fn delete(&mut self, key: &[u8]) -> Result<(), Error> {
        unsafe {
            ffi::rocksdb_writebatch_delete(
                self.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
            );
            Ok(())
        }
    }

    pub fn delete_cf(&mut self, cf: ColumnFamily, key: &[u8]) -> Result<(), Error> {
        unsafe {
            ffi::rocksdb_writebatch_delete_cf(
                self.inner,
                cf.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
            );
            Ok(())
        }
    }
}

impl Default for WriteBatch {
    fn default() -> WriteBatch {
        WriteBatch { inner: unsafe { ffi::rocksdb_writebatch_create() } }
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
                ffi::rocksdb_column_family_handle_destroy(cf.inner);
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
            ffi::rocksdb_readoptions_set_iterate_upper_bound(
                self.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
            );
        }
    }

    pub fn set_prefix_same_as_start(&mut self, v: bool) {
        unsafe {
            ffi::rocksdb_readoptions_set_prefix_same_as_start(self.inner, v as c_uchar)
        }
    }

    pub fn set_total_order_seek(&mut self, v:bool) {
        unsafe {
            ffi::rocksdb_readoptions_set_total_order_seek(self.inner, v as c_uchar)
        }
    }
}

impl Default for ReadOptions {
    fn default() -> ReadOptions {
        unsafe { ReadOptions { inner: ffi::rocksdb_readoptions_create() } }
    }
}

/// Vector of bytes stored in the database.
///
/// This is a `C` allocated byte array and a length value.
/// Normal usage would be to utilize the fact it implements `Deref<[u8]>` and use it as
/// a slice.
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
    /// Used internally to create a DBVector from a `C` memory block
    ///
    /// # Unsafe
    /// Requires that the ponter be allocated by a `malloc` derivative (all C libraries), and
    /// `val_len` be the length of the C array to be safe (since `sizeof(u8) = 1`).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let buf_len: libc::size_t = unsafe { mem::uninitialized() };
    /// // Assume the function fills buf_len with the length of the returned array
    /// let buf: *mut u8 = unsafe { ffi_function_returning_byte_array(&buf_len) };
    /// DBVector::from_c(buf, buf_len)
    /// ```
    pub unsafe fn from_c(val: *mut u8, val_len: size_t) -> DBVector {
        DBVector {
            base: val,
            len: val_len as usize,
        }
    }

    /// Convenience function to attempt to reinterperet value as string.
    ///
    /// implemented as `str::from_utf8(&self[..])`
    pub fn to_utf8(&self) -> Option<&str> {
        str::from_utf8(self.deref()).ok()
    }
}

#[test]
fn test_db_vector() {
    use std::mem;
    let len: size_t = 4;
    let data: *mut u8 = unsafe { mem::transmute(libc::calloc(len, mem::size_of::<u8>())) };
    let v = unsafe { DBVector::from_c(data, len) };
    let ctrl = [0u8, 0, 0, 0];
    assert_eq!(&*v, &ctrl[..]);
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
    let _db = DB::open_default(path).unwrap();
    let opts = Options::default();
    // The DB will still be open when we try to destroy it and the lock should fail.
    match DB::destroy(&opts, path) {
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
        let db = DB::open_default(path).unwrap();
        {
            // test put
            let mut batch = WriteBatch::default();
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
            let mut batch = WriteBatch::default();
            let _ = batch.delete(b"k1");
            assert_eq!(batch.len(), 1);
            assert!(!batch.is_empty());
            let p = db.write(batch);
            assert!(p.is_ok());
            assert!(db.get(b"k1").unwrap().is_none());
        }
        {
            // test size_in_bytes
            let mut batch = WriteBatch::default();
            let before = batch.size_in_bytes();
            let _ = batch.put(b"k1", b"v1234567890");
            let after = batch.size_in_bytes();
            assert!(before + 10 <= after);
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
            println!(
                "Hello {}: {}",
                str::from_utf8(&*k).unwrap(),
                str::from_utf8(&*v).unwrap()
            );
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
