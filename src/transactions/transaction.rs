// Copyright 2021 Yiyuan Liu
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

use std::{marker::PhantomData, ptr};

use crate::{
    db::{convert_values, DBAccess},
    ffi, AsColumnFamilyRef, DBIteratorWithThreadMode, DBPinnableSlice, DBRawIteratorWithThreadMode,
    Direction, Error, IteratorMode, ReadOptions, SnapshotWithThreadMode, WriteBatchWithTransaction,
};
use libc::{c_char, c_void, size_t};

/// RocksDB Transaction.
///
/// To use transactions, you must first create a [`TransactionDB`] or [`OptimisticTransactionDB`].
///
/// [`TransactionDB`]: crate::TransactionDB
/// [`OptimisticTransactionDB`]: crate::OptimisticTransactionDB
pub struct Transaction<'db, DB> {
    pub(crate) inner: *mut ffi::rocksdb_transaction_t,
    pub(crate) _marker: PhantomData<&'db DB>,
}

unsafe impl<'db, DB> Send for Transaction<'db, DB> {}

impl<'db, DB> DBAccess for Transaction<'db, DB> {
    unsafe fn create_snapshot(&self) -> *const ffi::rocksdb_snapshot_t {
        ffi::rocksdb_transaction_get_snapshot(self.inner)
    }

    unsafe fn release_snapshot(&self, snapshot: *const ffi::rocksdb_snapshot_t) {
        ffi::rocksdb_free(snapshot as *mut c_void);
    }

    unsafe fn create_iterator(&self, readopts: &ReadOptions) -> *mut ffi::rocksdb_iterator_t {
        ffi::rocksdb_transaction_create_iterator(self.inner, readopts.inner)
    }

