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
    column_family::AsColumnFamilyRef,
    column_family::BoundColumnFamily,
    column_family::UnboundColumnFamily,
    db_options::OptionsMustOutliveDB,
    ffi,
    ffi_util::{from_cstr, opt_bytes_to_ptr, raw_data, to_cpath, CStrLike},
    ColumnFamily, ColumnFamilyDescriptor, CompactOptions, DBIteratorWithThreadMode,
    DBPinnableSlice, DBRawIteratorWithThreadMode, DBWALIterator, Direction, Error, FlushOptions,
    IngestExternalFileOptions, IteratorMode, Options, ReadOptions, SnapshotWithThreadMode,
    WriteBatch, WriteOptions, DEFAULT_COLUMN_FAMILY_NAME,
};

use crate::ffi_util::CSlice;
use libc::{self, c_char, c_int, c_uchar, c_void, size_t};
use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::fmt;
use std::fs;
use std::iter;
use std::path::Path;
use std::path::PathBuf;
use std::ptr;
use std::slice;
use std::str;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;

/// Marker trait to specify single or multi threaded column family alternations for
/// [`DBWithThreadMode<T>`]
///
/// This arrangement makes differences in self mutability and return type in
/// some of `DBWithThreadMode` methods.
///
/// While being a marker trait to be generic over `DBWithThreadMode`, this trait
/// also has a minimum set of not-encapsulated internal methods between
/// [`SingleThreaded`] and [`MultiThreaded`].  These methods aren't expected to be
/// called and defined externally.
pub trait ThreadMode {
    /// Internal implementation for storing column family handles
    fn new_cf_map_internal(
        cf_map: BTreeMap<String, *mut ffi::rocksdb_column_family_handle_t>,
    ) -> Self;
    /// Internal implementation for dropping column family handles
    fn drop_all_cfs_internal(&mut self);
}

/// Actual marker type for the marker trait `ThreadMode`, which holds
/// a collection of column families without synchronization primitive, providing
/// no overhead for the single-threaded column family alternations. The other
/// mode is [`MultiThreaded`].
///
/// See [`DB`] for more details, including performance implications for each mode
pub struct SingleThreaded {
    pub(crate) cfs: BTreeMap<String, ColumnFamily>,
}

/// Actual marker type for the marker trait `ThreadMode`, which holds
/// a collection of column families wrapped in a RwLock to be mutated
/// concurrently. The other mode is [`SingleThreaded`].
///
/// See [`DB`] for more details, including performance implications for each mode
pub struct MultiThreaded {
    pub(crate) cfs: RwLock<BTreeMap<String, Arc<UnboundColumnFamily>>>,
}

impl ThreadMode for SingleThreaded {
    fn new_cf_map_internal(
        cfs: BTreeMap<String, *mut ffi::rocksdb_column_family_handle_t>,
    ) -> Self {
        Self {
            cfs: cfs
                .into_iter()
                .map(|(n, c)| (n, ColumnFamily { inner: c }))
                .collect(),
        }
    }

    fn drop_all_cfs_internal(&mut self) {
        // Cause all ColumnFamily objects to be Drop::drop()-ed.
        self.cfs.clear();
    }
}

impl ThreadMode for MultiThreaded {
    fn new_cf_map_internal(
        cfs: BTreeMap<String, *mut ffi::rocksdb_column_family_handle_t>,
    ) -> Self {
        Self {
            cfs: RwLock::new(
                cfs.into_iter()
                    .map(|(n, c)| (n, Arc::new(UnboundColumnFamily { inner: c })))
                    .collect(),
            ),
        }
    }

    fn drop_all_cfs_internal(&mut self) {
        // Cause all UnboundColumnFamily objects to be Drop::drop()-ed.
        self.cfs.write().unwrap().clear();
    }
}

/// Get underlying `rocksdb_t`.
pub trait DBInner {
    fn inner(&self) -> *mut ffi::rocksdb_t;
}

/// A helper type to implement some common methods for [`DBWithThreadMode`]
/// and [`OptimisticTransactionDB`].
///
/// [`OptimisticTransactionDB`]: crate::OptimisticTransactionDB
pub struct DBCommon<T: ThreadMode, D: DBInner> {
    pub(crate) inner: D,
    cfs: T, // Column families are held differently depending on thread mode
    path: PathBuf,
    _outlive: Vec<OptionsMustOutliveDB>,
}

/// Minimal set of DB-related methods, intended to be generic over
/// `DBWithThreadMode<T>`. Mainly used internally
pub trait DBAccess {
    unsafe fn create_snapshot(&self) -> *const ffi::rocksdb_snapshot_t;

    unsafe fn release_snapshot(&self, snapshot: *const ffi::rocksdb_snapshot_t);

    unsafe fn create_iterator(&self, readopts: &ReadOptions) -> *mut ffi::rocksdb_iterator_t;

    unsafe fn create_iterator_cf(
        &self,
        cf_handle: *mut ffi::rocksdb_column_family_handle_t,
        readopts: &ReadOptions,
    ) -> *mut ffi::rocksdb_iterator_t;

    fn get_opt<K: AsRef<[u8]>>(
        &self,
        key: K,
        readopts: &ReadOptions,
    ) -> Result<Option<Vec<u8>>, Error>;

    fn get_cf_opt<K: AsRef<[u8]>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        key: K,
        readopts: &ReadOptions,
    ) -> Result<Option<Vec<u8>>, Error>;

    fn get_pinned_opt<K: AsRef<[u8]>>(
        &self,
        key: K,
        readopts: &ReadOptions,
    ) -> Result<Option<DBPinnableSlice>, Error>;

    fn get_pinned_cf_opt<K: AsRef<[u8]>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        key: K,
        readopts: &ReadOptions,
    ) -> Result<Option<DBPinnableSlice>, Error>;

    fn multi_get_opt<K, I>(
        &self,
        keys: I,
        readopts: &ReadOptions,
    ) -> Vec<Result<Option<Vec<u8>>, Error>>
    where
        K: AsRef<[u8]>,
        I: IntoIterator<Item = K>;

    fn multi_get_cf_opt<'b, K, I, W>(
        &self,
        keys_cf: I,
        readopts: &ReadOptions,
    ) -> Vec<Result<Option<Vec<u8>>, Error>>
    where
        K: AsRef<[u8]>,
        I: IntoIterator<Item = (&'b W, K)>,
        W: AsColumnFamilyRef + 'b;
}

impl<T: ThreadMode, D: DBInner> DBAccess for DBCommon<T, D> {
    unsafe fn create_snapshot(&self) -> *const ffi::rocksdb_snapshot_t {
        ffi::rocksdb_create_snapshot(self.inner.inner())
    }

    unsafe fn release_snapshot(&self, snapshot: *const ffi::rocksdb_snapshot_t) {
        ffi::rocksdb_release_snapshot(self.inner.inner(), snapshot);
    }

    unsafe fn create_iterator(&self, readopts: &ReadOptions) -> *mut ffi::rocksdb_iterator_t {
        ffi::rocksdb_create_iterator(self.inner.inner(), readopts.inner)
    }

    unsafe fn create_iterator_cf(
        &self,
        cf_handle: *mut ffi::rocksdb_column_family_handle_t,
        readopts: &ReadOptions,
    ) -> *mut ffi::rocksdb_iterator_t {
        ffi::rocksdb_create_iterator_cf(self.inner.inner(), readopts.inner, cf_handle)
    }

    fn get_opt<K: AsRef<[u8]>>(
        &self,
        key: K,
        readopts: &ReadOptions,
    ) -> Result<Option<Vec<u8>>, Error> {
        self.get_opt(key, readopts)
    }

