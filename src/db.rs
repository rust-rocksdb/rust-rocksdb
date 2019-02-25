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

use ffi;
use ffi_util::opt_bytes_to_ptr;
use util::to_cpath;

use crate::{
    handle::Handle, ColumnFamily, ColumnFamilyDescriptor, DBIterator, DBRawIterator, Direction, Error,
    IteratorMode, Options, ReadOptions, WriteOptions, Snapshot, ops,
};

use libc::{self, c_char, c_int, c_void, size_t};
use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::ptr;
use std::slice;
use std::str;
use std::sync::{Arc, RwLock};

/// A RocksDB database.
///
/// See crate level documentation for a simple usage example.
pub struct DB {
    pub(crate) inner: *mut ffi::rocksdb_t,
    cfs: Arc<RwLock<BTreeMap<String, *mut ffi::rocksdb_column_family_handle_t>>>,
    path: PathBuf,
}

impl Handle<ffi::rocksdb_t> for DB {
  fn handle(&self) -> *mut ffi::rocksdb_t {
    self.inner
  }
}

impl ops::Read for DB {}
impl ops::Write for DB {}

unsafe impl Send for DB {}
unsafe impl Sync for DB {}

/// An atomic batch of write operations.
///
/// Making an atomic commit of several writes:
///
/// ```
/// use rocksdb::{DB, Options, WriteBatch};
/// # use rocksdb::TemporaryDBPath;
///
/// let path = "_path_for_rocksdb_storage1";
/// # let path = TemporaryDBPath::new();
/// # {
///
/// let db = DB::open_default(&path).unwrap();
///
/// let mut batch = WriteBatch::default();
/// batch.put(b"my key", b"my value");
/// batch.put(b"key2", b"value2");
/// batch.put(b"key3", b"value3");
///
/// db.write(batch); // Atomically commits the batch