    unsafe fn create_iterator_cf(
        &self,
        cf_handle: *mut ffi::rocksdb_column_family_handle_t,
        readopts: &ReadOptions,
    ) -> *mut ffi::rocksdb_iterator_t {
        ffi::rocksdb_transaction_create_iterator_cf(self.inner, readopts.inner, cf_handle)
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

    fn multi_get_opt<K, I>(
        &self,
        keys: I,
        readopts: &ReadOptions,
    ) -> Vec<Result<Option<Vec<u8>>, Error>>
    where
        K: AsRef<[u8]>,
        I: IntoIterator<Item = K>,
    {
        self.multi_get_opt(keys, readopts)
    }

    fn multi_get_cf_opt<'b, K, I, W>(
        &self,
        keys_cf: I,
        readopts: &ReadOptions,
    ) -> Vec<Result<Option<Vec<u8>>, Error>>
    where
        K: AsRef<[u8]>,
        I: IntoIterator<Item = (&'b W, K)>,
        W: AsColumnFamilyRef + 'b,
    {
        self.multi_get_cf_opt(keys_cf, readopts)
    }
}

impl<'db, DB> Transaction<'db, DB> {
    /// Write all batched keys to the DB atomically.
    ///
    /// May return any error that could be returned by `DB::write`.
    ///
    /// If this transaction was created by a [`TransactionDB`], an error of
    /// the [`Expired`] kind may be returned if this transaction has
    /// lived longer than expiration time in [`TransactionOptions`].
    ///
    /// If this transaction was created by an [`OptimisticTransactionDB`], an error of
    /// the [`Busy`] kind may be returned if the transaction
    /// could not guarantee that there are no write conflicts.
    /// An error of the [`TryAgain`] kind may be returned if the memtable
    /// history size is not large enough (see [`Options::set_max_write_buffer_size_to_maintain`]).
    ///
    /// [`Expired`]: crate::ErrorKind::Expired
    /// [`TransactionOptions`]: crate::TransactionOptions
    /// [`TransactionDB`]: crate::TransactionDB
    /// [`OptimisticTransactionDB`]: crate::OptimisticTransactionDB
    /// [`Busy`]: crate::ErrorKind::Busy
    /// [`TryAgain`]: crate::ErrorKind::TryAgain
    /// [`Options::set_max_write_buffer_size_to_maintain`]: crate::Options::set_max_write_buffer_size_to_maintain
    pub fn commit(self) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_transaction_commit(self.inner));
        }
        Ok(())
    }

    pub fn set_name(&self, name: &[u8]) -> Result<(), Error> {
        let ptr = name.as_ptr();
        let len = name.len();
        unsafe {
            ffi_try!(ffi::rocksdb_transaction_set_name(
                self.inner, ptr as _, len as _
            ));
        }

        Ok(())
    }

    pub fn get_name(&self) -> Option<Vec<u8>> {
        unsafe {
            let mut name_len = 0;
            let name = ffi::rocksdb_transaction_get_name(self.inner, &mut name_len);
            if name.is_null() {
                None
            } else {
                let mut vec = vec![0; name_len];
                std::ptr::copy_nonoverlapping(name as *mut u8, vec.as_mut_ptr(), name_len);
                ffi::rocksdb_free(name as *mut c_void);
                Some(vec)
            }
        }
    }

    pub fn prepare(&self) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_transaction_prepare(self.inner));
        }
        Ok(())
    }

    /// Returns snapshot associated with transaction if snapshot was enabled in [`TransactionOptions`].
    /// Otherwise, returns a snapshot with `nullptr` inside which doesn't effect read operations.
    ///
    /// [`TransactionOptions`]: crate::TransactionOptions
    pub fn snapshot(&self) -> SnapshotWithThreadMode<Self> {
        SnapshotWithThreadMode::new(self)
    }

    /// Discard all batched writes in this transaction.
    pub fn rollback(&self) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_transaction_rollback(self.inner));
            Ok(())
        }
    }

    /// Record the state of the transaction for future calls to [`rollback_to_savepoint`].
    /// May be called multiple times to set multiple save points.
    ///
    /// [`rollback_to_savepoint`]: Self::rollback_to_savepoint
    pub fn set_savepoint(&self) {
        unsafe {
            ffi::rocksdb_transaction_set_savepoint(self.inner);
        }
    }

    /// Undo all operations in this transaction since the most recent call to [`set_savepoint`]
    /// and removes the most recent [`set_savepoint`].
    ///
    /// Returns error if there is no previous call to [`set_savepoint`].
    ///
    /// [`set_savepoint`]: Self::set_savepoint
    pub fn rollback_to_savepoint(&self) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_transaction_rollback_to_savepoint(self.inner));
            Ok(())
        }
    }

    /// Get the bytes associated with a key value.
    ///
    /// See [`get_cf_opt`] for details.
    ///
    /// [`get_cf_opt`]: Self::get_cf_opt
    pub fn get<K: AsRef<[u8]>>(&self, key: K) -> Result<Option<Vec<u8>>, Error> {
        self.get_opt(key, &ReadOptions::default())
    }

    pub fn get_pinned<K: AsRef<[u8]>>(&self, key: K) -> Result<Option<DBPinnableSlice>, Error> {
        self.get_pinned_opt(key, &ReadOptions::default())
    }

    /// Get the bytes associated with a key value and the given column family.
    ///
    /// See [`get_cf_opt`] for details.
    ///
    /// [`get_cf_opt`]: Self::get_cf_opt
    pub fn get_cf<K: AsRef<[u8]>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        key: K,
    ) -> Result<Option<Vec<u8>>, Error> {
        self.get_cf_opt(cf, key, &ReadOptions::default())
    }

    pub fn get_pinned_cf<K: AsRef<[u8]>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        key: K,
    ) -> Result<Option<DBPinnableSlice>, Error> {
        self.get_pinned_cf_opt(cf, key, &ReadOptions::default())
    }

    /// Get the key and ensure that this transaction will only
    /// be able to be committed if this key is not written outside this
    /// transaction after it has first been read (or after the snapshot if a
    /// snapshot is set in this transaction).
    ///
    /// See [`get_for_update_cf_opt`] for details.
    ///
    /// [`get_for_update_cf_opt`]: Self::get_for_update_cf_opt
    pub fn get_for_update<K: AsRef<[u8]>>(
        &self,
        key: K,
        exclusive: bool,
    ) -> Result<Option<Vec<u8>>, Error> {
        self.get_for_update_opt(key, exclusive, &ReadOptions::default())
    }

    pub fn get_pinned_for_update<K: AsRef<[u8]>>(
        &self,
        key: K,
        exclusive: bool,
    ) -> Result<Option<DBPinnableSlice>, Error> {
        self.get_pinned_for_update_opt(key, exclusive, &ReadOptions::default())
    }

    /// Get the key in the given column family and ensure that this transaction will only
    /// be able to be committed if this key is not written outside this
    /// transaction after it has first been read (or after the snapshot if a
    /// snapshot is set in this transaction).
    ///
    /// See [`get_for_update_cf_opt`] for details.
    ///
    /// [`get_for_update_cf_opt`]: Self::get_for_update_cf_opt
    pub fn get_for_update_cf<K: AsRef<[u8]>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        key: K,
        exclusive: bool,
    ) -> Result<Option<Vec<u8>>, Error> {
        self.get_for_update_cf_opt(cf, key, exclusive, &ReadOptions::default())
    }

    pub fn get_pinned_for_update_cf<K: AsRef<[u8]>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        key: K,
        exclusive: bool,
    ) -> Result<Option<DBPinnableSlice>, Error> {
        self.get_pinned_for_update_cf_opt(cf, key, exclusive, &ReadOptions::default())
    }

    /// Returns the bytes associated with a key value with read options.
    ///
    /// See [`get_cf_opt`] for details.
    ///
    /// [`get_cf_opt`]: Self::get_cf_opt
    pub fn get_opt<K: AsRef<[u8]>>(
        &self,
        key: K,
        readopts: &ReadOptions,
    ) -> Result<Option<Vec<u8>>, Error> {
        self.get_pinned_opt(key, readopts)
            .map(|x| x.map(|v| v.as_ref().to_vec()))
    }

    pub fn get_pinned_opt<K: AsRef<[u8]>>(
        &self,
        key: K,
        readopts: &ReadOptions,
    ) -> Result<Option<DBPinnableSlice>, Error> {
        unsafe {
            let val = ffi_try!(ffi::rocksdb_transaction_get_pinned(
                self.inner,
                readopts.inner,
                key.as_ref().as_ptr() as *const c_char,
                key.as_ref().len(),
            ));
            if val.is_null() {
                Ok(None)
            } else {
                Ok(Some(DBPinnableSlice::from_c(val)))
            }
        }
    }

    /// Get the bytes associated with a key value and the given column family with read options.
    ///
    /// This function will also read pending changes in this transaction.
    /// Currently, this function will return an error of the [`MergeInProgress`] kind
    /// if the most recent write to the queried key in this batch is a Merge.
    ///
    /// [`MergeInProgress`]: crate::ErrorKind::MergeInProgress
    pub fn get_cf_opt<K: AsRef<[u8]>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        key: K,
        readopts: &ReadOptions,
    ) -> Result<Option<Vec<u8>>, Error> {
        self.get_pinned_cf_opt(cf, key, readopts)
            .map(|x| x.map(|v| v.as_ref().to_vec()))
    }

    pub fn get_pinned_cf_opt<K: AsRef<[u8]>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        key: K,
        readopts: &ReadOptions,
    ) -> Result<Option<DBPinnableSlice>, Error> {
        unsafe {
            let val = ffi_try!(ffi::rocksdb_transaction_get_pinned_cf(
                self.inner,
                readopts.inner,
                cf.inner(),
                key.as_ref().as_ptr() as *const c_char,
                key.as_ref().len(),
            ));
            if val.is_null() {
                Ok(None)
            } else {
                Ok(Some(DBPinnableSlice::from_c(val)))
            }
        }
    }

    /// Get the key with read options and ensure that this transaction will only
    /// be able to be committed if this key is not written outside this
    /// transaction after it has first been read (or after the snapshot if a
    /// snapshot is set in this transaction).
    ///
    /// See [`get_for_update_cf_opt`] for details.
    ///
    /// [`get_for_update_cf_opt`]: Self::get_for_update_cf_opt
    pub fn get_for_update_opt<K: AsRef<[u8]>>(
        &self,
        key: K,
        exclusive: bool,
        opts: &ReadOptions,
    ) -> Result<Option<Vec<u8>>, Error> {
        self.get_pinned_for_update_opt(key, exclusive, opts)
            .map(|x| x.map(|v| v.as_ref().to_vec()))
    }

    pub fn get_pinned_for_update_opt<K: AsRef<[u8]>>(
        &self,
        key: K,
        exclusive: bool,
        opts: &ReadOptions,
    ) -> Result<Option<DBPinnableSlice>, Error> {
        unsafe {
            let val = ffi_try!(ffi::rocksdb_transaction_get_pinned_for_update(
                self.inner,
                opts.inner,
                key.as_ref().as_ptr() as *const c_char,
                key.as_ref().len() as size_t,
                u8::from(exclusive),
            ));
            if val.is_null() {
                Ok(None)
            } else {
                Ok(Some(DBPinnableSlice::from_c(val)))
            }
        }
    }

    /// Get the key in the given column family with read options
    /// and ensure that this transaction will only
    /// be able to be committed if this key is not written outside this
    /// transaction after it has first been read (or after the snapshot if a
    /// snapshot is set in this transaction).
    ///
    /// Currently, this function will return an error of the [`MergeInProgress`]
    /// if the most recent write to the queried key in this batch is a Merge.
    ///
    /// If this transaction was created by a [`TransactionDB`], it can return error of kind:
    /// * [`Busy`] if there is a write conflict.
    /// * [`TimedOut`] if a lock could not be acquired.
    /// * [`TryAgain`] if the memtable history size is not large enough.
    /// * [`MergeInProgress`] if merge operations cannot be resolved.
    /// * or other errors if this key could not be read.
    ///
    /// If this transaction was created by an `[OptimisticTransactionDB]`, `get_for_update_opt`
    /// can cause [`commit`] to fail. Otherwise, it could return any error that could
    /// be returned by `[DB::get]`.
    ///
    /// [`Busy`]: crate::ErrorKind::Busy
    /// [`TimedOut`]: crate::ErrorKind::TimedOut
    /// [`TryAgain`]: crate::ErrorKind::TryAgain
    /// [`MergeInProgress`]: crate::ErrorKind::MergeInProgress
    /// [`TransactionDB`]: crate::TransactionDB
    /// [`OptimisticTransactionDB`]: crate::OptimisticTransactionDB
    /// [`commit`]: Self::commit
    /// [`DB::get`]: crate::DB::get
    pub fn get_for_update_cf_opt<K: AsRef<[u8]>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        key: K,
        exclusive: bool,
        opts: &ReadOptions,
    ) -> Result<Option<Vec<u8>>, Error> {
        self.get_pinned_for_update_cf_opt(cf, key, exclusive, opts)
            .map(|x| x.map(|v| v.as_ref().to_vec()))
    }

    pub fn get_pinned_for_update_cf_opt<K: AsRef<[u8]>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        key: K,
        exclusive: bool,
        opts: &ReadOptions,
    ) -> Result<Option<DBPinnableSlice>, Error> {
        unsafe {
            let val = ffi_try!(ffi::rocksdb_transaction_get_pinned_for_update_cf(
                self.inner,
                opts.inner,
                cf.inner(),
                key.as_ref().as_ptr() as *const c_char,
                key.as_ref().len() as size_t,
                u8::from(exclusive),
            ));
            if val.is_null() {
                Ok(None)
            } else {
                Ok(Some(DBPinnableSlice::from_c(val)))
            }
        }
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
            ffi::rocksdb_transaction_multi_get(
                self.inner,
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
            ffi::rocksdb_transaction_multi_get_cf(
                self.inner,
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

    /// Put the key value in default column family and do conflict checking on the key.
    ///
    /// See [`put_cf`] for details.
    ///
    /// [`put_cf`]: Self::put_cf
    pub fn put<K: AsRef<[u8]>, V: AsRef<[u8]>>(&self, key: K, value: V) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_transaction_put(
                self.inner,
                key.as_ref().as_ptr() as *const c_char,
                key.as_ref().len() as size_t,
                value.as_ref().as_ptr() as *const c_char,
                value.as_ref().len() as size_t,
            ));
            Ok(())
        }
    }

    /// Put the key value in the given column famuly and do conflict checking on the key.
    ///
    /// If this transaction was created by a [`TransactionDB`], it can return error of kind:
    /// * [`Busy`] if there is a write conflict.
    /// * [`TimedOut`] if a lock could not be acquired.
    /// * [`TryAgain`] if the memtable history size is not large enough.
    /// * [`MergeInProgress`] if merge operations cannot be resolved.
    /// * or other errors on unexpected failures.
    ///
    /// [`Busy`]: crate::ErrorKind::Busy
    /// [`TimedOut`]: crate::ErrorKind::TimedOut
    /// [`TryAgain`]: crate::ErrorKind::TryAgain
    /// [`MergeInProgress`]: crate::ErrorKind::MergeInProgress
    /// [`TransactionDB`]: crate::TransactionDB
    /// [`OptimisticTransactionDB`]: crate::OptimisticTransactionDB
    pub fn put_cf<K: AsRef<[u8]>, V: AsRef<[u8]>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        key: K,
        value: V,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_transaction_put_cf(
                self.inner,
                cf.inner(),
                key.as_ref().as_ptr() as *const c_char,
                key.as_ref().len() as size_t,
                value.as_ref().as_ptr() as *const c_char,
                value.as_ref().len() as size_t,
            ));
            Ok(())
        }
    }

    /// Merge value with existing value of key, and also do conflict checking on the key.
    ///
    /// See [`merge_cf`] for details.
    ///
    /// [`merge_cf`]: Self::merge_cf
    pub fn merge<K: AsRef<[u8]>, V: AsRef<[u8]>>(&self, key: K, value: V) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_transaction_merge(
                self.inner,
                key.as_ref().as_ptr() as *const c_char,
                key.as_ref().len() as size_t,
                value.as_ref().as_ptr() as *const c_char,
                value.as_ref().len() as size_t
            ));
            Ok(())
        }
    }

    /// Merge `value` with existing value of `key` in the given column family,
    /// and also do conflict checking on the key.
    ///
    /// If this transaction was created by a [`TransactionDB`], it can return error of kind:
    /// * [`Busy`] if there is a write conflict.
    /// * [`TimedOut`] if a lock could not be acquired.
    /// * [`TryAgain`] if the memtable history size is not large enough.
    /// * [`MergeInProgress`] if merge operations cannot be resolved.
    /// * or other errors on unexpected failures.
    ///
    /// [`Busy`]: crate::ErrorKind::Busy
    /// [`TimedOut`]: crate::ErrorKind::TimedOut
    /// [`TryAgain`]: crate::ErrorKind::TryAgain
    /// [`MergeInProgress`]: crate::ErrorKind::MergeInProgress
    /// [`TransactionDB`]: crate::TransactionDB
    pub fn merge_cf<K: AsRef<[u8]>, V: AsRef<[u8]>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        key: K,
        value: V,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_transaction_merge_cf(
                self.inner,
                cf.inner(),
                key.as_ref().as_ptr() as *const c_char,
                key.as_ref().len() as size_t,
                value.as_ref().as_ptr() as *const c_char,
                value.as_ref().len() as size_t
            ));
            Ok(())
        }
    }

    /// Delete the key value if it exists and do conflict checking on the key.
    ///
    /// See [`delete_cf`] for details.
    ///
    /// [`delete_cf`]: Self::delete_cf
    pub fn delete<K: AsRef<[u8]>>(&self, key: K) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_transaction_delete(
                self.inner,
                key.as_ref().as_ptr() as *const c_char,
                key.as_ref().len() as size_t
            ));
        }
        Ok(())
    }

    /// Delete the key value in the given column family and do conflict checking.
    ///
    /// If this transaction was created by a [`TransactionDB`], it can return error of kind:
    /// * [`Busy`] if there is a write conflict.
    /// * [`TimedOut`] if a lock could not be acquired.
    /// * [`TryAgain`] if the memtable history size is not large enough.
    /// * [`MergeInProgress`] if merge operations cannot be resolved.
    /// * or other errors on unexpected failures.
    ///
    /// [`Busy`]: crate::ErrorKind::Busy
    /// [`TimedOut`]: crate::ErrorKind::TimedOut
    /// [`TryAgain`]: crate::ErrorKind::TryAgain
    /// [`MergeInProgress`]: crate::ErrorKind::MergeInProgress
    /// [`TransactionDB`]: crate::TransactionDB
    pub fn delete_cf<K: AsRef<[u8]>>(
        &self,
        cf: &impl AsColumnFamilyRef,
        key: K,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_transaction_delete_cf(
                self.inner,
                cf.inner(),
                key.as_ref().as_ptr() as *const c_char,
                key.as_ref().len() as size_t
            ));
        }
        Ok(())
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
    /// This is used when you want to iterate over a specific ColumnFamily with a modified ReadOptions.
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

    pub fn get_writebatch(&self) -> WriteBatchWithTransaction<true> {
        unsafe {
            let wi = ffi::rocksdb_transaction_get_writebatch_wi(self.inner);
            let mut len: usize = 0;
            let ptr = ffi::rocksdb_writebatch_wi_data(wi, &mut len as _);
            let writebatch = ffi::rocksdb_writebatch_create_from(ptr, len);
            ffi::rocksdb_free(wi as *mut c_void);
            WriteBatchWithTransaction { inner: writebatch }
        }
    }

    pub fn rebuild_from_writebatch(
        &self,
        writebatch: &WriteBatchWithTransaction<true>,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_transaction_rebuild_from_writebatch(
                self.inner,
                writebatch.inner
            ));
        }
        Ok(())
    }
}

impl<'db, DB> Drop for Transaction<'db, DB> {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_transaction_destroy(self.inner);
        }
    }
}