    fn get_cf_opt<K: AsRef<[u8]>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        key: K,
        readopts: &ReadOptions,
    ) -> Result<Option<Vec<u8>>, Error> {
        self.get_cf_opt(cf, key, readopts)
    }

    fn get_pinned_opt<K: AsRef<[u8]>>(
        &self,
        key: K,
        readopts: &ReadOptions,
    ) -> Result<Option<DBPinnableSlice>, Error> {
        self.get_pinned_opt(key, readopts)
    }

    fn get_pinned_cf_opt<K: AsRef<[u8]>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        key: K,
        readopts: &ReadOptions,
    ) -> Result<Option<DBPinnableSlice>, Error> {
        self.get_pinned_cf_opt(cf, key, readopts)
    }

    fn multi_get_opt<K, Iter>(
        &self,
        keys: Iter,
        readopts: &ReadOptions,
    ) -> Vec<Result<Option<Vec<u8>>, Error>>
    where
        K: AsRef<[u8]>,
        Iter: IntoIterator<Item = K>,
    {
        self.multi_get_opt(keys, readopts)
    }

    fn multi_get_cf_opt<'b, K, Iter, W>(
        &self,
        keys_cf: Iter,
        readopts: &ReadOptions,
    ) -> Vec<Result<Option<Vec<u8>>, Error>>
    where
        K: AsRef<[u8]>,
        Iter: IntoIterator<Item = (&'b W, K)>,
        W: AsColumnFamilyRef + 'b,
    {
        self.multi_get_cf_opt(keys_cf, readopts)
    }
}

pub struct DBWithThreadModeInner {
    inner: *mut ffi::rocksdb_t,
}

impl DBInner for DBWithThreadModeInner {
    fn inner(&self) -> *mut ffi::rocksdb_t {
        self.inner
    }
}

impl Drop for DBWithThreadModeInner {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_close(self.inner);
        }
    }
}

/// A type alias to RocksDB database.
///
/// See crate level documentation for a simple usage example.
/// See [`DBCommon`] for full list of methods.
pub type DBWithThreadMode<T> = DBCommon<T, DBWithThreadModeInner>;

/// A type alias to DB instance type with the single-threaded column family
/// creations/deletions
///
/// # Compatibility and multi-threaded mode
///
/// Previously, [`DB`] was defined as a direct `struct`. Now, it's type-aliased for
/// compatibility. Use `DBCommon<MultiThreaded>` for multi-threaded
/// column family alternations.
///
/// # Limited performance implication for single-threaded mode
///
/// Even with [`SingleThreaded`], almost all of RocksDB operations is
/// multi-threaded unless the underlying RocksDB instance is
/// specifically configured otherwise. `SingleThreaded` only forces
/// serialization of column family alternations by requiring `&mut self` of DB
/// instance due to its wrapper implementation details.
///
/// # Multi-threaded mode
///
/// [`MultiThreaded`] can be appropriate for the situation of multi-threaded
/// workload including multi-threaded column family alternations, costing the
/// RwLock overhead inside `DB`.
#[cfg(not(feature = "multi-threaded-cf"))]
pub type DB = DBWithThreadMode<SingleThreaded>;

#[cfg(feature = "multi-threaded-cf")]
pub type DB = DBWithThreadMode<MultiThreaded>;

// Safety note: auto-implementing Send on most db-related types is prevented by the inner FFI
// pointer. In most cases, however, this pointer is Send-safe because it is never aliased and
// rocksdb internally does not rely on thread-local information for its user-exposed types.
unsafe impl<T: ThreadMode + Send, I: DBInner> Send for DBCommon<T, I> {}

// Sync is similarly safe for many types because they do not expose interior mutability, and their
// use within the rocksdb library is generally behind a const reference
unsafe impl<T: ThreadMode, I: DBInner> Sync for DBCommon<T, I> {}

// Specifies whether open DB for read only.
enum AccessType<'a> {
    ReadWrite,
    ReadOnly { error_if_log_file_exist: bool },
    Secondary { secondary_path: &'a Path },
    WithTTL { ttl: Duration },
}

