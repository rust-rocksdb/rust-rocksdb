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
    ffi_util::{from_cstr, raw_data, to_cpath},
    handle::Handle,
    ops::{
        backup::BackupInternal,
        checkpoint::CheckpointInternal,
        column_family::{CreateColumnFamily, DropColumnFamily, GetColumnFamily},
        compact_range::{CompactRangeCFOpt, CompactRangeOpt},
        delete::{DeleteCFOpt, DeleteOpt},
        delete_file_in_range::{DeleteFileInRange, DeleteFileInRangeCF},
        delete_range::DeleteRangeCFOpt,
        flush::{FlushCFOpt, FlushOpt},
        get_pinned::{GetPinnedCFOpt, GetPinnedOpt},
        ingest_external_file::{IngestExternalFileCFOpt, IngestExternalFileOpt},
        iterate::{Iterate, IterateCF},
        merge::{MergeCFOpt, MergeOpt},
        multi_get::{MultiGetCFOpt, MultiGetOpt},
        perf::PerfInternal,
        property::{GetProperty, GetPropertyCF},
        put::{PutCFOpt, PutOpt},
        set_options::{SetOptions, SetOptionsCF},
        snapshot::SnapshotInternal,
        write_batch::WriteBatchWriteOpt,
        GetColumnFamilies,
    },
    ColumnFamily, ColumnFamilyDescriptor, CompactOptions, DBIterator, DBPinnableSlice,
    DBRawIterator, DBWALIterator, Error, FlushOptions, IngestExternalFileOptions, IteratorMode,
    Options, ReadOptions, Snapshot, WriteBatch, WriteOptions, DEFAULT_COLUMN_FAMILY_NAME,
};

use ambassador::Delegate;
use libc::{self, c_char, c_int, c_uchar};
use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::fmt;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::ptr;
use std::slice;
use std::time::Duration;

/// A RocksDB database.
///
/// See crate level documentation for a simple usage example.
pub struct DBInner {
    pub(crate) inner: *mut ffi::rocksdb_t,
    cfs: BTreeMap<String, ColumnFamily>,
    path: PathBuf,
}

// Safety note: auto-implementing Send on most db-related types is prevented by the inner FFI
// pointer. In most cases, however, this pointer is Send-safe because it is never aliased and
// rocksdb internally does not rely on thread-local information for its user-exposed types.
unsafe impl Send for DBInner {}

// Sync is similarly safe for many types because they do not expose interior mutability, and their
// use within the rocksdb library is generally behind a const reference
unsafe impl Sync for DBInner {}

impl Handle<ffi::rocksdb_t> for DBInner {
    fn handle(&self) -> *mut ffi::rocksdb_t {
        self.inner
    }
}

// Specifies whether open DB for read only.
enum AccessType<'a> {
    ReadWrite,
    ReadOnly { error_if_log_file_exist: bool },
    Secondary { secondary_path: &'a Path },
    WithTTL { ttl: Duration },
}

impl DBInner {
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

        Ok(Self {
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

    pub fn path(&self) -> &Path {
        &self.path.as_path()
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

    /// Returns a list of all table files with their level, start key
    /// and end key
    pub fn live_files(&self) -> Result<Vec<LiveFile>, Error> {
        unsafe {
            let files = ffi::rocksdb_livefiles(self.inner);
            if files.is_null() {
                Err(Error::new("Could not get live files".to_owned()))
            } else {
                let n = ffi::rocksdb_livefiles_count(files);

                let mut livefiles = Vec::with_capacity(n as usize);
                let mut key_size: usize = 0;

                for i in 0..n {
                    let name = from_cstr(ffi::rocksdb_livefiles_name(files, i));
                    let size = ffi::rocksdb_livefiles_size(files, i);
                    let level = ffi::rocksdb_livefiles_level(files, i) as i32;

                    // get smallest key inside file
                    let smallest_key = ffi::rocksdb_livefiles_smallestkey(files, i, &mut key_size);
                    let smallest_key = raw_data(smallest_key, key_size);

                    // get largest key inside file
                    let largest_key = ffi::rocksdb_livefiles_largestkey(files, i, &mut key_size);
                    let largest_key = raw_data(largest_key, key_size);

                    livefiles.push(LiveFile {
                        name,
                        size,
                        level,
                        start_key: smallest_key,
                        end_key: largest_key,
                        num_entries: ffi::rocksdb_livefiles_entries(files, i),
                        num_deletions: ffi::rocksdb_livefiles_deletions(files, i),
                    })
                }

                // destroy livefiles metadata(s)
                ffi::rocksdb_livefiles_destroy(files);

                // return
                Ok(livefiles)
            }
        }
    }

    /// Request stopping background work, if wait is true wait until it's done.
    pub fn cancel_all_background_work(&self, wait: bool) {
        unsafe {
            ffi::rocksdb_cancel_all_background_work(self.inner, wait as u8);
        }
    }

    unsafe fn create_snapshot_rocksdb<'a, D>(&self, db: &'a D) -> Snapshot<'a, D>
    where
        D: SnapshotInternal<DB = D>,
    {
        let inner = ffi::rocksdb_create_snapshot(self.handle());
        Snapshot { db, inner }
    }

    unsafe fn release_snapshot_rocksdb<'a, D>(&self, snapshot: &mut Snapshot<'a, D>)
    where
        D: SnapshotInternal<DB = D>,
    {
        ffi::rocksdb_release_snapshot(self.handle(), snapshot.inner);
    }
}

impl GetColumnFamilies for DBInner {
    fn get_cfs(&self) -> &BTreeMap<String, ColumnFamily> {
        &self.cfs
    }

