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

use crate::{
    ffi,
    ffi_util::{opt_bytes_to_ptr, to_cpath},
    ColumnFamily, ColumnFamilyDescriptor, DBIterator, DBPinnableSlice, DBRawIterator,
    DBWALIterator, Direction, Error, FlushOptions, IngestExternalFileOptions, IteratorMode,
    Options, ReadOptions, Snapshot, WriteBatch, WriteOptions, DEFAULT_COLUMN_FAMILY_NAME,
};

use libc::{self, c_char, c_int, c_uchar, c_void, size_t};
use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::fmt;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::ptr;
use std::slice;
use std::str;
use std::time::Duration;

/// A RocksDB database.
///
/// See crate level documentation for a simple usage example.
pub struct DB {
    pub(crate) inner: *mut ffi::rocksdb_t,
    cfs: BTreeMap<String, ColumnFamily>,
    path: PathBuf,
}

// Safety note: auto-implementing Send on most db-related types is prevented by the inner FFI
// pointer. In most cases, however, this pointer is Send-safe because it is never aliased and
// rocksdb internally does not rely on thread-local information for its user-exposed types.
unsafe impl Send for DB {}

// Sync is similarly safe for many types because they do not expose interior mutability, and their
// use within the rocksdb library is generally behind a const reference
unsafe impl Sync for DB {}

// Specifies whether open DB for read only.
enum AccessType<'a> {
    ReadWrite,
    ReadOnly { error_if_log_file_exist: bool },
    Secondary { secondary_path: &'a Path },
    WithTTL { ttl: Duration },
}

