use crate::{
    handle::{ConstHandle, Handle},
    ops::*,
    ColumnFamily, DBRawIterator, DBVector, Error, ReadOptions,
};
use ffi;
use libc::{c_char, c_uchar, c_void, size_t};
use std::marker::PhantomData;

pub struct Transaction<'a, T> {
    inner: *mut ffi::rocksdb_transaction_t,
    db: PhantomData<&'a T>,
}

impl<'a, T> Transaction<'a, T> {
    pub(crate) fn new(inner: *mut ffi::rocksdb_transaction_t) -> Transaction<'a, T> {
        Transaction {
            inner,
            db: PhantomData,
        }
    }

    /// commits a transaction
    pub fn commit(&self) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_transaction_commit(self.inner,));
        }
        Ok(())
    }

    /// Transaction rollback
    pub fn rollback(&self) -> Result<(), Error> {
        unsafe { ffi_try!(ffi::rocksdb_transaction_rollback(self.inner,)) }
        Ok(())
    }

    /// Transaction rollback to savepoint
    pub fn rollback_to_savepoint(&self) -> Result<(), Error> {
        unsafe { ffi_try!(ffi::rocksdb_transaction_rollback_to_savepoint(self.inner,)) }
        Ok(())
    }

    /// Set savepoint for transaction
    pub fn set_savepoint(&self) {
        unsafe { ffi::rocksdb_transaction_set_savepoint(self.inner) }
    }

    /// Get Snapshot
    pub fn snapshot(&'a self) -> TransactionSnapshot<'a, T> {
        unsafe {
            let snapshot = ffi::rocksdb_transaction_get_snapshot(self.inner);
            TransactionSnapshot {
                inner: snapshot,
                db: self,
            }
        }
    }

    /// Get For Update
    /// ReadOptions: Default
    /// exclusive: true
    pub fn get_for_update<K: AsRef<[u8]>>(&self, key: K) -> Result<Option<DBVector>, Error> {
        let opt = ReadOptions::default();
        self.get_for_update_opt(key, &opt, true)
    }

    /// Get For Update with custom ReadOptions and exclusive
    pub fn get_for_update_opt<K: AsRef<[u8]>>(
        &self,
        key: K,
        readopts: &ReadOptions,
        exclusive: bool,
    ) -> Result<Option<DBVector>, Error> {
        let key = key.as_ref();
        let key_ptr = key.as_ptr() as *const c_char;
        let key_len = key.len() as size_t;
        unsafe {
            let mut val_len: size_t = 0;
            let val = ffi_try!(ffi::rocksdb_transaction_get_for_update(
                self.handle(),
                readopts.handle(),
                key_ptr,
                key_len,
                &mut val_len,
                exclusive as c_uchar,
            )) as *mut u8;

            if val.is_null() {
                Ok(None)
            } else {
                Ok(Some(DBVector::from_c(val, val_len)))
            }
        }
    }

    pub fn get_for_update_cf<K: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        key: K,
    ) -> Result<Option<DBVector>, Error> {
        let opt = ReadOptions::default();
        self.get_for_update_cf_opt(cf, key, &opt, true)
    }

    pub fn get_for_update_cf_opt<K: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        key: K,
        readopts: &ReadOptions,
        exclusive: bool,
    ) -> Result<Option<DBVector>, Error> {
        let key = key.as_ref();
        let key_ptr = key.as_ptr() as *const c_char;
        let key_len = key.len() as size_t;
        unsafe {
            let mut val_len: size_t = 0;
            let val = ffi_try!(ffi::rocksdb_transaction_get_for_update_cf(
                self.handle(),
                readopts.handle(),
                cf.handle(),
                key_ptr,
                key_len,
                &mut val_len,
                exclusive as c_uchar,
            )) as *mut u8;

            if val.is_null() {
                Ok(None)
            } else {
                Ok(Some(DBVector::from_c(val, val_len)))
            }
        }
    }
}

impl<'a, T> Drop for Transaction<'a, T> {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_transaction_destroy(self.inner);
        }
    }
}

impl<'a, T> Handle<ffi::rocksdb_transaction_t> for Transaction<'a, T> {
    fn handle(&self) -> *mut ffi::rocksdb_transaction_t {
        self.inner
    }
}

impl<'a, T> Read for Transaction<'a, T> {}

impl<'a, T> GetCF<ReadOptions> for Transaction<'a, T>
where
    Transaction<'a, T>: Handle<ffi::rocksdb_transaction_t> + Read,
{
    fn get_cf_full<K: AsRef<[u8]>>(
        &self,
        cf: Option<&ColumnFamily>,
        key: K,
        readopts: Option<&ReadOptions>,
    ) -> Result<Option<DBVector>, Error> {
        let mut default_readopts = None;

        let ro_handle = ReadOptions::input_or_default(readopts, &mut default_readopts)?;

        let key = key.as_ref();
        let key_ptr = key.as_ptr() as *const c_char;
        let key_len = key.len() as size_t;

        unsafe {
            let mut val_len: size_t = 0;

            let val = match cf {
                Some(cf) => ffi_try!(ffi::rocksdb_transaction_get_cf(
                    self.handle(),
                    ro_handle,
                    cf.inner,
                    key_ptr,
                    key_len,
                    &mut val_len,
                )),
                None => ffi_try!(ffi::rocksdb_transaction_get(
                    self.handle(),
                    ro_handle,
                    key_ptr,
                    key_len,
                    &mut val_len,
                )),
            } as *mut u8;

            if val.is_null() {
                Ok(None)
            } else {
                Ok(Some(DBVector::from_c(val, val_len)))
            }
        }
    }
}
impl<'a, T> Iterate for Transaction<'a, T> {
    fn get_raw_iter(&self, readopts: &ReadOptions) -> DBRawIterator {
        unsafe {
            DBRawIterator {
                inner: ffi::rocksdb_transaction_create_iterator(self.inner, readopts.handle()),
                db: PhantomData,
            }
        }
    }
}