    fn get_mut_cfs(&mut self) -> &mut BTreeMap<String, ColumnFamily> {
        &mut self.cfs
    }
}

impl Drop for DBInner {
    fn drop(&mut self) {
        unsafe {
            for cf in self.cfs.values() {
                ffi::rocksdb_column_family_handle_destroy(cf.inner);
            }
            ffi::rocksdb_close(self.inner);
        }
    }
}

impl fmt::Debug for DBInner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RocksDB {{ path: {:?} }}", self.path())
    }
}

/// The metadata that describes a SST file
#[derive(Debug, Clone)]
pub struct LiveFile {
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

pub struct DBUtils;

impl DBUtils {
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
}

macro_rules! make_new_db_with_traits {
    ($struct_name:ident, [$($d:ty),+])  => (
        #[derive(Delegate)]
        $(#[delegate($d)])+
        pub struct $struct_name(DBInner);

        #[allow(clippy::inline_always)]
        impl $struct_name {
            #[inline(always)]
            pub fn path(&self) -> &Path {
                self.0.path()
            }

            #[inline(always)]
            pub fn latest_sequence_number(&self) -> u64 {
                self.0.latest_sequence_number()
            }

            #[inline(always)]
            pub fn get_updates_since(&self, seq_number: u64) -> Result<DBWALIterator, Error> {
               self.0.get_updates_since(seq_number)
            }

            #[inline(always)]
            pub fn live_files(&self) -> Result<Vec<LiveFile>, Error> {
                self.0.live_files()
            }

            #[inline(always)]
            pub fn cancel_all_background_work(&self, wait: bool) {
                self.0.cancel_all_background_work(wait)
            }
        }

        impl SnapshotInternal for $struct_name {
            type DB = Self;

            unsafe fn create_snapshot(&self) -> Snapshot<Self> {
                self.0.create_snapshot_rocksdb(self)
            }

            unsafe fn release_snapshot(&self, snapshot: &mut Snapshot<Self>) {
                self.0.release_snapshot_rocksdb(snapshot)
            }
        }

        impl MultiGetOpt<&ReadOptions> for $struct_name {
            fn multi_get_opt<K, I>(&self, keys: I, readopts: &ReadOptions) -> Result<Vec<Vec<u8>>, Error>
            where
                K: AsRef<[u8]>,
                I: IntoIterator<Item = K>,
            {
                self.0.multi_get_opt(keys, readopts)
            }
        }

        impl MultiGetCFOpt<&ReadOptions> for $struct_name {
            fn multi_get_cf_opt<'c, K, I>(
                &self,
                keys: I,
                readopts: &ReadOptions,
            ) -> Result<Vec<Vec<u8>>, Error>
            where
                K: AsRef<[u8]>,
                I: IntoIterator<Item = (&'c ColumnFamily, K)>,
            {
                self.0.multi_get_cf_opt(keys, readopts)
            }
        }

    )
}

make_new_db_with_traits!(
    DB,
    [
        BackupInternal,
        CheckpointInternal,
        CreateColumnFamily,
        DropColumnFamily,
        GetColumnFamily,
        CompactRangeCFOpt,
        CompactRangeOpt,
        DeleteCFOpt,
        DeleteOpt,
        DeleteRangeCFOpt,
        DeleteFileInRange,
        DeleteFileInRangeCF,
        FlushCFOpt,
        FlushOpt,
        GetPinnedCFOpt,
        GetPinnedOpt,
        IngestExternalFileCFOpt,
        IngestExternalFileOpt,
        Iterate,
        IterateCF,
        MergeCFOpt,
        MergeOpt,
        GetProperty,
        GetPropertyCF,
        PerfInternal,
        PutCFOpt,
        PutOpt,
        SetOptions,
        SetOptionsCF,
        WriteBatchWriteOpt
    ]
);

impl DB {
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

        DBInner::open_cf_descriptors_internal(opts, path, cfs, &AccessType::ReadWrite).map(Self)
    }

    /// Opens a database with the given database options and column family descriptors.
    pub fn open_cf_descriptors<P, I>(opts: &Options, path: P, cfs: I) -> Result<Self, Error>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = ColumnFamilyDescriptor>,
    {
        DBInner::open_cf_descriptors_internal(opts, path, cfs, &AccessType::ReadWrite).map(Self)
    }
}

make_new_db_with_traits!(
    ReadOnlyDB,
    [
        BackupInternal,
        CheckpointInternal,
        GetColumnFamily,
        GetPinnedCFOpt,
        GetPinnedOpt,
        Iterate,
        IterateCF,
        GetProperty,
        GetPropertyCF,
        PerfInternal
    ]
);

impl ReadOnlyDB {
    /// Opens the database for read only with the specified options.
    pub fn open<P: AsRef<Path>>(
        opts: &Options,
        path: P,
        error_if_log_file_exist: bool,
    ) -> Result<Self, Error> {
        Self::open_cf(opts, path, None::<&str>, error_if_log_file_exist)
    }