impl DB {
    /// Opens a database with default options.
    pub fn open_default<P: AsRef<Path>>(path: P) -> Result<DB, Error> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        DB::open(&opts, path)
    }

    /// Opens the database with the specified options.
    pub fn open<P: AsRef<Path>>(opts: &Options, path: P) -> Result<DB, Error> {
        DB::open_cf(opts, path, None::<&str>)
    }

    /// Opens the database for read only with the specified options.
    pub fn open_for_read_only<P: AsRef<Path>>(
        opts: &Options,
        path: P,
        error_if_log_file_exist: bool,
    ) -> Result<DB, Error> {
        DB::open_cf_for_read_only(opts, path, None::<&str>, error_if_log_file_exist)
    }

    /// Opens the database as a secondary.
    pub fn open_as_secondary<P: AsRef<Path>>(
        opts: &Options,
        primary_path: P,
        secondary_path: P,
    ) -> Result<DB, Error> {
        DB::open_cf_as_secondary(opts, primary_path, secondary_path, None::<&str>)
    }

    /// Opens the database with a Time to Live compaction filter.
    pub fn open_with_ttl<P: AsRef<Path>>(
        opts: &Options,
        path: P,
        ttl: Duration,
    ) -> Result<DB, Error> {
        let c_path = to_cpath(&path)?;
        let db = DB::open_raw(opts, &c_path, &AccessType::WithTTL { ttl })?;
        if db.is_null() {
            return Err(Error::new("Could not initialize database.".to_owned()));
        }

        Ok(DB {
            inner: db,
            cfs: BTreeMap::new(),
            path: path.as_ref().to_path_buf(),
        })
    }

    /// Opens a database with the given database options and column family names.
    ///
    /// Column families opened using this function will be created with default `Options`.
    pub fn open_cf<P, I, N>(opts: &Options, path: P, cfs: I) -> Result<DB, Error>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = N>,
        N: AsRef<str>,
    {
        let cfs = cfs
            .into_iter()
            .map(|name| ColumnFamilyDescriptor::new(name.as_ref(), Options::default()));

        DB::open_cf_descriptors_internal(opts, path, cfs, &AccessType::ReadWrite)
    }

    /// Opens a database for read only with the given database options and column family names.
    pub fn open_cf_for_read_only<P, I, N>(
        opts: &Options,
        path: P,
        cfs: I,
        error_if_log_file_exist: bool,
    ) -> Result<DB, Error>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = N>,
        N: AsRef<str>,
    {
        let cfs = cfs
            .into_iter()
            .map(|name| ColumnFamilyDescriptor::new(name.as_ref(), Options::default()));

        DB::open_cf_descriptors_internal(
            opts,
            path,
            cfs,
            &AccessType::ReadOnly {
                error_if_log_file_exist,
            },
        )
    }

    /// Opens the database as a secondary with the given database options and column family names.
    pub fn open_cf_as_secondary<P, I, N>(
        opts: &Options,
        primary_path: P,
        secondary_path: P,
        cfs: I,
    ) -> Result<DB, Error>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = N>,
        N: AsRef<str>,
    {
        let cfs = cfs
            .into_iter()
            .map(|name| ColumnFamilyDescriptor::new(name.as_ref(), Options::default()));

        DB::open_cf_descriptors_internal(
            opts,
            primary_path,
            cfs,
            &AccessType::Secondary {
                secondary_path: secondary_path.as_ref(),
            },
        )
    }

    /// Opens a database with the given database options and column family descriptors.
    pub fn open_cf_descriptors<P, I>(opts: &Options, path: P, cfs: I) -> Result<DB, Error>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = ColumnFamilyDescriptor>,
    {
        DB::open_cf_descriptors_internal(opts, path, cfs, &AccessType::ReadWrite)
    }

    /// Internal implementation for opening RocksDB.
    fn open_cf_descriptors_internal<P, I>(
        opts: &Options,
        path: P,
        cfs: I,
        access_type: &AccessType,
    ) -> Result<DB, Error>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = ColumnFamilyDescriptor>,
    {
        let cfs: Vec<_> = cfs.into_iter().collect();

        let cpath = to_cpath(&path)?;

        if let Err(e) = fs::create_dir_all(&path) {
            return Err(Error::new(format!(
                "Failed to create RocksDB directory: `{:?}`.",
                e
            )));
        }

        let db: *mut ffi::rocksdb_t;
        let mut cf_map = BTreeMap::new();

        if cfs.is_empty() {
            db = DB::open_raw(opts, &cpath, access_type)?;
        } else {
            let mut cfs_v = cfs;
            // Always open the default column family.
            if !cfs_v.iter().any(|cf| cf.name == DEFAULT_COLUMN_FAMILY_NAME) {
                cfs_v.push(ColumnFamilyDescriptor {
                    name: String::from(DEFAULT_COLUMN_FAMILY_NAME),
                    options: Options::default(),
                });
            }
            // We need to store our CStrings in an intermediate vector
            // so that their pointers remain valid.
            let c_cfs: Vec<CString> = cfs_v
                .iter()
                .map(|cf| CString::new(cf.name.as_bytes()).unwrap())
                .collect();

            let cfnames: Vec<_> = c_cfs.iter().map(|cf| cf.as_ptr()).collect();

            // These handles will be populated by DB.
            let mut cfhandles: Vec<_> = cfs_v.iter().map(|_| ptr::null_mut()).collect();

            let cfopts: Vec<_> = cfs_v
                .iter()
                .map(|cf| cf.options.inner as *const _)
                .collect();

            db = DB::open_cf_raw(
                opts,
                &cpath,
                &cfs_v,
                &cfnames,
                &cfopts,
                &mut cfhandles,
                &access_type,
            )?;
            for handle in &cfhandles {
                if handle.is_null() {
                    return Err(Error::new(
                        "Received null column family handle from DB.".to_owned(),
                    ));
                }
            }

            for (cf_desc, inner) in cfs_v.iter().zip(cfhandles) {
                cf_map.insert(cf_desc.name.clone(), ColumnFamily { inner });
            }
        }

        if db.is_null() {
            return Err(Error::new("Could not initialize database.".to_owned()));
        }

        Ok(DB {
            inner: db,
            cfs: cf_map,
            path: path.as_ref().to_path_buf(),
        })
    }

    fn open_raw(
        opts: &Options,
        cpath: &CString,
        access_type: &AccessType,
    ) -> Result<*mut ffi::rocksdb_t, Error> {
        let db = unsafe {
            match *access_type {
                AccessType::ReadOnly {
                    error_if_log_file_exist,
                } => ffi_try!(ffi::rocksdb_open_for_read_only(
                    opts.inner,
                    cpath.as_ptr() as *const _,
                    error_if_log_file_exist as c_uchar,
                )),
                AccessType::ReadWrite => {
                    ffi_try!(ffi::rocksdb_open(opts.inner, cpath.as_ptr() as *const _))
                }
                AccessType::Secondary { secondary_path } => {
                    ffi_try!(ffi::rocksdb_open_as_secondary(
                        opts.inner,
                        cpath.as_ptr() as *const _,
                        to_cpath(secondary_path)?.as_ptr() as *const _,
                    ))
                }
                AccessType::WithTTL { ttl } => ffi_try!(ffi::rocksdb_open_with_ttl(
                    opts.inner,
                    cpath.as_ptr() as *const _,
                    ttl.as_secs() as c_int,
                )),
            }
        };
        Ok(db)
    }

    fn open_cf_raw(
        opts: &Options,
        cpath: &CString,
        cfs_v: &[ColumnFamilyDescriptor],
        cfnames: &[*const c_char],
        cfopts: &[*const ffi::rocksdb_options_t],
        cfhandles: &mut Vec<*mut ffi::rocksdb_column_family_handle_t>,
        access_type: &AccessType,
    ) -> Result<*mut ffi::rocksdb_t, Error> {
        let db = unsafe {
            match *access_type {
                AccessType::ReadOnly {
                    error_if_log_file_exist,
                } => ffi_try!(ffi::rocksdb_open_for_read_only_column_families(
                    opts.inner,
                    cpath.as_ptr(),
                    cfs_v.len() as c_int,
                    cfnames.as_ptr(),
                    cfopts.as_ptr(),
                    cfhandles.as_mut_ptr(),
                    error_if_log_file_exist as c_uchar,
                )),
                AccessType::ReadWrite => ffi_try!(ffi::rocksdb_open_column_families(
                    opts.inner,
                    cpath.as_ptr(),
                    cfs_v.len() as c_int,
                    cfnames.as_ptr(),
                    cfopts.as_ptr(),
                    cfhandles.as_mut_ptr(),
                )),
                AccessType::Secondary { secondary_path } => {
                    ffi_try!(ffi::rocksdb_open_as_secondary_column_families(
                        opts.inner,
                        cpath.as_ptr() as *const _,
                        to_cpath(secondary_path)?.as_ptr() as *const _,
                        cfs_v.len() as c_int,
                        cfnames.as_ptr(),
                        cfopts.as_ptr(),
                        cfhandles.as_mut_ptr(),
                    ))
                }
                _ => return Err(Error::new("Unsupported access type".to_owned())),
            }
        };
        Ok(db)
    }

    pub fn list_cf<P: AsRef<Path>>(opts: &Options, path: P) -> Result<Vec<String>, Error> {
        let cpath = to_cpath(path)?;
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
        let cpath = to_cpath(path)?;
        unsafe {
            ffi_try!(ffi::rocksdb_destroy_db(opts.inner, cpath.as_ptr()));
        }
        Ok(())
    }

    pub fn repair<P: AsRef<Path>>(opts: &Options, path: P) -> Result<(), Error> {
        let cpath = to_cpath(path)?;
        unsafe {
            ffi_try!(ffi::rocksdb_repair_db(opts.inner, cpath.as_ptr()));
        }
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path.as_path()
    }

    /// Flushes database memtables to SST files on the disk.
    pub fn flush_opt(&self, flushopts: &FlushOptions) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_flush(self.inner, flushopts.inner));
        }
        Ok(())
    }

    /// Flushes database memtables to SST files on the disk using default options.
    pub fn flush(&self) -> Result<(), Error> {
        self.flush_opt(&FlushOptions::default())
    }

    /// Flushes database memtables to SST files on the disk for a given column family.
    pub fn flush_cf_opt(&self, cf: &ColumnFamily, flushopts: &FlushOptions) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_flush_cf(self.inner, flushopts.inner, cf.inner));
        }
        Ok(())
    }

    /// Flushes database memtables to SST files on the disk for a given column family using default
    /// options.
    pub fn flush_cf(&self, cf: &ColumnFamily) -> Result<(), Error> {
        self.flush_cf_opt(cf, &FlushOptions::default())
    }

    pub fn write_opt(&self, batch: WriteBatch, writeopts: &WriteOptions) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_write(self.inner, writeopts.inner, batch.inner));
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

    /// Return the bytes associated with a key value with read options. If you only intend to use
    /// the vector returned temporarily, consider using [`get_pinned_opt`](#method.get_pinned_opt)
    /// to avoid unnecessary memory copy.
    pub fn get_opt<K: AsRef<[u8]>>(
        &self,
        key: K,
        readopts: &ReadOptions,
    ) -> Result<Option<Vec<u8>>, Error> {
        self.get_pinned_opt(key, readopts)
            .map(|x| x.map(|v| v.as_ref().to_vec()))
    }

    /// Return the bytes associated with a key value. If you only intend to use the vector returned
    /// temporarily, consider using [`get_pinned`](#method.get_pinned) to avoid unnecessary memory
    /// copy.
    pub fn get<K: AsRef<[u8]>>(&self, key: K) -> Result<Option<Vec<u8>>, Error> {
        self.get_opt(key.as_ref(), &ReadOptions::default())
    }

    /// Return the bytes associated with a key value and the given column family with read options.
    /// If you only intend to use the vector returned temporarily, consider using
    /// [`get_pinned_cf_opt`](#method.get_pinned_cf_opt) to avoid unnecessary memory.
    pub fn get_cf_opt<K: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        key: K,
        readopts: &ReadOptions,
    ) -> Result<Option<Vec<u8>>, Error> {
        self.get_pinned_cf_opt(cf, key, readopts)
            .map(|x| x.map(|v| v.as_ref().to_vec()))
    }

    /// Return the bytes associated with a key value and the given column family. If you only
    /// intend to use the vector returned temporarily, consider using
    /// [`get_pinned_cf`](#method.get_pinned_cf) to avoid unnecessary memory.
    pub fn get_cf<K: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        key: K,
    ) -> Result<Option<Vec<u8>>, Error> {
        self.get_cf_opt(cf, key.as_ref(), &ReadOptions::default())
    }

    /// Return the value associated with a key using RocksDB's PinnableSlice
    /// so as to avoid unnecessary memory copy.
    pub fn get_pinned_opt<K: AsRef<[u8]>>(
        &self,
        key: K,
        readopts: &ReadOptions,
    ) -> Result<Option<DBPinnableSlice>, Error> {
        if readopts.inner.is_null() {
            return Err(Error::new(
                "Unable to create RocksDB read options. This is a fairly trivial call, and its \
                 failure may be indicative of a mis-compiled or mis-loaded RocksDB library."
                    .to_owned(),
            ));
        }

        let key = key.as_ref();
        unsafe {
            let val = ffi_try!(ffi::rocksdb_get_pinned(
                self.inner,
                readopts.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
            ));
            if val.is_null() {
                Ok(None)
            } else {
                Ok(Some(DBPinnableSlice::from_c(val)))
            }
        }
    }

    /// Return the value associated with a key using RocksDB's PinnableSlice
    /// so as to avoid unnecessary memory copy. Similar to get_pinned_opt but
    /// leverages default options.
    pub fn get_pinned<K: AsRef<[u8]>>(&self, key: K) -> Result<Option<DBPinnableSlice>, Error> {
        self.get_pinned_opt(key, &ReadOptions::default())
    }

    /// Return the value associated with a key using RocksDB's PinnableSlice
    /// so as to avoid unnecessary memory copy. Similar to get_pinned_opt but
    /// allows specifying ColumnFamily
    pub fn get_pinned_cf_opt<K: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        key: K,
        readopts: &ReadOptions,
    ) -> Result<Option<DBPinnableSlice>, Error> {
        if readopts.inner.is_null() {
            return Err(Error::new(
                "Unable to create RocksDB read options. This is a fairly trivial call, and its \
                 failure may be indicative of a mis-compiled or mis-loaded RocksDB library."
                    .to_owned(),
            ));
        }

        let key = key.as_ref();
        unsafe {
            let val = ffi_try!(ffi::rocksdb_get_pinned_cf(
                self.inner,
                readopts.inner,
                cf.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
            ));
            if val.is_null() {
                Ok(None)
            } else {
                Ok(Some(DBPinnableSlice::from_c(val)))
            }
        }
    }

    /// Return the value associated with a key using RocksDB's PinnableSlice
    /// so as to avoid unnecessary memory copy. Similar to get_pinned_cf_opt but
    /// leverages default options.
    pub fn get_pinned_cf<K: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        key: K,
    ) -> Result<Option<DBPinnableSlice>, Error> {
        self.get_pinned_cf_opt(cf, key, &ReadOptions::default())
    }

    pub fn create_cf<N: AsRef<str>>(&mut self, name: N, opts: &Options) -> Result<(), Error> {
        let cf_name = if let Ok(c) = CString::new(name.as_ref().as_bytes()) {
            c
        } else {
            return Err(Error::new(
                "Failed to convert path to CString when creating cf".to_owned(),
            ));
        };
        unsafe {
            let inner = ffi_try!(ffi::rocksdb_create_column_family(
                self.inner,
                opts.inner,
                cf_name.as_ptr(),
            ));

            self.cfs
                .insert(name.as_ref().to_string(), ColumnFamily { inner });
        };
        Ok(())
    }

    pub fn drop_cf(&mut self, name: &str) -> Result<(), Error> {
        if let Some(cf) = self.cfs.remove(name) {
            unsafe {
                ffi_try!(ffi::rocksdb_drop_column_family(self.inner, cf.inner));
            }
            Ok(())
        } else {
            Err(Error::new(format!("Invalid column family: {}", name)))
        }
    }

    /// Return the underlying column family handle.
    pub fn cf_handle(&self, name: &str) -> Option<&ColumnFamily> {
        self.cfs.get(name)
    }

    pub fn iterator<'a: 'b, 'b>(&'a self, mode: IteratorMode) -> DBIterator<'b> {
        let readopts = ReadOptions::default();
        self.iterator_opt(mode, readopts)
    }

    pub fn iterator_opt<'a: 'b, 'b>(
        &'a self,
        mode: IteratorMode,
        readopts: ReadOptions,
    ) -> DBIterator<'b> {
        DBIterator::new(self, readopts, mode)
    }

    /// Opens an iterator using the provided ReadOptions.
    /// This is used when you want to iterate over a specific ColumnFamily with a modified ReadOptions
    pub fn iterator_cf_opt<'a: 'b, 'b>(
        &'a self,
        cf_handle: &ColumnFamily,
        readopts: ReadOptions,
        mode: IteratorMode,
    ) -> DBIterator<'b> {
        DBIterator::new_cf(self, cf_handle, readopts, mode)
    }

    /// Opens an iterator with `set_total_order_seek` enabled.
    /// This must be used to iterate across prefixes when `set_memtable_factory` has been called
    /// with a Hash-based implementation.
    pub fn full_iterator<'a: 'b, 'b>(&'a self, mode: IteratorMode) -> DBIterator<'b> {
        let mut opts = ReadOptions::default();
        opts.set_total_order_seek(true);
        DBIterator::new(self, opts, mode)
    }

    pub fn prefix_iterator<'a: 'b, 'b, P: AsRef<[u8]>>(&'a self, prefix: P) -> DBIterator<'b> {
        let mut opts = ReadOptions::default();
        opts.set_prefix_same_as_start(true);
        DBIterator::new(
            self,
            opts,
            IteratorMode::From(prefix.as_ref(), Direction::Forward),
        )
    }

    pub fn iterator_cf<'a: 'b, 'b>(
        &'a self,
        cf_handle: &ColumnFamily,
        mode: IteratorMode,
    ) -> DBIterator<'b> {
        let opts = ReadOptions::default();
        DBIterator::new_cf(self, cf_handle, opts, mode)
    }

    pub fn full_iterator_cf<'a: 'b, 'b>(
        &'a self,
        cf_handle: &ColumnFamily,
        mode: IteratorMode,
    ) -> DBIterator<'b> {
        let mut opts = ReadOptions::default();
        opts.set_total_order_seek(true);
        DBIterator::new_cf(self, cf_handle, opts, mode)
    }

    pub fn prefix_iterator_cf<'a: 'b, 'b, P: AsRef<[u8]>>(
        &'a self,
        cf_handle: &ColumnFamily,
        prefix: P,
    ) -> DBIterator<'b> {
        let mut opts = ReadOptions::default();
        opts.set_prefix_same_as_start(true);
        DBIterator::new_cf(
            self,
            cf_handle,
            opts,
            IteratorMode::From(prefix.as_ref(), Direction::Forward),
        )
    }

    /// Opens a raw iterator over the database, using the default read options
    pub fn raw_iterator<'a: 'b, 'b>(&'a self) -> DBRawIterator<'b> {
        let opts = ReadOptions::default();
        DBRawIterator::new(self, opts)
    }

    /// Opens a raw iterator over the given column family, using the default read options
    pub fn raw_iterator_cf<'a: 'b, 'b>(&'a self, cf_handle: &ColumnFamily) -> DBRawIterator<'b> {
        let opts = ReadOptions::default();
        DBRawIterator::new_cf(self, cf_handle, opts)
    }

    /// Opens a raw iterator over the database, using the given read options
    pub fn raw_iterator_opt<'a: 'b, 'b>(&'a self, readopts: ReadOptions) -> DBRawIterator<'b> {
        DBRawIterator::new(self, readopts)
    }

    /// Opens a raw iterator over the given column family, using the given read options
    pub fn raw_iterator_cf_opt<'a: 'b, 'b>(
        &'a self,
        cf_handle: &ColumnFamily,
        readopts: ReadOptions,
    ) -> DBRawIterator<'b> {
        DBRawIterator::new_cf(self, cf_handle, readopts)
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot::new(self)
    }

    pub fn put_opt<K, V>(&self, key: K, value: V, writeopts: &WriteOptions) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let key = key.as_ref();
        let value = value.as_ref();

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

    pub fn put_cf_opt<K, V>(
        &self,
        cf: &ColumnFamily,
        key: K,
        value: V,
        writeopts: &WriteOptions,
    ) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let key = key.as_ref();
        let value = value.as_ref();

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

    pub fn merge_opt<K, V>(&self, key: K, value: V, writeopts: &WriteOptions) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let key = key.as_ref();
        let value = value.as_ref();

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

    pub fn merge_cf_opt<K, V>(
        &self,
        cf: &ColumnFamily,
        key: K,
        value: V,
        writeopts: &WriteOptions,
    ) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let key = key.as_ref();
        let value = value.as_ref();

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

    pub fn delete_opt<K: AsRef<[u8]>>(
        &self,
        key: K,
        writeopts: &WriteOptions,
    ) -> Result<(), Error> {
        let key = key.as_ref();

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

    pub fn delete_cf_opt<K: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        key: K,
        writeopts: &WriteOptions,
    ) -> Result<(), Error> {
        let key = key.as_ref();

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

    /// Removes the database entries in the range `["from", "to")` using given write options.
    pub fn delete_range_cf_opt<K: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        from: K,
        to: K,
        writeopts: &WriteOptions,
    ) -> Result<(), Error> {
        let from = from.as_ref();
        let to = to.as_ref();

        unsafe {
            ffi_try!(ffi::rocksdb_delete_range_cf(
                self.inner,
                writeopts.inner,
                cf.inner,
                from.as_ptr() as *const c_char,
                from.len() as size_t,
                to.as_ptr() as *const c_char,
                to.len() as size_t,
            ));
            Ok(())
        }
    }

    pub fn put<K, V>(&self, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.put_opt(key.as_ref(), value.as_ref(), &WriteOptions::default())
    }

    pub fn put_cf<K, V>(&self, cf: &ColumnFamily, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.put_cf_opt(cf, key.as_ref(), value.as_ref(), &WriteOptions::default())
    }

    pub fn merge<K, V>(&self, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.merge_opt(key.as_ref(), value.as_ref(), &WriteOptions::default())
    }

    pub fn merge_cf<K, V>(&self, cf: &ColumnFamily, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.merge_cf_opt(cf, key.as_ref(), value.as_ref(), &WriteOptions::default())
    }

    pub fn delete<K: AsRef<[u8]>>(&self, key: K) -> Result<(), Error> {
        self.delete_opt(key.as_ref(), &WriteOptions::default())
    }

    pub fn delete_cf<K: AsRef<[u8]>>(&self, cf: &ColumnFamily, key: K) -> Result<(), Error> {
        self.delete_cf_opt(cf, key.as_ref(), &WriteOptions::default())
    }

    /// Removes the database entries in the range `["from", "to")` using default write options.
    pub fn delete_range_cf<K: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        from: K,
        to: K,
    ) -> Result<(), Error> {
        self.delete_range_cf_opt(cf, from, to, &WriteOptions::default())
    }

    pub fn compact_range<S: AsRef<[u8]>, E: AsRef<[u8]>>(&self, start: Option<S>, end: Option<E>) {
        unsafe {
            let start = start.as_ref().map(AsRef::as_ref);
            let end = end.as_ref().map(AsRef::as_ref);

            ffi::rocksdb_compact_range(
                self.inner,
                opt_bytes_to_ptr(start),
                start.map_or(0, |s| s.len()) as size_t,
                opt_bytes_to_ptr(end),
                end.map_or(0, |e| e.len()) as size_t,
            );
        }
    }

    pub fn compact_range_cf<S: AsRef<[u8]>, E: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        start: Option<S>,
        end: Option<E>,
    ) {
        unsafe {
            let start = start.as_ref().map(AsRef::as_ref);
            let end = end.as_ref().map(AsRef::as_ref);

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

    pub fn set_options(&self, opts: &[(&str, &str)]) -> Result<(), Error> {
        let copts = opts
            .iter()
            .map(|(name, value)| {
                let cname = match CString::new(name.as_bytes()) {
                    Ok(cname) => cname,
                    Err(e) => return Err(Error::new(format!("Invalid option name `{}`", e))),
                };
                let cvalue = match CString::new(value.as_bytes()) {
                    Ok(cvalue) => cvalue,
                    Err(e) => return Err(Error::new(format!("Invalid option value: `{}`", e))),
                };
                Ok((cname, cvalue))
            })
            .collect::<Result<Vec<(CString, CString)>, Error>>()?;

        let cnames: Vec<*const c_char> = copts.iter().map(|opt| opt.0.as_ptr()).collect();
        let cvalues: Vec<*const c_char> = copts.iter().map(|opt| opt.1.as_ptr()).collect();
        let count = opts.len() as i32;
        unsafe {
            ffi_try!(ffi::rocksdb_set_options(
                self.inner,
                count,
                cnames.as_ptr(),
                cvalues.as_ptr(),
            ));
        }
        Ok(())
    }

    /// Retrieves a RocksDB property by name.
    ///
    /// Full list of properties could be find
    /// [here](https://github.com/facebook/rocksdb/blob/08809f5e6cd9cc4bc3958dd4d59457ae78c76660/include/rocksdb/db.h#L428-L634).
    pub fn property_value(&self, name: &str) -> Result<Option<String>, Error> {
        let prop_name = match CString::new(name) {
            Ok(c) => c,
            Err(e) => {
                return Err(Error::new(format!(
                    "Failed to convert property name to CString: {}",
                    e
                )));
            }
        };

        unsafe {
            let value = ffi::rocksdb_property_value(self.inner, prop_name.as_ptr());
            if value.is_null() {
                return Ok(None);
            }

            let str_value = match CStr::from_ptr(value).to_str() {
                Ok(s) => s.to_owned(),
                Err(e) => {
                    return Err(Error::new(format!(
                        "Failed to convert property value to string: {}",
                        e
                    )));
                }
            };

            libc::free(value as *mut c_void);
            Ok(Some(str_value))
        }
    }

    /// Retrieves a RocksDB property by name, for a specific column family.
    ///
    /// Full list of properties could be find
    /// [here](https://github.com/facebook/rocksdb/blob/08809f5e6cd9cc4bc3958dd4d59457ae78c76660/include/rocksdb/db.h#L428-L634).
    pub fn property_value_cf(
        &self,
        cf: &ColumnFamily,
        name: &str,
    ) -> Result<Option<String>, Error> {
        let prop_name = match CString::new(name) {
            Ok(c) => c,
            Err(e) => {
                return Err(Error::new(format!(
                    "Failed to convert property name to CString: {}",
                    e
                )));
            }
        };

        unsafe {
            let value = ffi::rocksdb_property_value_cf(self.inner, cf.inner, prop_name.as_ptr());
            if value.is_null() {
                return Ok(None);
            }

            let str_value = match CStr::from_ptr(value).to_str() {
                Ok(s) => s.to_owned(),
                Err(e) => {
                    return Err(Error::new(format!(
                        "Failed to convert property value to string: {}",
                        e
                    )));
                }
            };

            libc::free(value as *mut c_void);
            Ok(Some(str_value))
        }
    }

    /// Retrieves a RocksDB property and casts it to an integer.
    ///
    /// Full list of properties that return int values could be find
    /// [here](https://github.com/facebook/rocksdb/blob/08809f5e6cd9cc4bc3958dd4d59457ae78c76660/include/rocksdb/db.h#L654-L689).
    pub fn property_int_value(&self, name: &str) -> Result<Option<u64>, Error> {
        match self.property_value(name) {
            Ok(Some(value)) => match value.parse::<u64>() {
                Ok(int_value) => Ok(Some(int_value)),
                Err(e) => Err(Error::new(format!(
                    "Failed to convert property value to int: {}",
                    e
                ))),
            },
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Retrieves a RocksDB property for a specific column family and casts it to an integer.
    ///
    /// Full list of properties that return int values could be find
    /// [here](https://github.com/facebook/rocksdb/blob/08809f5e6cd9cc4bc3958dd4d59457ae78c76660/include/rocksdb/db.h#L654-L689).
    pub fn property_int_value_cf(
        &self,
        cf: &ColumnFamily,
        name: &str,
    ) -> Result<Option<u64>, Error> {
        match self.property_value_cf(cf, name) {
            Ok(Some(value)) => match value.parse::<u64>() {
                Ok(int_value) => Ok(Some(int_value)),
                Err(e) => Err(Error::new(format!(
                    "Failed to convert property value to int: {}",
                    e
                ))),
            },
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// The sequence number of the most recent transaction.
    pub fn latest_sequence_number(&self) -> u64 {
        unsafe { ffi::rocksdb_get_latest_sequence_number(self.inner) }
    }

    /// Iterate over batches of write operations since a given sequence.
    ///
    /// Produce an iterator that will provide the batches of write operations
    /// that have occurred since the given sequence (see
    /// `latest_sequence_number()`). Use the provided iterator to retrieve each
    /// (`u64`, `WriteBatch`) tuple, and then gather the individual puts and
    /// deletes using the `WriteBatch::iterate()` function.
    ///
    /// Calling `get_updates_since()` with a sequence number that is out of
    /// bounds will return an error.
    pub fn get_updates_since(&self, seq_number: u64) -> Result<DBWALIterator, Error> {
        unsafe {
            // rocksdb_wal_readoptions_t does not appear to have any functions
            // for creating and destroying it; fortunately we can pass a nullptr
            // here to get the default behavior
            let opts: *const ffi::rocksdb_wal_readoptions_t = ptr::null();
            let iter = ffi_try!(ffi::rocksdb_get_updates_since(self.inner, seq_number, opts));
            Ok(DBWALIterator { inner: iter })
        }
    }

    /// Tries to catch up with the primary by reading as much as possible from the
    /// log files.
    pub fn try_catch_up_with_primary(&self) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_try_catch_up_with_primary(self.inner));
        }
        Ok(())
    }

    /// Loads a list of external SST files created with SstFileWriter into the DB with default opts
    pub fn ingest_external_file<P: AsRef<Path>>(&self, paths: Vec<P>) -> Result<(), Error> {
        let opts = IngestExternalFileOptions::default();
        self.ingest_external_file_opts(&opts, paths)
    }

    /// Loads a list of external SST files created with SstFileWriter into the DB
    pub fn ingest_external_file_opts<P: AsRef<Path>>(
        &self,
        opts: &IngestExternalFileOptions,
        paths: Vec<P>,
    ) -> Result<(), Error> {
        let paths_v: Vec<CString> = paths
            .iter()
            .map(|path| to_cpath(&path))
            .collect::<Result<Vec<_>, _>>()?;

        let cpaths: Vec<_> = paths_v.iter().map(|path| path.as_ptr()).collect();

        self.ingest_external_file_raw(&opts, &paths_v, &cpaths)
    }

    /// Loads a list of external SST files created with SstFileWriter into the DB for given Column Family
    /// with default opts
    pub fn ingest_external_file_cf<P: AsRef<Path>>(
        &self,
        cf: &ColumnFamily,
        paths: Vec<P>,
    ) -> Result<(), Error> {
        let opts = IngestExternalFileOptions::default();
        self.ingest_external_file_cf_opts(&cf, &opts, paths)
    }

    /// Loads a list of external SST files created with SstFileWriter into the DB for given Column Family
    pub fn ingest_external_file_cf_opts<P: AsRef<Path>>(
        &self,
        cf: &ColumnFamily,
        opts: &IngestExternalFileOptions,
        paths: Vec<P>,
    ) -> Result<(), Error> {
        let paths_v: Vec<CString> = paths
            .iter()
            .map(|path| to_cpath(&path))
            .collect::<Result<Vec<_>, _>>()?;

        let cpaths: Vec<_> = paths_v.iter().map(|path| path.as_ptr()).collect();

        self.ingest_external_file_raw_cf(&cf, &opts, &paths_v, &cpaths)
    }

    fn ingest_external_file_raw(
        &self,
        opts: &IngestExternalFileOptions,
        paths_v: &[CString],
        cpaths: &[*const c_char],
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_ingest_external_file(
                self.inner,
                cpaths.as_ptr(),
                paths_v.len(),
                opts.inner as *const _
            ));
            Ok(())
        }
    }

    fn ingest_external_file_raw_cf(
        &self,
        cf: &ColumnFamily,
        opts: &IngestExternalFileOptions,
        paths_v: &[CString],
        cpaths: &[*const c_char],
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_ingest_external_file_cf(
                self.inner,
                cf.inner,
                cpaths.as_ptr(),
                paths_v.len(),
                opts.inner as *const _
            ));
            Ok(())
        }
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