/// # }
/// ```
pub struct WriteBatch {
    pub(crate) inner: *mut ffi::rocksdb_writebatch_t,
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
        DB::open_cf(opts, path, None::<&str>)
    }

    /// Open a database with the given database options and column family names.
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

        DB::open_cf_descriptors(opts, path, cfs)
    }

    /// Open a database with the given database options and column family descriptors.
    pub fn open_cf_descriptors<P, I>(opts: &Options, path: P, cfs: I) -> Result<DB, Error>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = ColumnFamilyDescriptor>,
    {
        let cfs: Vec<_> = cfs.into_iter().collect();

        let path = path.as_ref();
        let cpath = match CString::new(path.to_string_lossy().as_bytes()) {
            Ok(c) => c,
            Err(_) => {
                return Err(Error::new(
                    "Failed to convert path to CString \
                     when opening DB."
                        .to_owned(),
                ));
            }
        };

        let db: *mut ffi::rocksdb_t;
        let cf_map = Arc::new(RwLock::new(BTreeMap::new()));

        if cfs.is_empty() {
            unsafe {
                db = ffi_try!(ffi::rocksdb_open(opts.inner, cpath.as_ptr() as *const _,));
            }
        } else {
            let mut cfs_v = cfs;
            // Always open the default column family.
            if !cfs_v.iter().any(|cf| cf.name == "default") {
                cfs_v.push(ColumnFamilyDescriptor {
                    name: String::from("default"),
                    options: Options::default(),
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

            let mut cfopts: Vec<_> = cfs_v
                .iter()
                .map(|cf| cf.options.inner as *const _)
                .collect();

            unsafe {
                db = ffi_try!(ffi::rocksdb_open_column_families(
                    opts.inner,
                    cpath.as_ptr(),
                    cfs_v.len() as c_int,
                    cfnames.as_mut_ptr(),
                    cfopts.as_mut_ptr(),
                    cfhandles.as_mut_ptr(),
                ));
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
                cf_map
                    .write()
                    .map_err(|e| Error::new(e.to_string()))?
                    .insert(n.name.clone(), h);
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
            ffi_try!(ffi::rocksdb_destroy_db(opts.inner, cpath.as_ptr(),));
        }
        Ok(())
    }

    pub fn repair<P: AsRef<Path>>(opts: Options, path: P) -> Result<(), Error> {
        let cpath = to_cpath(path)?;
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

    /// Return the value associated with a key using RocksDB's PinnableSlice
    /// so as to avoid unnecessary memory copy.
    pub fn get_pinned_opt<K: AsRef<[u8]>>(
        &self,
        key: K,
        readopts: &ReadOptions,
    ) -> Result<Option<DBPinnableSlice>, Error> {
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
        cf: ColumnFamily,
        key: K,
        readopts: &ReadOptions,
    ) -> Result<Option<DBPinnableSlice>, Error> {
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
        cf: ColumnFamily,
        key: K,
    ) -> Result<Option<DBPinnableSlice>, Error> {
        self.get_pinned_cf_opt(cf, key, &ReadOptions::default())
    }

    pub fn create_cf<N: AsRef<str>>(&self, name: N, opts: &Options) -> Result<ColumnFamily, Error> {
        let cname = match CString::new(name.as_ref().as_bytes()) {
            Ok(c) => c,
            Err(_) => {
                return Err(Error::new(
                    "Failed to convert path to CString \
                     when opening rocksdb"
                        .to_owned(),
                ));
            }
        };
        let cf = unsafe {
            let cf_handle = ffi_try!(ffi::rocksdb_create_column_family(
                self.inner,
                opts.inner,
                cname.as_ptr(),
            ));

            self.cfs
                .write()
                .map_err(|e| Error::new(e.to_string()))?
                .insert(name.as_ref().to_string(), cf_handle);

            ColumnFamily::new(self, cf_handle)
        };
        Ok(cf)
    }

    pub fn drop_cf(&self, name: &str) -> Result<(), Error> {
        if let Some(cf) = self
            .cfs
            .write()
            .map_err(|e| Error::new(e.to_string()))?
            .remove(name)
        {
            unsafe {
                ffi_try!(ffi::rocksdb_drop_column_family(self.inner, cf,));
            }
            Ok(())
        } else {
            Err(Error::new(
                format!("Invalid column family: {}", name).to_owned(),
            ))
        }
    }

    /// Return the underlying column family handle.
    pub fn cf_handle(&self, name: &str) -> Option<ColumnFamily> {
        self.cfs
            .read()
            .ok()?
            .get(name)
            .map(|h| ColumnFamily::new(self, *h))
    }

    pub fn iterator(&self, mode: IteratorMode) -> DBIterator {
        let readopts = ReadOptions::default();
        self.iterator_opt(mode, &readopts)
    }

    pub fn iterator_opt(&self, mode: IteratorMode, readopts: &ReadOptions) -> DBIterator {
        DBIterator::new(self, &readopts, mode)
    }

    /// Opens an interator with `set_total_order_seek` enabled.
    /// This must be used to iterate across prefixes when `set_memtable_factory` has been called
    /// with a Hash-based implementation.
    pub fn full_iterator(&self, mode: IteratorMode) -> DBIterator {
        let mut opts = ReadOptions::default();
        opts.set_total_order_seek(true);
        DBIterator::new(self, &opts, mode)
    }

    pub fn prefix_iterator<P: AsRef<[u8]>>(&self, prefix: P) -> DBIterator {
        let mut opts = ReadOptions::default();
        opts.set_prefix_same_as_start(true);
        DBIterator::new(
            self,
            &opts,
            IteratorMode::From(prefix.as_ref(), Direction::Forward),
        )
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

    pub fn prefix_iterator_cf<P: AsRef<[u8]>>(
        &self,
        cf_handle: ColumnFamily,
        prefix: P,
    ) -> Result<DBIterator, Error> {
        let mut opts = ReadOptions::default();
        opts.set_prefix_same_as_start(true);
        DBIterator::new_cf(
            self,
            cf_handle,
            &opts,
            IteratorMode::From(prefix.as_ref(), Direction::Forward),
        )
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
        cf: ColumnFamily,
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

    pub fn merge<K, V>(&self, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.merge_opt(key.as_ref(), value.as_ref(), &WriteOptions::default())
    }

    pub fn merge_cf<K, V>(&self, cf: ColumnFamily, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.merge_cf_opt(cf, key.as_ref(), value.as_ref(), &WriteOptions::default())
    }

    pub fn compact_range<S: AsRef<[u8]>, E: AsRef<[u8]>>(&self, start: Option<S>, end: Option<E>) {
        unsafe {
            let start = start.as_ref().map(|s| s.as_ref());
            let end = end.as_ref().map(|e| e.as_ref());

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
    /// For a full list of properties, see
    /// https://github.com/facebook/rocksdb/blob/08809f5e6cd9cc4bc3958dd4d59457ae78c76660/include/rocksdb/db.h#L428-L634
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
    /// For a full list of properties, see
    /// https://github.com/facebook/rocksdb/blob/08809f5e6cd9cc4bc3958dd4d59457ae78c76660/include/rocksdb/db.h#L428-L634
    pub fn property_value_cf(&self, cf: ColumnFamily, name: &str) -> Result<Option<String>, Error> {
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
    /// For a full list of properties that return int values, see
    /// https://github.com/facebook/rocksdb/blob/08809f5e6cd9cc4bc3958dd4d59457ae78c76660/include/rocksdb/db.h#L654-L689
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
    /// For a full list of properties that return int values, see
    /// https://github.com/facebook/rocksdb/blob/08809f5e6cd9cc4bc3958dd4d59457ae78c76660/include/rocksdb/db.h#L654-L689
    pub fn property_int_value_cf(
        &self,
        cf: ColumnFamily,
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
    pub fn put<K, V>(&mut self, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let key = key.as_ref();
        let value = value.as_ref();

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

    pub fn put_cf<K, V>(&mut self, cf: ColumnFamily, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let key = key.as_ref();
        let value = value.as_ref();

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

    pub fn merge<K, V>(&mut self, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let key = key.as_ref();
        let value = value.as_ref();

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

    pub fn merge_cf<K, V>(&mut self, cf: ColumnFamily, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let key = key.as_ref();
        let value = value.as_ref();

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
    pub fn delete<K: AsRef<[u8]>>(&mut self, key: K) -> Result<(), Error> {
        let key = key.as_ref();

        unsafe {
            ffi::rocksdb_writebatch_delete(
                self.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
            );
            Ok(())
        }
    }

    pub fn delete_cf<K: AsRef<[u8]>>(&mut self, cf: ColumnFamily, key: K) -> Result<(), Error> {
        let key = key.as_ref();

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

    /// Clear all updates buffered in this batch.
    pub fn clear(&mut self) -> Result<(), Error> {
        unsafe {
            ffi::rocksdb_writebatch_clear(self.inner);
        }
        Ok(())
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
            if let Ok(cfs) = self.cfs.read() {
                for cf in cfs.values() {
                    ffi::rocksdb_column_family_handle_destroy(*cf);
                }
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

/// Wrapper around RocksDB PinnableSlice struct.
///
/// With a pinnable slice, we can directly leverage in-memory data within
/// RocksDB toa void unnecessary memory copies. The struct here wraps the
/// returned raw pointer and ensures proper finalization work.
pub struct DBPinnableSlice<'a> {
    ptr: *mut ffi::rocksdb_pinnableslice_t,
    db: PhantomData<&'a DB>,
}

impl<'a> AsRef<[u8]> for DBPinnableSlice<'a> {
    fn as_ref(&self) -> &[u8] {
        // Implement this via Deref so as not to repeat ourselves
        &*self
    }
}

impl<'a> Deref for DBPinnableSlice<'a> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        unsafe {
            let mut val_len: size_t = 0;
            let val = ffi::rocksdb_pinnableslice_value(self.ptr, &mut val_len) as *mut u8;
            slice::from_raw_parts(val, val_len)
        }
    }
}

impl<'a> Drop for DBPinnableSlice<'a> {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_pinnableslice_destroy(self.ptr);
        }
    }
}

impl<'a> DBPinnableSlice<'a> {
    /// Used to wrap a PinnableSlice from rocksdb to avoid unnecessary memcpy
    ///
    /// # Unsafe
    /// Requires that the pointer must be generated by rocksdb_get_pinned
    pub unsafe fn from_c(ptr: *mut ffi::rocksdb_pinnableslice_t) -> DBPinnableSlice<'a> {
        DBPinnableSlice {
            ptr,
            db: PhantomData,
        }
    }
}

#[test]
fn test_db_vector() {
    use std::mem;
    use crate::prelude::*;

    let len: size_t = 4;
    let data = unsafe { libc::calloc(len, mem::size_of::<u8>()) as *mut u8 };
    let v = unsafe { DBVector::from_c(data, len) };
    let ctrl = [0u8, 0, 0, 0];
    assert_eq!(&*v, &ctrl[..]);
}

#[test]
fn external() {
    use crate::{prelude::*, TemporaryDBPath};

    let path = TemporaryDBPath::new();
    {
        let db = DB::open_default(&path).unwrap();
        let p = db.put(b"k1", b"v1111");
        assert!(p.is_ok());
        let r: Result<Option<DBVector>, Error> = db.get(b"k1");
        assert!(r.unwrap().unwrap().to_utf8().unwrap() == "v1111");
        assert!(db.delete(b"k1").is_ok());
        assert!(db.get(b"k1").unwrap().is_none());
    }
}

#[test]
fn errors_do_stuff() {
    use crate::{prelude::*, TemporaryDBPath};

    let path = TemporaryDBPath::new();
    {
        let _db = DB::open_default(&path).unwrap();
        let opts = Options::default();
        // The DB will still be open when we try to destroy it and the lock should fail.
        match DB::destroy(&opts, &path) {
            Err(s) => {
                let message = s.to_string();
                assert!(message.find("IO error:").is_some());
                assert!(message.find("/LOCK:").is_some());
            }
            Ok(_) => panic!("should fail"),
        }
    }
}

#[test]
fn writebatch_works() {
    use crate::{prelude::*, TemporaryDBPath};

    let path = TemporaryDBPath::new();
    {
        let db = DB::open_default(&path).unwrap();
        {
            // test putx
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
}

#[test]
fn iterator_test() {
    use crate::{prelude::*, TemporaryDBPath};

    let path = TemporaryDBPath::new();
    {
        let db = DB::open_default(&path).unwrap();
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
}

#[test]
fn snapshot_test() {
    use crate::{prelude::*, TemporaryDBPath};

    let path = TemporaryDBPath::new();
    {
        let db = DB::open_default(&path).unwrap();
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
}

#[test]
fn set_option_test() {
    use crate::{prelude::*, TemporaryDBPath};

    let path = TemporaryDBPath::new();
    {
        let db = DB::open_default(&path).unwrap();
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
}
