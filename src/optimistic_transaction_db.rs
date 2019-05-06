use crate::{
    db_iterator::DBRawIterator,
    db_options::ReadOptions,
    db_vector::DBVector,
    handle::{ConstHandle, Handle},
    open_raw::{OpenRaw, OpenRawFFI},
    ops::*,
    ColumnFamily, Error, Transaction, WriteOptions,
};
use ffi;
use libc::c_uchar;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::ptr;

/// A optimistic transaction database.
pub struct OptimisticTransactionDB {
    inner: *mut ffi::rocksdb_optimistictransactiondb_t,
    path: PathBuf,
    cfs: BTreeMap<String, ColumnFamily>,
    base_db: *mut ffi::rocksdb_t,
}

impl Handle<ffi::rocksdb_optimistictransactiondb_t> for OptimisticTransactionDB {
    fn handle(&self) -> *mut ffi::rocksdb_optimistictransactiondb_t {
        self.inner
    }
}

impl Open for OptimisticTransactionDB {}
impl OpenCF for OptimisticTransactionDB {}

impl OpenRaw for OptimisticTransactionDB {
    type Pointer = ffi::rocksdb_optimistictransactiondb_t;
    type Descriptor = ();

    fn open_ffi(input: OpenRawFFI<'_, Self::Descriptor>) -> Result<*mut Self::Pointer, Error> {
        let pointer = unsafe {
            if input.num_column_families <= 0 {
                ffi_try!(ffi::rocksdb_optimistictransactiondb_open(
                    input.options,
                    input.path,
                ))
            } else {
                ffi_try!(ffi::rocksdb_optimistictransactiondb_open_column_families(
                    input.options,
                    input.path,
                    input.num_column_families,
                    input.column_family_names,
                    input.column_family_options,
                    input.column_family_handles,
                ))
            }
        };

        Ok(pointer)
    }

    fn build<I>(
        path: PathBuf,
        _open_descriptor: Self::Descriptor,
        pointer: *mut Self::Pointer,
        column_families: I,
    ) -> Result<Self, Error>
    where
        I: IntoIterator<Item = (String, *mut ffi::rocksdb_column_family_handle_t)>,
    {
        let cfs: BTreeMap<_, _> = column_families
            .into_iter()
            .map(|(k, h)| (k, ColumnFamily::new(h)))
            .collect();
        let base_db = unsafe { ffi::rocksdb_optimistictransactiondb_get_base_db(pointer) };
        Ok(OptimisticTransactionDB {
            inner: pointer,
            cfs,
            path,
            base_db,
        })
    }
}

impl Read for OptimisticTransactionDB {}
impl Write for OptimisticTransactionDB {}

unsafe impl Send for OptimisticTransactionDB {}
unsafe impl Sync for OptimisticTransactionDB {}

impl GetColumnFamilys for OptimisticTransactionDB {
    fn get_cfs(&self) -> &BTreeMap<String, ColumnFamily> {
        &self.cfs
    }
    fn get_mut_cfs(&mut self) -> &mut BTreeMap<String, ColumnFamily> {
        &mut self.cfs
    }
}

impl OptimisticTransactionDB {
    pub fn path(&self) -> &Path {
        &self.path.as_path()
    }
}

impl Drop for OptimisticTransactionDB {
    fn drop(&mut self) {
        unsafe {
            for cf in self.cfs.values() {
                ffi::rocksdb_column_family_handle_destroy(cf.inner);
            }
            ffi::rocksdb_optimistictransactiondb_close_base_db(self.base_db);
            ffi::rocksdb_optimistictransactiondb_close(self.inner);
        }
    }
}

impl TransactionBegin for OptimisticTransactionDB {
    type WriteOptions = WriteOptions;
    type TransactionOptions = OptimisticTransactionOptions;
    fn transaction(
        &self,
        write_options: &WriteOptions,
        tx_options: &OptimisticTransactionOptions,
    ) -> Transaction<OptimisticTransactionDB> {
        unsafe {
            let inner = ffi::rocksdb_optimistictransaction_begin(
                self.inner,
                write_options.handle(),
                tx_options.inner,
                ptr::null_mut(),
            );
            Transaction::new(inner)
        }
    }
}