#[test]
fn test_open_for_read_only() {
    let path = "_rust_rocksdb_test_open_for_read_only";
    {
        let db = DB::open_default(path).unwrap();
        db.put(b"k1", b"v1").unwrap();
    }
    {
        let opts = Options::default();
        let error_if_log_file_exist = false;
        let db = DB::open_for_read_only(&opts, path, error_if_log_file_exist).unwrap();
        assert_eq!(db.get(b"k1").unwrap().unwrap(), b"v1");
        assert!(db.put(b"k2", b"v2").is_err());
    }
    let opts = Options::default();
    assert!(DB::destroy(&opts, path).is_ok());
}

#[test]
fn test_open_cf_for_read_only() {
    let path = "_rust_rocksdb_test_open_cf_for_read_only";
    let cfs = vec!["cf1"];
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        let db = DB::open_cf(&opts, path, cfs.clone()).unwrap();
        let cf1 = db.cf_handle("cf1").unwrap();
        db.put_cf(cf1, b"k1", b"v1").unwrap();
    }
    {
        let opts = Options::default();
        let error_if_log_file_exist = false;
        let db = DB::open_cf_for_read_only(&opts, path, cfs, error_if_log_file_exist).unwrap();
        let cf1 = db.cf_handle("cf1").unwrap();
        assert_eq!(db.get_cf(cf1, b"k1").unwrap().unwrap(), b"v1");
        assert!(db.put_cf(cf1, b"k2", b"v2").is_err());
    }
    let opts = Options::default();
    assert!(DB::destroy(&opts, path).is_ok());
}