/// Methods of `DBWithThreadMode`.
impl<T: ThreadMode> DBWithThreadMode<T> {
    /// Opens a database with default options.
    pub fn open_default<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        Self::open(&opts, path)
    }

    /// Opens the database with the specified options.
    pub fn open<P: AsRef<Path>>(opts: &Options, path: P) -> Result<Self, Error> {
        Self::open_cf(opts, path, None::<&str>)
    }

    /// Opens the database for read only with the specified options.
    pub fn open_for_read_only<P: AsRef<Path>>(
        opts: &Options,
        path: P,
        error_if_log_file_exist: bool,
    ) -> Result<Self, Error> {
        Self::open_cf_for_read_only(opts, path, None::<&str>, error_if_log_file_exist)
    }

    /// Opens the database as a secondary.
    pub fn open_as_secondary<P: AsRef<Path>>(
        opts: &Options,
        primary_path: P,
        secondary_path: P,
    ) -> Result<Self, Error> {
        Self::open_cf_as_secondary(opts, primary_path, secondary_path, None::<&str>)
    }

    /// Opens the database with a Time to Live compaction filter.
    pub fn open_with_ttl<P: AsRef<Path>>(
        opts: &Options,
        path: P,
        ttl: Duration,
    ) -> Result<Self, Error> {
        Self::open_cf_descriptors_with_ttl(opts, path, std::iter::empty(), ttl)
    }

    /// Opens the database with a Time to Live compaction filter and column family names.
    ///
    /// Column families opened using this function will be created with default `Options`.
    pub fn open_cf_with_ttl<P, I, N>(
        opts: &Options,
        path: P,
        cfs: I,
        ttl: Duration,
    ) -> Result<Self, Error>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = N>,
        N: AsRef<str>,
    {
        let cfs = cfs
            .into_iter()
            .map(|name| ColumnFamilyDescriptor::new(name.as_ref(), Options::default()));

        Self::open_cf_descriptors_with_ttl(opts, path, cfs, ttl)
    }

    /// Opens a database with the given database with a Time to Live compaction filter and
    /// column family descriptors.
    pub fn open_cf_descriptors_with_ttl<P, I>(
        opts: &Options,
        path: P,
        cfs: I,
        ttl: Duration,
    ) -> Result<Self, Error>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = ColumnFamilyDescriptor>,
    {
        Self::open_cf_descriptors_internal(opts, path, cfs, &AccessType::WithTTL { ttl })
    }

    /// Opens a database with the given database options and column family names.
    ///
    /// Column families opened using this function will be created with default `Options`.
    pub fn open_cf<P, I, N>(opts: &Options, path: P, cfs: I) -> Result<Self, Error>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = N>,
        N: AsRef<str>,
    {
        let cfs = cfs
            .into_iter()
            .map(|name| ColumnFamilyDescriptor::new(name.as_ref(), Options::default()));

        Self::open_cf_descriptors_internal(opts, path, cfs, &AccessType::ReadWrite)
    }

    /// Opens a database with the given database options and column family names.
    ///
    /// Column families opened using given `Options`.
    pub fn open_cf_with_opts<P, I, N>(opts: &Options, path: P, cfs: I) -> Result<Self, Error>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = (N, Options)>,
        N: AsRef<str>,
    {
        let cfs = cfs
            .into_iter()
            .map(|(name, opts)| ColumnFamilyDescriptor::new(name.as_ref(), opts));

        Self::open_cf_descriptors(opts, path, cfs)
    }

    /// Opens a database for read only with the given database options and column family names.
    pub fn open_cf_for_read_only<P, I, N>(
        opts: &Options,
        path: P,
        cfs: I,
        error_if_log_file_exist: bool,
    ) -> Result<Self, Error>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = N>,
        N: AsRef<str>,
    {
        let cfs = cfs
            .into_iter()
            .map(|name| ColumnFamilyDescriptor::new(name.as_ref(), Options::default()));

        Self::open_cf_descriptors_internal(
            opts,
            path,
            cfs,
            &AccessType::ReadOnly {
                error_if_log_file_exist,
            },
        )
    }

    /// Opens a database for read only with the given database options and column family names.
    pub fn open_cf_with_opts_for_read_only<P, I, N>(
        db_opts: &Options,
        path: P,
        cfs: I,
        error_if_log_file_exist: bool,
    ) -> Result<Self, Error>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = (N, Options)>,
        N: AsRef<str>,
    {
        let cfs = cfs
            .into_iter()
            .map(|(name, cf_opts)| ColumnFamilyDescriptor::new(name.as_ref(), cf_opts));

        Self::open_cf_descriptors_internal(
            db_opts,
            path,
            cfs,
            &AccessType::ReadOnly {
                error_if_log_file_exist,
            },
        )
    }

    /// Opens a database for ready only with the given database options and
    /// column family descriptors.
    pub fn open_cf_descriptors_read_only<P, I>(
        opts: &Options,
        path: P,
        cfs: I,
        error_if_log_file_exist: bool,
    ) -> Result<Self, Error>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = ColumnFamilyDescriptor>,
    {
        Self::open_cf_descriptors_internal(
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
    ) -> Result<Self, Error>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = N>,
        N: AsRef<str>,
    {
        let cfs = cfs
            .into_iter()
            .map(|name| ColumnFamilyDescriptor::new(name.as_ref(), Options::default()));

        Self::open_cf_descriptors_internal(
            opts,
            primary_path,
            cfs,
            &AccessType::Secondary {
                secondary_path: secondary_path.as_ref(),
            },
        )
    }

    /// Opens the database as a secondary with the given database options and
    /// column family descriptors.
    pub fn open_cf_descriptors_as_secondary<P, I>(
        opts: &Options,
        path: P,
        secondary_path: P,
        cfs: I,
    ) -> Result<Self, Error>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = ColumnFamilyDescriptor>,
    {
        Self::open_cf_descriptors_internal(
            opts,
            path,
            cfs,
            &AccessType::Secondary {
                secondary_path: secondary_path.as_ref(),
            },
        )
    }

    /// Opens a database with the given database options and column family descriptors.
    pub fn open_cf_descriptors<P, I>(opts: &Options, path: P, cfs: I) -> Result<Self, Error>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = ColumnFamilyDescriptor>,
    {
        Self::open_cf_descriptors_internal(opts, path, cfs, &AccessType::ReadWrite)
    }

    /// Internal implementation for opening RocksDB.
    fn open_cf_descriptors_internal<P, I>(
        opts: &Options,
        path: P,
        cfs: I,
        access_type: &AccessType,
    ) -> Result<Self, Error>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = ColumnFamilyDescriptor>,
    {
        let cfs: Vec<_> = cfs.into_iter().collect();
        let outlive = iter::once(opts.outlive.clone())
            .chain(cfs.iter().map(|cf| cf.options.outlive.clone()))
            .collect();

        let cpath = to_cpath(&path)?;

        if let Err(e) = fs::create_dir_all(&path) {
            return Err(Error::new(format!(
                "Failed to create RocksDB directory: `{e:?}`."
            )));
        }

        let db: *mut ffi::rocksdb_t;
        let mut cf_map = BTreeMap::new();

        if cfs.is_empty() {
            db = Self::open_raw(opts, &cpath, access_type)?;
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

            db = Self::open_cf_raw(
                opts,
                &cpath,
                &cfs_v,
                &cfnames,
                &cfopts,
                &mut cfhandles,
                access_type,
            )?;
            for handle in &cfhandles {
                if handle.is_null() {
                    return Err(Error::new(
                        "Received null column family handle from DB.".to_owned(),
                    ));
                }
            }

            for (cf_desc, inner) in cfs_v.iter().zip(cfhandles) {
                cf_map.insert(cf_desc.name.clone(), inner);
            }
        }

        if db.is_null() {
            return Err(Error::new("Could not initialize database.".to_owned()));
        }

        Ok(Self {
            inner: DBWithThreadModeInner { inner: db },
            path: path.as_ref().to_path_buf(),
            cfs: T::new_cf_map_internal(cf_map),
            _outlive: outlive,
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
                    cpath.as_ptr(),
                    c_uchar::from(error_if_log_file_exist),
                )),
                AccessType::ReadWrite => {
                    ffi_try!(ffi::rocksdb_open(opts.inner, cpath.as_ptr()))
                }
                AccessType::Secondary { secondary_path } => {
                    ffi_try!(ffi::rocksdb_open_as_secondary(
                        opts.inner,
                        cpath.as_ptr(),
                        to_cpath(secondary_path)?.as_ptr(),
                    ))
                }
                AccessType::WithTTL { ttl } => ffi_try!(ffi::rocksdb_open_with_ttl(
                    opts.inner,
                    cpath.as_ptr(),
                    ttl.as_secs() as c_int,
                )),
            }
        };
        Ok(db)
    }

    #[allow(clippy::pedantic)]
    fn open_cf_raw(
        opts: &Options,
        cpath: &CString,
        cfs_v: &[ColumnFamilyDescriptor],
        cfnames: &[*const c_char],
        cfopts: &[*const ffi::rocksdb_options_t],
        cfhandles: &mut [*mut ffi::rocksdb_column_family_handle_t],
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
                    c_uchar::from(error_if_log_file_exist),
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
                        cpath.as_ptr(),
                        to_cpath(secondary_path)?.as_ptr(),
                        cfs_v.len() as c_int,
                        cfnames.as_ptr(),
                        cfopts.as_ptr(),
                        cfhandles.as_mut_ptr(),
                    ))
                }
                AccessType::WithTTL { ttl } => {
                    let ttls_v = vec![ttl.as_secs() as c_int; cfs_v.len()];
                    ffi_try!(ffi::rocksdb_open_column_families_with_ttl(
                        opts.inner,
                        cpath.as_ptr(),
                        cfs_v.len() as c_int,
                        cfnames.as_ptr(),
                        cfopts.as_ptr(),
                        cfhandles.as_mut_ptr(),
                        ttls_v.as_ptr(),
                    ))
                }
            }
        };
        Ok(db)
    }

    /// Removes the database entries in the range `["from", "to")` using given write options.
    pub fn delete_range_cf_opt<K: AsRef<[u8]>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        from: K,
        to: K,
        writeopts: &WriteOptions,
    ) -> Result<(), Error> {
        let from = from.as_ref();
        let to = to.as_ref();

        unsafe {
            ffi_try!(ffi::rocksdb_delete_range_cf(
                self.inner.inner(),
                writeopts.inner,
                cf.inner(),
                from.as_ptr() as *const c_char,
                from.len() as size_t,
                to.as_ptr() as *const c_char,
                to.len() as size_t,
            ));
            Ok(())
        }
    }

    /// Removes the database entries in the range `["from", "to")` using default write options.
    pub fn delete_range_cf<K: AsRef<[u8]>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        from: K,
        to: K,
    ) -> Result<(), Error> {
        self.delete_range_cf_opt(cf, from, to, &WriteOptions::default())
    }

    pub fn write_opt(&self, batch: WriteBatch, writeopts: &WriteOptions) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_write(
                self.inner.inner(),
                writeopts.inner,
                batch.inner
            ));
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
}

/// Common methods of `DBWithThreadMode` and `OptimisticTransactionDB`.
impl<T: ThreadMode, D: DBInner> DBCommon<T, D> {
    pub(crate) fn new(inner: D, cfs: T, path: PathBuf, outlive: Vec<OptionsMustOutliveDB>) -> Self {
        Self {
            inner,
            cfs,
            path,
            _outlive: outlive,
        }
    }