pub struct OptimisticTransactionOptions {
    inner: *mut ffi::rocksdb_optimistictransaction_options_t,
}

impl OptimisticTransactionOptions {
    /// Create new optimistic transaction options
    pub fn new() -> OptimisticTransactionOptions {
        unsafe {
            let inner = ffi::rocksdb_optimistictransaction_options_create();
            OptimisticTransactionOptions { inner }
        }
    }

    /// Set a snapshot at start of transaction by setting set_snapshot=true
    /// Default: false
    pub fn set_snapshot(&mut self, set_snapshot: bool) {
        unsafe {
            ffi::rocksdb_optimistictransaction_options_set_set_snapshot(
                self.inner,
                set_snapshot as c_uchar,
            );
        }
    }
}

impl Drop for OptimisticTransactionOptions {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_optimistictransaction_options_destroy(self.inner);
        }
    }
}

impl Default for OptimisticTransactionOptions {
    fn default() -> OptimisticTransactionOptions {
        OptimisticTransactionOptions::new()
    }
}

impl Handle<ffi::rocksdb_t> for OptimisticTransactionDB {
    fn handle(&self) -> *mut ffi::rocksdb_t {
        self.base_db
    }
}

impl Iterate for OptimisticTransactionDB {
    fn get_raw_iter(&self, readopts: &ReadOptions) -> DBRawIterator {
        unsafe {
            DBRawIterator {
                inner: ffi::rocksdb_create_iterator(self.base_db, readopts.handle()),
                db: PhantomData,
            }
        }
    }
}

impl IterateCF for OptimisticTransactionDB {
    fn get_raw_iter_cf(
        &self,
        cf_handle: &ColumnFamily,
        readopts: &ReadOptions,
    ) -> Result<DBRawIterator, Error> {
        unsafe {
            Ok(DBRawIterator {
                inner: ffi::rocksdb_create_iterator_cf(
                    self.base_db,
                    readopts.handle(),
                    cf_handle.inner,
                ),
                db: PhantomData,
            })
        }
    }
}

impl OptimisticTransactionDB {
    pub fn snapshot(&self) -> Snapshot {
        let snapshot = unsafe { ffi::rocksdb_create_snapshot(self.base_db) };
        Snapshot {
            db: self,
            inner: snapshot,
        }
    }
}

pub struct Snapshot<'a> {
    db: &'a OptimisticTransactionDB,
    inner: *const ffi::rocksdb_snapshot_t,
}

impl<'a> ConstHandle<ffi::rocksdb_snapshot_t> for Snapshot<'a> {
    fn const_handle(&self) -> *const ffi::rocksdb_snapshot_t {
        self.inner
    }
}

impl<'a> Read for Snapshot<'a> {}

impl<'a> GetCF<ReadOptions> for Snapshot<'a> {
    fn get_cf_full<K: AsRef<[u8]>>(
        &self,
        cf: Option<&ColumnFamily>,
        key: K,
        readopts: Option<&ReadOptions>,
    ) -> Result<Option<DBVector>, Error> {
        let mut ro = readopts.cloned().unwrap_or_default();
        ro.set_snapshot(self);

        self.db.get_cf_full(cf, key, Some(&ro))
    }
}

impl<'a> Drop for Snapshot<'a> {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_release_snapshot(self.db.base_db, self.inner);
        }
    }
}

impl<'a> Iterate for Snapshot<'a> {
    fn get_raw_iter(&self, readopts: &ReadOptions) -> DBRawIterator {
        let mut ro = readopts.to_owned();
        ro.set_snapshot(self);
        self.db.get_raw_iter(&ro)
    }
}

impl<'a> IterateCF for Snapshot<'a> {
    fn get_raw_iter_cf(
        &self,
        cf_handle: &ColumnFamily,
        readopts: &ReadOptions,
    ) -> Result<DBRawIterator, Error> {
        let mut ro = readopts.to_owned();
        ro.set_snapshot(self);
        self.db.get_raw_iter_cf(cf_handle, &ro)
    }
}