    /// Opens a database for read only with the given database options and column family names.
    pub fn open_cf<P, I, N>(
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

        DBInner::open_cf_descriptors_internal(
            opts,
            path,
            cfs,
            &AccessType::ReadOnly {
                error_if_log_file_exist,
            },
        )
        .map(Self)
    }
}

make_new_db_with_traits!(
    SecondaryDB,
    [
        BackupInternal,
        CheckpointInternal,
        GetColumnFamily,
        GetPinnedCFOpt,
        GetPinnedOpt,
        Iterate,
        IterateCF,
        GetProperty,
        GetPropertyCF,
        PerfInternal
    ]
);

impl SecondaryDB {
    /// Opens the database as a secondary.
    pub fn open<P: AsRef<Path>>(
        opts: &Options,
        primary_path: P,
        secondary_path: P,
    ) -> Result<Self, Error> {
        Self::open_cf(opts, primary_path, secondary_path, None::<&str>)
    }

    /// Opens the database as a secondary with the given database options and column family names.
    pub fn open_cf<P, I, N>(
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

        DBInner::open_cf_descriptors_internal(
            opts,
            primary_path,
            cfs,
            &AccessType::Secondary {
                secondary_path: secondary_path.as_ref(),
            },
        )
        .map(Self)
    }

    /// Tries to catch up with the primary by reading as much as possible from the
    /// log files.
    pub fn try_catch_up_with_primary(&self) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_try_catch_up_with_primary(self.0.inner));
        }
        Ok(())
    }
}

make_new_db_with_traits!(
    DBWithTTL,
    [
        BackupInternal,
        CheckpointInternal,
        CompactRangeOpt,
        DeleteOpt,
        DeleteFileInRange,
        FlushOpt,
        GetPinnedOpt,
        IngestExternalFileOpt,
        Iterate,
        MergeOpt,
        GetProperty,
        PerfInternal,
        PutOpt,
        SetOptions,
        WriteBatchWriteOpt
    ]
);

impl DBWithTTL {
    /// Opens the database with a Time to Live compaction filter.
    pub fn open<P: AsRef<Path>>(opts: &Options, path: P, ttl: Duration) -> Result<Self, Error> {
        DBInner::open_cf_descriptors_internal(opts, path, None, &AccessType::WithTTL { ttl })
            .map(Self)
    }
}