    pub fn list_cf<P: AsRef<Path>>(opts: &Options, path: P) -> Result<Vec<String>, Error> {
        let cpath = to_cpath(path)?;
        let mut length = 0;

        unsafe {
            let ptr = ffi_try!(ffi::rocksdb_list_column_families(
                opts.inner,
                cpath.as_ptr(),
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
        self.path.as_path()
    }

    /// Flushes the WAL buffer. If `sync` is set to `true`, also syncs
    /// the data to disk.
    pub fn flush_wal(&self, sync: bool) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_flush_wal(
                self.inner.inner(),
                c_uchar::from(sync)
            ));
        }
        Ok(())
    }

    /// Flushes database memtables to SST files on the disk.
    pub fn flush_opt(&self, flushopts: &FlushOptions) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_flush(self.inner.inner(), flushopts.inner));
        }
        Ok(())
    }

    /// Flushes database memtables to SST files on the disk using default options.
    pub fn flush(&self) -> Result<(), Error> {
        self.flush_opt(&FlushOptions::default())
    }

    /// Flushes database memtables to SST files on the disk for a given column family.
    pub fn flush_cf_opt(
        &self,
        cf: &impl AsColumnFamilyRef,
        flushopts: &FlushOptions,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_flush_cf(
                self.inner.inner(),
                flushopts.inner,
                cf.inner()
            ));
        }
        Ok(())
    }

    /// Flushes multiple column families.
    ///
    /// If atomic flush is not enabled, it is equivalent to calling flush_cf multiple times.
    /// If atomic flush is enabled, it will flush all column families specified in `cfs` up to the latest sequence
    /// number at the time when flush is requested.
    pub fn flush_cfs_opt(
        &self,
        cfs: &[&impl AsColumnFamilyRef],
        opts: &FlushOptions,
    ) -> Result<(), Error> {
        let mut cfs = cfs.iter().map(|cf| cf.inner()).collect::<Vec<_>>();
        unsafe {
            ffi_try!(ffi::rocksdb_flush_cfs(
                self.inner.inner(),
                opts.inner,
                cfs.as_mut_ptr(),
                cfs.len() as libc::c_int,
            ));
        }
        Ok(())
    }

    /// Flushes database memtables to SST files on the disk for a given column family using default
    /// options.
    pub fn flush_cf(&self, cf: &impl AsColumnFamilyRef) -> Result<(), Error> {
        self.flush_cf_opt(cf, &FlushOptions::default())
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
        cf: &impl AsColumnFamilyRef,
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
        cf: &impl AsColumnFamilyRef,
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
                self.inner.inner(),
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
        cf: &impl AsColumnFamilyRef,
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
                self.inner.inner(),
                readopts.inner,
                cf.inner(),
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
        cf: &impl AsColumnFamilyRef,
        key: K,
    ) -> Result<Option<DBPinnableSlice>, Error> {
        self.get_pinned_cf_opt(cf, key, &ReadOptions::default())
    }

    /// Return the values associated with the given keys.
    pub fn multi_get<K, I>(&self, keys: I) -> Vec<Result<Option<Vec<u8>>, Error>>
    where
        K: AsRef<[u8]>,
        I: IntoIterator<Item = K>,
    {
        self.multi_get_opt(keys, &ReadOptions::default())
    }

    /// Return the values associated with the given keys using read options.
    pub fn multi_get_opt<K, I>(
        &self,
        keys: I,
        readopts: &ReadOptions,
    ) -> Vec<Result<Option<Vec<u8>>, Error>>
    where
        K: AsRef<[u8]>,
        I: IntoIterator<Item = K>,
    {
        let (keys, keys_sizes): (Vec<Box<[u8]>>, Vec<_>) = keys
            .into_iter()
            .map(|k| (Box::from(k.as_ref()), k.as_ref().len()))
            .unzip();
        let ptr_keys: Vec<_> = keys.iter().map(|k| k.as_ptr() as *const c_char).collect();

        let mut values = vec![ptr::null_mut(); keys.len()];
        let mut values_sizes = vec![0_usize; keys.len()];
        let mut errors = vec![ptr::null_mut(); keys.len()];
        unsafe {
            ffi::rocksdb_multi_get(
                self.inner.inner(),
                readopts.inner,
                ptr_keys.len(),
                ptr_keys.as_ptr(),
                keys_sizes.as_ptr(),
                values.as_mut_ptr(),
                values_sizes.as_mut_ptr(),
                errors.as_mut_ptr(),
            );
        }

        convert_values(values, values_sizes, errors)
    }

    /// Return the values associated with the given keys and column families.
    pub fn multi_get_cf<'a, 'b: 'a, K, I, W>(
        &'a self,
        keys: I,
    ) -> Vec<Result<Option<Vec<u8>>, Error>>
    where
        K: AsRef<[u8]>,
        I: IntoIterator<Item = (&'b W, K)>,
        W: 'b + AsColumnFamilyRef,
    {
        self.multi_get_cf_opt(keys, &ReadOptions::default())
    }

    /// Return the values associated with the given keys and column families using read options.
    pub fn multi_get_cf_opt<'a, 'b: 'a, K, I, W>(
        &'a self,
        keys: I,
        readopts: &ReadOptions,
    ) -> Vec<Result<Option<Vec<u8>>, Error>>
    where
        K: AsRef<[u8]>,
        I: IntoIterator<Item = (&'b W, K)>,
        W: 'b + AsColumnFamilyRef,
    {
        let (cfs_and_keys, keys_sizes): (Vec<(_, Box<[u8]>)>, Vec<_>) = keys
            .into_iter()
            .map(|(cf, key)| ((cf, Box::from(key.as_ref())), key.as_ref().len()))
            .unzip();
        let ptr_keys: Vec<_> = cfs_and_keys
            .iter()
            .map(|(_, k)| k.as_ptr() as *const c_char)
            .collect();
        let ptr_cfs: Vec<_> = cfs_and_keys
            .iter()
            .map(|(c, _)| c.inner() as *const _)
            .collect();

        let mut values = vec![ptr::null_mut(); ptr_keys.len()];
        let mut values_sizes = vec![0_usize; ptr_keys.len()];
        let mut errors = vec![ptr::null_mut(); ptr_keys.len()];
        unsafe {
            ffi::rocksdb_multi_get_cf(
                self.inner.inner(),
                readopts.inner,
                ptr_cfs.as_ptr(),
                ptr_keys.len(),
                ptr_keys.as_ptr(),
                keys_sizes.as_ptr(),
                values.as_mut_ptr(),
                values_sizes.as_mut_ptr(),
                errors.as_mut_ptr(),
            );
        }

        convert_values(values, values_sizes, errors)
    }

    /// Return the values associated with the given keys and the specified column family
    /// where internally the read requests are processed in batch if block-based table
    /// SST format is used.  It is a more optimized version of multi_get_cf.
    pub fn batched_multi_get_cf<K, I>(
        &self,
        cf: &impl AsColumnFamilyRef,
        keys: I,
        sorted_input: bool,
    ) -> Vec<Result<Option<DBPinnableSlice>, Error>>
    where
        K: AsRef<[u8]>,
        I: IntoIterator<Item = K>,
    {
        self.batched_multi_get_cf_opt(cf, keys, sorted_input, &ReadOptions::default())
    }

    /// Return the values associated with the given keys and the specified column family
    /// where internally the read requests are processed in batch if block-based table
    /// SST format is used.  It is a more optimized version of multi_get_cf_opt.
    pub fn batched_multi_get_cf_opt<K, I>(
        &self,
        cf: &impl AsColumnFamilyRef,
        keys: I,
        sorted_input: bool,
        readopts: &ReadOptions,
    ) -> Vec<Result<Option<DBPinnableSlice>, Error>>
    where
        K: AsRef<[u8]>,
        I: IntoIterator<Item = K>,
    {
        let (keys, keys_sizes): (Vec<Box<[u8]>>, Vec<_>) = keys
            .into_iter()
            .map(|k| (Box::from(k.as_ref()), k.as_ref().len()))
            .unzip();
        let ptr_keys: Vec<_> = keys.iter().map(|k| k.as_ptr() as *const c_char).collect();

        let mut pinned_values = vec![ptr::null_mut(); ptr_keys.len()];
        let mut errors = vec![ptr::null_mut(); ptr_keys.len()];

        unsafe {
            ffi::rocksdb_batched_multi_get_cf(
                self.inner.inner(),
                readopts.inner,
                cf.inner(),
                ptr_keys.len(),
                ptr_keys.as_ptr(),
                keys_sizes.as_ptr(),
                pinned_values.as_mut_ptr(),
                errors.as_mut_ptr(),
                sorted_input,
            );
            pinned_values
                .into_iter()
                .zip(errors.into_iter())
                .map(|(v, e)| {
                    if e.is_null() {
                        if v.is_null() {
                            Ok(None)
                        } else {
                            Ok(Some(DBPinnableSlice::from_c(v)))
                        }
                    } else {
                        Err(Error::new(crate::ffi_util::error_message(e)))
                    }
                })
                .collect()
        }
    }

    /// Returns `false` if the given key definitely doesn't exist in the database, otherwise returns
    /// `true`. This function uses default `ReadOptions`.
    pub fn key_may_exist<K: AsRef<[u8]>>(&self, key: K) -> bool {
        self.key_may_exist_opt(key, &ReadOptions::default())
    }

    /// Returns `false` if the given key definitely doesn't exist in the database, otherwise returns
    /// `true`.
    pub fn key_may_exist_opt<K: AsRef<[u8]>>(&self, key: K, readopts: &ReadOptions) -> bool {
        let key = key.as_ref();
        unsafe {
            0 != ffi::rocksdb_key_may_exist(
                self.inner.inner(),
                readopts.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
                ptr::null_mut(), /*value*/
                ptr::null_mut(), /*val_len*/
                ptr::null(),     /*timestamp*/
                0,               /*timestamp_len*/
                ptr::null_mut(), /*value_found*/
            )
        }
    }

    /// Returns `false` if the given key definitely doesn't exist in the specified column family,
    /// otherwise returns `true`. This function uses default `ReadOptions`.
    pub fn key_may_exist_cf<K: AsRef<[u8]>>(&self, cf: &impl AsColumnFamilyRef, key: K) -> bool {
        self.key_may_exist_cf_opt(cf, key, &ReadOptions::default())
    }

    /// Returns `false` if the given key definitely doesn't exist in the specified column family,
    /// otherwise returns `true`.
    pub fn key_may_exist_cf_opt<K: AsRef<[u8]>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        key: K,
        readopts: &ReadOptions,
    ) -> bool {
        let key = key.as_ref();
        0 != unsafe {
            ffi::rocksdb_key_may_exist_cf(
                self.inner.inner(),
                readopts.inner,
                cf.inner(),
                key.as_ptr() as *const c_char,
                key.len() as size_t,
                ptr::null_mut(), /*value*/
                ptr::null_mut(), /*val_len*/
                ptr::null(),     /*timestamp*/
                0,               /*timestamp_len*/
                ptr::null_mut(), /*value_found*/
            )
        }
    }

    /// If the key definitely does not exist in the database, then this method
    /// returns `(false, None)`, else `(true, None)` if it may.
    /// If the key is found in memory, then it returns `(true, Some<CSlice>)`.
    ///
    /// This check is potentially lighter-weight than calling `get()`. One way
    /// to make this lighter weight is to avoid doing any IOs.
    pub fn key_may_exist_cf_opt_value<K: AsRef<[u8]>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        key: K,
        readopts: &ReadOptions,
    ) -> (bool, Option<CSlice>) {
        let key = key.as_ref();
        let mut val: *mut c_char = ptr::null_mut();
        let mut val_len: usize = 0;
        let mut value_found: c_uchar = 0;
        let may_exists = 0
            != unsafe {
                ffi::rocksdb_key_may_exist_cf(
                    self.inner.inner(),
                    readopts.inner,
                    cf.inner(),
                    key.as_ptr() as *const c_char,
                    key.len() as size_t,
                    &mut val,         /*value*/
                    &mut val_len,     /*val_len*/
                    ptr::null(),      /*timestamp*/
                    0,                /*timestamp_len*/
                    &mut value_found, /*value_found*/
                )
            };
        // The value is only allocated (using malloc) and returned if it is found and
        // value_found isn't NULL. In that case the user is responsible for freeing it.
        if may_exists && value_found != 0 {
            (
                may_exists,
                Some(unsafe { CSlice::from_raw_parts(val, val_len) }),
            )
        } else {
            (may_exists, None)
        }
    }

    fn create_inner_cf_handle(
        &self,
        name: impl CStrLike,
        opts: &Options,
    ) -> Result<*mut ffi::rocksdb_column_family_handle_t, Error> {
        let cf_name = name.bake().map_err(|err| {
            Error::new(format!(
                "Failed to convert path to CString when creating cf: {err}"
            ))
        })?;
        Ok(unsafe {
            ffi_try!(ffi::rocksdb_create_column_family(
                self.inner.inner(),
                opts.inner,
                cf_name.as_ptr(),
            ))
        })
    }

    pub fn iterator<'a: 'b, 'b>(
        &'a self,
        mode: IteratorMode,
    ) -> DBIteratorWithThreadMode<'b, Self> {
        let readopts = ReadOptions::default();
        self.iterator_opt(mode, readopts)
    }

    pub fn iterator_opt<'a: 'b, 'b>(
        &'a self,
        mode: IteratorMode,
        readopts: ReadOptions,
    ) -> DBIteratorWithThreadMode<'b, Self> {
        DBIteratorWithThreadMode::new(self, readopts, mode)
    }

    /// Opens an iterator using the provided ReadOptions.
    /// This is used when you want to iterate over a specific ColumnFamily with a modified ReadOptions
    pub fn iterator_cf_opt<'a: 'b, 'b>(
        &'a self,
        cf_handle: &impl AsColumnFamilyRef,
        readopts: ReadOptions,
        mode: IteratorMode,
    ) -> DBIteratorWithThreadMode<'b, Self> {
        DBIteratorWithThreadMode::new_cf(self, cf_handle.inner(), readopts, mode)
    }

    /// Opens an iterator with `set_total_order_seek` enabled.
    /// This must be used to iterate across prefixes when `set_memtable_factory` has been called
    /// with a Hash-based implementation.
    pub fn full_iterator<'a: 'b, 'b>(
        &'a self,
        mode: IteratorMode,
    ) -> DBIteratorWithThreadMode<'b, Self> {
        let mut opts = ReadOptions::default();
        opts.set_total_order_seek(true);
        DBIteratorWithThreadMode::new(self, opts, mode)
    }

    pub fn prefix_iterator<'a: 'b, 'b, P: AsRef<[u8]>>(
        &'a self,
        prefix: P,
    ) -> DBIteratorWithThreadMode<'b, Self> {
        let mut opts = ReadOptions::default();
        opts.set_prefix_same_as_start(true);
        DBIteratorWithThreadMode::new(
            self,
            opts,
            IteratorMode::From(prefix.as_ref(), Direction::Forward),
        )
    }

    pub fn iterator_cf<'a: 'b, 'b>(
        &'a self,
        cf_handle: &impl AsColumnFamilyRef,
        mode: IteratorMode,
    ) -> DBIteratorWithThreadMode<'b, Self> {
        let opts = ReadOptions::default();
        DBIteratorWithThreadMode::new_cf(self, cf_handle.inner(), opts, mode)
    }

    pub fn full_iterator_cf<'a: 'b, 'b>(
        &'a self,
        cf_handle: &impl AsColumnFamilyRef,
        mode: IteratorMode,
    ) -> DBIteratorWithThreadMode<'b, Self> {
        let mut opts = ReadOptions::default();
        opts.set_total_order_seek(true);
        DBIteratorWithThreadMode::new_cf(self, cf_handle.inner(), opts, mode)
    }

    pub fn prefix_iterator_cf<'a, P: AsRef<[u8]>>(
        &'a self,
        cf_handle: &impl AsColumnFamilyRef,
        prefix: P,
    ) -> DBIteratorWithThreadMode<'a, Self> {
        let mut opts = ReadOptions::default();
        opts.set_prefix_same_as_start(true);
        DBIteratorWithThreadMode::<'a, Self>::new_cf(
            self,
            cf_handle.inner(),
            opts,
            IteratorMode::From(prefix.as_ref(), Direction::Forward),
        )
    }

    /// Opens a raw iterator over the database, using the default read options
    pub fn raw_iterator<'a: 'b, 'b>(&'a self) -> DBRawIteratorWithThreadMode<'b, Self> {
        let opts = ReadOptions::default();
        DBRawIteratorWithThreadMode::new(self, opts)
    }

    /// Opens a raw iterator over the given column family, using the default read options
    pub fn raw_iterator_cf<'a: 'b, 'b>(
        &'a self,
        cf_handle: &impl AsColumnFamilyRef,
    ) -> DBRawIteratorWithThreadMode<'b, Self> {
        let opts = ReadOptions::default();
        DBRawIteratorWithThreadMode::new_cf(self, cf_handle.inner(), opts)
    }

    /// Opens a raw iterator over the database, using the given read options
    pub fn raw_iterator_opt<'a: 'b, 'b>(
        &'a self,
        readopts: ReadOptions,
    ) -> DBRawIteratorWithThreadMode<'b, Self> {
        DBRawIteratorWithThreadMode::new(self, readopts)
    }

    /// Opens a raw iterator over the given column family, using the given read options
    pub fn raw_iterator_cf_opt<'a: 'b, 'b>(
        &'a self,
        cf_handle: &impl AsColumnFamilyRef,
        readopts: ReadOptions,
    ) -> DBRawIteratorWithThreadMode<'b, Self> {
        DBRawIteratorWithThreadMode::new_cf(self, cf_handle.inner(), readopts)
    }

    pub fn snapshot(&self) -> SnapshotWithThreadMode<Self> {
        SnapshotWithThreadMode::<Self>::new(self)
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
                self.inner.inner(),
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
        cf: &impl AsColumnFamilyRef,
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
                self.inner.inner(),
                writeopts.inner,
                cf.inner(),
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
                self.inner.inner(),
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
        cf: &impl AsColumnFamilyRef,
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
                self.inner.inner(),
                writeopts.inner,
                cf.inner(),
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
                self.inner.inner(),
                writeopts.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
            ));
            Ok(())
        }
    }

    pub fn delete_cf_opt<K: AsRef<[u8]>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        key: K,
        writeopts: &WriteOptions,
    ) -> Result<(), Error> {
        let key = key.as_ref();

        unsafe {
            ffi_try!(ffi::rocksdb_delete_cf(
                self.inner.inner(),
                writeopts.inner,
                cf.inner(),
                key.as_ptr() as *const c_char,
                key.len() as size_t,
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

    pub fn put_cf<K, V>(&self, cf: &impl AsColumnFamilyRef, key: K, value: V) -> Result<(), Error>
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

    pub fn merge_cf<K, V>(&self, cf: &impl AsColumnFamilyRef, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.merge_cf_opt(cf, key.as_ref(), value.as_ref(), &WriteOptions::default())
    }

    pub fn delete<K: AsRef<[u8]>>(&self, key: K) -> Result<(), Error> {
        self.delete_opt(key.as_ref(), &WriteOptions::default())
    }

    pub fn delete_cf<K: AsRef<[u8]>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        key: K,
    ) -> Result<(), Error> {
        self.delete_cf_opt(cf, key.as_ref(), &WriteOptions::default())
    }

    /// Runs a manual compaction on the Range of keys given. This is not likely to be needed for typical usage.
    pub fn compact_range<S: AsRef<[u8]>, E: AsRef<[u8]>>(&self, start: Option<S>, end: Option<E>) {
        unsafe {
            let start = start.as_ref().map(AsRef::as_ref);
            let end = end.as_ref().map(AsRef::as_ref);

            ffi::rocksdb_compact_range(
                self.inner.inner(),
                opt_bytes_to_ptr(start),
                start.map_or(0, <[u8]>::len) as size_t,
                opt_bytes_to_ptr(end),
                end.map_or(0, <[u8]>::len) as size_t,
            );
        }
    }

    /// Same as `compact_range` but with custom options.
    pub fn compact_range_opt<S: AsRef<[u8]>, E: AsRef<[u8]>>(
        &self,
        start: Option<S>,
        end: Option<E>,
        opts: &CompactOptions,
    ) {
        unsafe {
            let start = start.as_ref().map(AsRef::as_ref);
            let end = end.as_ref().map(AsRef::as_ref);

            ffi::rocksdb_compact_range_opt(
                self.inner.inner(),
                opts.inner,
                opt_bytes_to_ptr(start),
                start.map_or(0, <[u8]>::len) as size_t,
                opt_bytes_to_ptr(end),
                end.map_or(0, <[u8]>::len) as size_t,
            );
        }
    }

    /// Runs a manual compaction on the Range of keys given on the
    /// given column family. This is not likely to be needed for typical usage.
    pub fn compact_range_cf<S: AsRef<[u8]>, E: AsRef<[u8]>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        start: Option<S>,
        end: Option<E>,
    ) {
        unsafe {
            let start = start.as_ref().map(AsRef::as_ref);
            let end = end.as_ref().map(AsRef::as_ref);

            ffi::rocksdb_compact_range_cf(
                self.inner.inner(),
                cf.inner(),
                opt_bytes_to_ptr(start),
                start.map_or(0, <[u8]>::len) as size_t,
                opt_bytes_to_ptr(end),
                end.map_or(0, <[u8]>::len) as size_t,
            );
        }
    }

    /// Same as `compact_range_cf` but with custom options.
    pub fn compact_range_cf_opt<S: AsRef<[u8]>, E: AsRef<[u8]>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        start: Option<S>,
        end: Option<E>,
        opts: &CompactOptions,
    ) {
        unsafe {
            let start = start.as_ref().map(AsRef::as_ref);
            let end = end.as_ref().map(AsRef::as_ref);

            ffi::rocksdb_compact_range_cf_opt(
                self.inner.inner(),
                cf.inner(),
                opts.inner,
                opt_bytes_to_ptr(start),
                start.map_or(0, <[u8]>::len) as size_t,
                opt_bytes_to_ptr(end),
                end.map_or(0, <[u8]>::len) as size_t,
            );
        }
    }

    pub fn set_options(&self, opts: &[(&str, &str)]) -> Result<(), Error> {
        let copts = convert_options(opts)?;
        let cnames: Vec<*const c_char> = copts.iter().map(|opt| opt.0.as_ptr()).collect();
        let cvalues: Vec<*const c_char> = copts.iter().map(|opt| opt.1.as_ptr()).collect();
        let count = opts.len() as i32;
        unsafe {
            ffi_try!(ffi::rocksdb_set_options(
                self.inner.inner(),
                count,
                cnames.as_ptr(),
                cvalues.as_ptr(),
            ));
        }
        Ok(())
    }

    pub fn set_options_cf(
        &self,
        cf: &impl AsColumnFamilyRef,
        opts: &[(&str, &str)],
    ) -> Result<(), Error> {
        let copts = convert_options(opts)?;
        let cnames: Vec<*const c_char> = copts.iter().map(|opt| opt.0.as_ptr()).collect();
        let cvalues: Vec<*const c_char> = copts.iter().map(|opt| opt.1.as_ptr()).collect();
        let count = opts.len() as i32;
        unsafe {
            ffi_try!(ffi::rocksdb_set_options_cf(
                self.inner.inner(),
                cf.inner(),
                count,
                cnames.as_ptr(),
                cvalues.as_ptr(),
            ));
        }
        Ok(())
    }

    /// Implementation for property_value et al methods.
    ///
    /// `name` is the name of the property.  It will be converted into a CString
    /// and passed to `get_property` as argument.  `get_property` reads the
    /// specified property and either returns NULL or a pointer to a C allocated
    /// string; this method takes ownership of that string and will free it at
    /// the end. That string is parsed using `parse` callback which produces
    /// the returned result.
    fn property_value_impl<R>(
        name: impl CStrLike,
        get_property: impl FnOnce(*const c_char) -> *mut c_char,
        parse: impl FnOnce(&str) -> Result<R, Error>,
    ) -> Result<Option<R>, Error> {
        let value = match name.bake() {
            Ok(prop_name) => get_property(prop_name.as_ptr()),
            Err(e) => {
                return Err(Error::new(format!(
                    "Failed to convert property name to CString: {e}"
                )));
            }
        };
        if value.is_null() {
            return Ok(None);
        }
        let result = match unsafe { CStr::from_ptr(value) }.to_str() {
            Ok(s) => parse(s).map(|value| Some(value)),
            Err(e) => Err(Error::new(format!(
                "Failed to convert property value to string: {e}"
            ))),
        };
        unsafe {
            ffi::rocksdb_free(value as *mut c_void);
        }
        result
    }

    /// Retrieves a RocksDB property by name.
    ///
    /// Full list of properties could be find
    /// [here](https://github.com/facebook/rocksdb/blob/08809f5e6cd9cc4bc3958dd4d59457ae78c76660/include/rocksdb/db.h#L428-L634).
    pub fn property_value(&self, name: impl CStrLike) -> Result<Option<String>, Error> {
        Self::property_value_impl(
            name,
            |prop_name| unsafe { ffi::rocksdb_property_value(self.inner.inner(), prop_name) },
            |str_value| Ok(str_value.to_owned()),
        )
    }

    /// Retrieves a RocksDB property by name, for a specific column family.
    ///
    /// Full list of properties could be find
    /// [here](https://github.com/facebook/rocksdb/blob/08809f5e6cd9cc4bc3958dd4d59457ae78c76660/include/rocksdb/db.h#L428-L634).
    pub fn property_value_cf(
        &self,
        cf: &impl AsColumnFamilyRef,
        name: impl CStrLike,
    ) -> Result<Option<String>, Error> {
        Self::property_value_impl(
            name,
            |prop_name| unsafe {
                ffi::rocksdb_property_value_cf(self.inner.inner(), cf.inner(), prop_name)
            },
            |str_value| Ok(str_value.to_owned()),
        )
    }

    fn parse_property_int_value(value: &str) -> Result<u64, Error> {
        value.parse::<u64>().map_err(|err| {
            Error::new(format!(
                "Failed to convert property value {value} to int: {err}"
            ))
        })
    }

    /// Retrieves a RocksDB property and casts it to an integer.
    ///
    /// Full list of properties that return int values could be find
    /// [here](https://github.com/facebook/rocksdb/blob/08809f5e6cd9cc4bc3958dd4d59457ae78c76660/include/rocksdb/db.h#L654-L689).
    pub fn property_int_value(&self, name: impl CStrLike) -> Result<Option<u64>, Error> {
        Self::property_value_impl(
            name,
            |prop_name| unsafe { ffi::rocksdb_property_value(self.inner.inner(), prop_name) },
            Self::parse_property_int_value,
        )
    }

    /// Retrieves a RocksDB property for a specific column family and casts it to an integer.
    ///
    /// Full list of properties that return int values could be find
    /// [here](https://github.com/facebook/rocksdb/blob/08809f5e6cd9cc4bc3958dd4d59457ae78c76660/include/rocksdb/db.h#L654-L689).
    pub fn property_int_value_cf(
        &self,
        cf: &impl AsColumnFamilyRef,
        name: impl CStrLike,
    ) -> Result<Option<u64>, Error> {
        Self::property_value_impl(
            name,
            |prop_name| unsafe {
                ffi::rocksdb_property_value_cf(self.inner.inner(), cf.inner(), prop_name)
            },
            Self::parse_property_int_value,
        )
    }

    /// The sequence number of the most recent transaction.
    pub fn latest_sequence_number(&self) -> u64 {
        unsafe { ffi::rocksdb_get_latest_sequence_number(self.inner.inner()) }
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
            let iter = ffi_try!(ffi::rocksdb_get_updates_since(
                self.inner.inner(),
                seq_number,
                opts
            ));
            Ok(DBWALIterator {
                inner: iter,
                start_seq_number: seq_number,
            })
        }
    }

    /// Tries to catch up with the primary by reading as much as possible from the
    /// log files.
    pub fn try_catch_up_with_primary(&self) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_try_catch_up_with_primary(self.inner.inner()));
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
        let paths_v: Vec<CString> = paths.iter().map(to_cpath).collect::<Result<Vec<_>, _>>()?;
        let cpaths: Vec<_> = paths_v.iter().map(|path| path.as_ptr()).collect();

        self.ingest_external_file_raw(opts, &paths_v, &cpaths)
    }

    /// Loads a list of external SST files created with SstFileWriter into the DB for given Column Family
    /// with default opts
    pub fn ingest_external_file_cf<P: AsRef<Path>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        paths: Vec<P>,
    ) -> Result<(), Error> {
        let opts = IngestExternalFileOptions::default();
        self.ingest_external_file_cf_opts(cf, &opts, paths)
    }

    /// Loads a list of external SST files created with SstFileWriter into the DB for given Column Family
    pub fn ingest_external_file_cf_opts<P: AsRef<Path>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        opts: &IngestExternalFileOptions,
        paths: Vec<P>,
    ) -> Result<(), Error> {
        let paths_v: Vec<CString> = paths.iter().map(to_cpath).collect::<Result<Vec<_>, _>>()?;
        let cpaths: Vec<_> = paths_v.iter().map(|path| path.as_ptr()).collect();

        self.ingest_external_file_raw_cf(cf, opts, &paths_v, &cpaths)
    }

    fn ingest_external_file_raw(
        &self,
        opts: &IngestExternalFileOptions,
        paths_v: &[CString],
        cpaths: &[*const c_char],
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_ingest_external_file(
                self.inner.inner(),
                cpaths.as_ptr(),
                paths_v.len(),
                opts.inner as *const _
            ));
            Ok(())
        }
    }

    fn ingest_external_file_raw_cf(
        &self,
        cf: &impl AsColumnFamilyRef,
        opts: &IngestExternalFileOptions,
        paths_v: &[CString],
        cpaths: &[*const c_char],
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_ingest_external_file_cf(
                self.inner.inner(),
                cf.inner(),
                cpaths.as_ptr(),
                paths_v.len(),
                opts.inner as *const _
            ));
            Ok(())
        }
    }

    /// Returns a list of all table files with their level, start key
    /// and end key
    pub fn live_files(&self) -> Result<Vec<LiveFile>, Error> {
        unsafe {
            let files = ffi::rocksdb_livefiles(self.inner.inner());
            if files.is_null() {
                Err(Error::new("Could not get live files".to_owned()))
            } else {
                let n = ffi::rocksdb_livefiles_count(files);

                let mut livefiles = Vec::with_capacity(n as usize);
                let mut key_size: usize = 0;

                for i in 0..n {
                    let column_family_name =
                        from_cstr(ffi::rocksdb_livefiles_column_family_name(files, i));
                    let name = from_cstr(ffi::rocksdb_livefiles_name(files, i));
                    let size = ffi::rocksdb_livefiles_size(files, i);
                    let level = ffi::rocksdb_livefiles_level(files, i);

                    // get smallest key inside file
                    let smallest_key = ffi::rocksdb_livefiles_smallestkey(files, i, &mut key_size);
                    let smallest_key = raw_data(smallest_key, key_size);

                    // get largest key inside file
                    let largest_key = ffi::rocksdb_livefiles_largestkey(files, i, &mut key_size);
                    let largest_key = raw_data(largest_key, key_size);

                    livefiles.push(LiveFile {
                        column_family_name,
                        name,
                        size,
                        level,
                        start_key: smallest_key,
                        end_key: largest_key,
                        num_entries: ffi::rocksdb_livefiles_entries(files, i),
                        num_deletions: ffi::rocksdb_livefiles_deletions(files, i),
                    });
                }

                // destroy livefiles metadata(s)
                ffi::rocksdb_livefiles_destroy(files);

                // return
                Ok(livefiles)
            }
        }
    }

    /// Delete sst files whose keys are entirely in the given range.
    ///
    /// Could leave some keys in the range which are in files which are not
    /// entirely in the range.
    ///
    /// Note: L0 files are left regardless of whether they're in the range.
    ///
    /// SnapshotWithThreadModes before the delete might not see the data in the given range.
    pub fn delete_file_in_range<K: AsRef<[u8]>>(&self, from: K, to: K) -> Result<(), Error> {
        let from = from.as_ref();
        let to = to.as_ref();
        unsafe {
            ffi_try!(ffi::rocksdb_delete_file_in_range(
                self.inner.inner(),
                from.as_ptr() as *const c_char,
                from.len() as size_t,
                to.as_ptr() as *const c_char,
                to.len() as size_t,
            ));
            Ok(())
        }
    }

    /// Same as `delete_file_in_range` but only for specific column family
    pub fn delete_file_in_range_cf<K: AsRef<[u8]>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        from: K,
        to: K,
    ) -> Result<(), Error> {
        let from = from.as_ref();
        let to = to.as_ref();
        unsafe {
            ffi_try!(ffi::rocksdb_delete_file_in_range_cf(
                self.inner.inner(),
                cf.inner(),
                from.as_ptr() as *const c_char,
                from.len() as size_t,
                to.as_ptr() as *const c_char,
                to.len() as size_t,
            ));
            Ok(())
        }
    }

    /// Request stopping background work, if wait is true wait until it's done.
    pub fn cancel_all_background_work(&self, wait: bool) {
        unsafe {
            ffi::rocksdb_cancel_all_background_work(self.inner.inner(), c_uchar::from(wait));
        }
    }

    fn drop_column_family<C>(
        &self,
        cf_inner: *mut ffi::rocksdb_column_family_handle_t,
        cf: C,
    ) -> Result<(), Error> {
        unsafe {
            // first mark the column family as dropped
            ffi_try!(ffi::rocksdb_drop_column_family(
                self.inner.inner(),
                cf_inner
            ));
        }
        // then finally reclaim any resources (mem, files) by destroying the only single column
        // family handle by drop()-ing it
        drop(cf);
        Ok(())
    }
}