#[test]
fn external() {
    let path = "_rust_rocksdb_externaltest";
    {
        let db = DB::open_default(path).unwrap();
        let p = db.put(b"k1", b"v1111");
        assert!(p.is_ok());
        let r: Result<Option<Vec<u8>>, Error> = db.get(b"k1");
        assert_eq!(r.unwrap().unwrap(), b"v1111");
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
    {
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
    let opts = Options::default();
    let result = DB::destroy(&opts, path);
    assert!(result.is_ok());
}

#[test]
fn writebatch_works() {
    let path = "_rust_rocksdb_writebacktest";
    {
        let db = DB::open_default(path).unwrap();
        {
            // test putx
            let mut batch = WriteBatch::default();
            assert!(db.get(b"k1").unwrap().is_none());
            assert_eq!(batch.len(), 0);
            assert!(batch.is_empty());
            batch.put(b"k1", b"v1111");
            batch.put(b"k2", b"v2222");
            batch.put(b"k3", b"v3333");
            assert_eq!(batch.len(), 3);
            assert!(!batch.is_empty());
            assert!(db.get(b"k1").unwrap().is_none());
            let p = db.write(batch);
            assert!(p.is_ok());
            let r: Result<Option<Vec<u8>>, Error> = db.get(b"k1");
            assert_eq!(r.unwrap().unwrap(), b"v1111");
        }
        {
            // test delete
            let mut batch = WriteBatch::default();
            batch.delete(b"k1");
            assert_eq!(batch.len(), 1);
            assert!(!batch.is_empty());
            let p = db.write(batch);
            assert!(p.is_ok());
            assert!(db.get(b"k1").unwrap().is_none());
        }
        {
            // test delete_range
            let mut batch = WriteBatch::default();
            batch.delete_range(b"k2", b"k4");
            assert_eq!(batch.len(), 1);
            assert!(!batch.is_empty());
            let p = db.write(batch);
            assert!(p.is_ok());
            assert!(db.get(b"k2").unwrap().is_none());
            assert!(db.get(b"k3").unwrap().is_none());
        }
        {
            // test size_in_bytes
            let mut batch = WriteBatch::default();
            let before = batch.size_in_bytes();
            batch.put(b"k1", b"v1234567890");
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
fn iterator_test_past_end() {
    let path = "_rust_rocksdb_iteratortest_past_end";
    {
        let db = DB::open_default(path).unwrap();
        db.put(b"k1", b"v1111").unwrap();
        let mut iter = db.iterator(IteratorMode::Start);
        assert!(iter.next().is_some());
        assert!(iter.next().is_none());
        assert!(iter.next().is_none());
    }
    let opts = Options::default();
    DB::destroy(&opts, path).unwrap();
}

#[test]
fn iterator_test_upper_bound() {
    let path = "_rust_rocksdb_iteratortest_upper_bound";
    {
        let db = DB::open_default(path).unwrap();
        db.put(b"k1", b"v1").unwrap();
        db.put(b"k2", b"v2").unwrap();
        db.put(b"k3", b"v3").unwrap();
        db.put(b"k4", b"v4").unwrap();
        db.put(b"k5", b"v5").unwrap();

        let mut readopts = ReadOptions::default();
        readopts.set_iterate_upper_bound(b"k4".to_vec());

        let iter = db.iterator_opt(IteratorMode::Start, readopts);
        let expected: Vec<_> = vec![(b"k1", b"v1"), (b"k2", b"v2"), (b"k3", b"v3")]
            .into_iter()
            .map(|(k, v)| (k.to_vec().into_boxed_slice(), v.to_vec().into_boxed_slice()))
            .collect();
        assert_eq!(expected, iter.collect::<Vec<_>>());
    }
    let opts = Options::default();
    DB::destroy(&opts, path).unwrap();
}

#[test]
fn iterator_test_tailing() {
    let path = "_rust_rocksdb_iteratortest_tailing";
    {
        let data = [(b"k1", b"v1"), (b"k2", b"v2"), (b"k3", b"v3")];
        let mut ro = ReadOptions::default();
        ro.set_tailing(true);
        let db = DB::open_default(path).unwrap();

        let mut data_iter = data.iter();
        let (k, v) = data_iter.next().unwrap();
        let r = db.put(k, v);
        assert!(r.is_ok());

        let tail_iter = db.iterator_opt(IteratorMode::Start, ro);
        for (k, v) in data_iter {
            let r = db.put(k, v);
            assert!(r.is_ok());
        }

        let mut tot = 0;
        for (i, (k, v)) in tail_iter.enumerate() {
            assert_eq!(
                (k.to_vec(), v.to_vec()),
                (data[i].0.to_vec(), data[i].1.to_vec())
            );
            tot += 1;
        }
        assert_eq!(tot, data.len());
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
        let r: Result<Option<Vec<u8>>, Error> = snap.get(b"k1");
        assert_eq!(r.unwrap().unwrap(), b"v1111");

        let p = db.put(b"k2", b"v2222");
        assert!(p.is_ok());

        assert!(db.get(b"k2").unwrap().is_some());
        assert!(snap.get(b"k2").unwrap().is_none());
    }
    let opts = Options::default();
    assert!(DB::destroy(&opts, path).is_ok());
}

#[test]
fn set_option_test() {
    let path = "_rust_rocksdb_set_optionstest";
    {
        let db = DB::open_default(path).unwrap();
        // set an option to valid values
        assert!(db
            .set_options(&[("disable_auto_compactions", "true")])
            .is_ok());
        assert!(db
            .set_options(&[("disable_auto_compactions", "false")])
            .is_ok());
        // invalid names/values should result in an error
        assert!(db
            .set_options(&[("disable_auto_compactions", "INVALID_VALUE")])
            .is_err());
        assert!(db
            .set_options(&[("INVALID_NAME", "INVALID_VALUE")])
            .is_err());
        // option names/values must not contain NULLs
        assert!(db
            .set_options(&[("disable_auto_compactions", "true\0")])
            .is_err());
        assert!(db
            .set_options(&[("disable_auto_compactions\0", "true")])
            .is_err());
        // empty options are not allowed
        assert!(db.set_options(&[]).is_err());
        // multiple options can be set in a single API call
        let multiple_options = [
            ("paranoid_file_checks", "true"),
            ("report_bg_io_stats", "true"),
        ];
        db.set_options(&multiple_options).unwrap();
    }
    assert!(DB::destroy(&Options::default(), path).is_ok());
}

#[test]
fn delete_range_test() {
    let path = "_rust_rocksdb_delete_range_test";
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cfs = vec!["cf1"];
        let db = DB::open_cf(&opts, path, cfs).unwrap();

        let cf1 = db.cf_handle("cf1").unwrap();
        db.put_cf(cf1, b"k1", b"v1").unwrap();
        db.put_cf(cf1, b"k2", b"v2").unwrap();
        db.put_cf(cf1, b"k3", b"v3").unwrap();
        db.put_cf(cf1, b"k4", b"v4").unwrap();
        db.put_cf(cf1, b"k5", b"v5").unwrap();

        db.delete_range_cf(cf1, b"k2", b"k4").unwrap();
        assert_eq!(db.get_cf(cf1, b"k1").unwrap().unwrap(), b"v1");
        assert_eq!(db.get_cf(cf1, b"k4").unwrap().unwrap(), b"v4");
        assert_eq!(db.get_cf(cf1, b"k5").unwrap().unwrap(), b"v5");
        assert!(db.get_cf(cf1, b"k2").unwrap().is_none());
        assert!(db.get_cf(cf1, b"k3").unwrap().is_none());
    }
    let opts = Options::default();
    DB::destroy(&opts, path).unwrap();
}