impl<'a, T> IterateCF for Transaction<'a, T> {
    fn get_raw_iter_cf(
        &self,
        cf_handle: &ColumnFamily,
        readopts: &ReadOptions,
    ) -> Result<DBRawIterator, Error> {
        unsafe {
            Ok(DBRawIterator {
                inner: ffi::rocksdb_transaction_create_iterator_cf(
                    self.inner,
                    readopts.handle(),
                    cf_handle.inner,
                ),
                db: PhantomData,
            })
        }
    }
}

pub struct TransactionSnapshot<'a, T> {
    db: &'a Transaction<'a, T>,
    inner: *const ffi::rocksdb_snapshot_t,
}

impl<'a, T> ConstHandle<ffi::rocksdb_snapshot_t> for TransactionSnapshot<'a, T> {
    fn const_handle(&self) -> *const ffi::rocksdb_snapshot_t {
        self.inner
    }
}

impl<'a, T> Read for TransactionSnapshot<'a, T> {}

impl<'a, T> GetCF<ReadOptions> for TransactionSnapshot<'a, T>
where
    Transaction<'a, T>: GetCF<ReadOptions>,
{
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

impl<'a, T> PutCF<()> for Transaction<'a, T> {
    fn put_cf_full<K, V>(
        &self,
        cf: Option<&ColumnFamily>,
        key: K,
        value: V,
        _: Option<&()>,
    ) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let key = key.as_ref();
        let value = value.as_ref();
        let key_ptr = key.as_ptr() as *const c_char;
        let key_len = key.len() as size_t;
        let val_ptr = value.as_ptr() as *const c_char;
        let val_len = value.len() as size_t;

        unsafe {
            match cf {
                Some(cf) => ffi_try!(ffi::rocksdb_transaction_put_cf(
                    self.handle(),
                    cf.handle(),
                    key_ptr,
                    key_len,
                    val_ptr,
                    val_len,
                )),
                None => ffi_try!(ffi::rocksdb_transaction_put(
                    self.handle(),
                    key_ptr,
                    key_len,
                    val_ptr,
                    val_len,
                )),
            }

            Ok(())
        }
    }
}

impl<'a, T> MergeCF<()> for Transaction<'a, T> {
    fn merge_cf_full<K, V>(
        &self,
        cf: Option<&ColumnFamily>,
        key: K,
        value: V,
        _: Option<&()>,
    ) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let key = key.as_ref();
        let value = value.as_ref();
        let key_ptr = key.as_ptr() as *const c_char;
        let key_len = key.len() as size_t;
        let val_ptr = value.as_ptr() as *const c_char;
        let val_len = value.len() as size_t;

        unsafe {
            match cf {
                Some(cf) => ffi_try!(ffi::rocksdb_transaction_merge_cf(
                    self.handle(),
                    cf.handle(),
                    key_ptr,
                    key_len,
                    val_ptr,
                    val_len,
                )),
                None => ffi_try!(ffi::rocksdb_transaction_merge(
                    self.handle(),
                    key_ptr,
                    key_len,
                    val_ptr,
                    val_len,
                )),
            }

            Ok(())
        }
    }
}

impl<'a, T> DeleteCF<()> for Transaction<'a, T> {
    fn delete_cf_full<K>(
        &self,
        cf: Option<&ColumnFamily>,
        key: K,
        _: Option<&()>,
    ) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
    {
        let key = key.as_ref();
        let key_ptr = key.as_ptr() as *const c_char;
        let key_len = key.len() as size_t;

        unsafe {
            match cf {
                Some(cf) => ffi_try!(ffi::rocksdb_transaction_delete_cf(
                    self.handle(),
                    cf.inner,
                    key_ptr,
                    key_len,
                )),
                None => ffi_try!(ffi::rocksdb_transaction_delete(
                    self.handle(),
                    key_ptr,
                    key_len,
                )),
            }

            Ok(())
        }
    }
}

impl<'a, T> Drop for TransactionSnapshot<'a, T> {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_free(self.inner as *mut c_void);
        }
    }
}

impl<'a, T: Iterate> Iterate for TransactionSnapshot<'a, T> {
    fn get_raw_iter(&self, readopts: &ReadOptions) -> DBRawIterator {
        let mut readopts = readopts.to_owned();
        readopts.set_snapshot(self);
        self.db.get_raw_iter(&readopts)
    }
}

impl<'a, T: IterateCF> IterateCF for TransactionSnapshot<'a, T> {
    fn get_raw_iter_cf(
        &self,
        cf_handle: &ColumnFamily,
        readopts: &ReadOptions,
    ) -> Result<DBRawIterator, Error> {
        let mut readopts = readopts.to_owned();
        readopts.set_snapshot(self);
        self.db.get_raw_iter_cf(cf_handle, &readopts)
    }
}