impl<I: DBInner> DBCommon<SingleThreaded, I> {
    /// Creates column family with given name and options
    pub fn create_cf<N: AsRef<str>>(&mut self, name: N, opts: &Options) -> Result<(), Error> {
        let inner = self.create_inner_cf_handle(name.as_ref(), opts)?;
        self.cfs
            .cfs
            .insert(name.as_ref().to_string(), ColumnFamily { inner });
        Ok(())
    }

    /// Drops the column family with the given name
    pub fn drop_cf(&mut self, name: &str) -> Result<(), Error> {
        if let Some(cf) = self.cfs.cfs.remove(name) {
            self.drop_column_family(cf.inner, cf)
        } else {
            Err(Error::new(format!("Invalid column family: {name}")))
        }
    }

    /// Returns the underlying column family handle
    pub fn cf_handle(&self, name: &str) -> Option<&ColumnFamily> {
        self.cfs.cfs.get(name)
    }
}

impl<I: DBInner> DBCommon<MultiThreaded, I> {
    /// Creates column family with given name and options
    pub fn create_cf<N: AsRef<str>>(&self, name: N, opts: &Options) -> Result<(), Error> {
        let inner = self.create_inner_cf_handle(name.as_ref(), opts)?;
        self.cfs.cfs.write().unwrap().insert(
            name.as_ref().to_string(),
            Arc::new(UnboundColumnFamily { inner }),
        );
        Ok(())
    }

    /// Drops the column family with the given name by internally locking the inner column
    /// family map. This avoids needing `&mut self` reference
    pub fn drop_cf(&self, name: &str) -> Result<(), Error> {
        if let Some(cf) = self.cfs.cfs.write().unwrap().remove(name) {
            self.drop_column_family(cf.inner, cf)
        } else {
            Err(Error::new(format!("Invalid column family: {name}")))
        }
    }

    /// Returns the underlying column family handle
    pub fn cf_handle(&self, name: &str) -> Option<Arc<BoundColumnFamily>> {
        self.cfs
            .cfs
            .read()
            .unwrap()
            .get(name)
            .cloned()
            .map(UnboundColumnFamily::bound_column_family)
    }
}

impl<T: ThreadMode, I: DBInner> Drop for DBCommon<T, I> {
    fn drop(&mut self) {
        self.cfs.drop_all_cfs_internal();
    }
}

impl<T: ThreadMode, I: DBInner> fmt::Debug for DBCommon<T, I> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RocksDB {{ path: {:?} }}", self.path())
    }
}

/// The metadata that describes a SST file
#[derive(Debug, Clone)]
pub struct LiveFile {
    /// Name of the column family the file belongs to
    pub column_family_name: String,
    /// Name of the file
    pub name: String,
    /// Size of the file
    pub size: usize,
    /// Level at which this file resides
    pub level: i32,
    /// Smallest user defined key in the file
    pub start_key: Option<Vec<u8>>,
    /// Largest user defined key in the file
    pub end_key: Option<Vec<u8>>,
    /// Number of entries/alive keys in the file
    pub num_entries: u64,
    /// Number of deletions/tomb key(s) in the file
    pub num_deletions: u64,
}

fn convert_options(opts: &[(&str, &str)]) -> Result<Vec<(CString, CString)>, Error> {
    opts.iter()
        .map(|(name, value)| {
            let cname = match CString::new(name.as_bytes()) {
                Ok(cname) => cname,
                Err(e) => return Err(Error::new(format!("Invalid option name `{e}`"))),
            };
            let cvalue = match CString::new(value.as_bytes()) {
                Ok(cvalue) => cvalue,
                Err(e) => return Err(Error::new(format!("Invalid option value: `{e}`"))),
            };
            Ok((cname, cvalue))
        })
        .collect()
}

pub(crate) fn convert_values(
    values: Vec<*mut c_char>,
    values_sizes: Vec<usize>,
    errors: Vec<*mut c_char>,
) -> Vec<Result<Option<Vec<u8>>, Error>> {
    values
        .into_iter()
        .zip(values_sizes.into_iter())
        .zip(errors.into_iter())
        .map(|((v, s), e)| {
            if e.is_null() {
                let value = unsafe { crate::ffi_util::raw_data(v, s) };
                unsafe {
                    ffi::rocksdb_free(v as *mut c_void);
                }
                Ok(value)
            } else {
                Err(Error::new(crate::ffi_util::error_message(e)))
            }
        })
        .collect()
}
