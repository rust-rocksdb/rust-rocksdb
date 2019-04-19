use crate::{
    handle::{ConstHandle, Handle},
    ops::*,
    ColumnFamily, DBRawIterator, DBVector, Error, ReadOptions, ReadOptionsFactory,
};
use ffi;
use libc::{c_char, c_void, size_t};
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

    /// Delete a key inside a transaction
    ///
    /// ColumnFamilyHandle: default
    pub fn delete<K: AsRef<[u8]>>(&self, key: K) -> Result<(), Error> {
        let key = key.as_ref();
        unsafe {
            ffi_try!(ffi::rocksdb_transaction_delete(
                self.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
            ));
            Ok(())
        }
    }

    /// Delete a key inside a transaction
    pub fn delete_cf<K: AsRef<[u8]>>(&self, cf: &ColumnFamily, key: K) -> Result<(), Error> {
        let key = key.as_ref();
        unsafe {
            ffi_try!(ffi::rocksdb_transaction_delete_cf(
                self.inner,
                cf.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
            ));
            Ok(())
        }
    }

    /// Insert a value into the database under the given key.
    ///
    /// ColumnFamilyHandle: default
    pub fn put<K, V>(&self, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let key = key.as_ref();
        let value = value.as_ref();
        unsafe {
            ffi_try!(ffi::rocksdb_transaction_put(
                self.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
                value.as_ptr() as *const c_char,
                value.len() as size_t,
            ));
            Ok(())
        }
    }

    /// Insert a value into the database under the given key.
    pub fn put_cf<K, V>(&self, cf: &ColumnFamily, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let key = key.as_ref();
        let value = value.as_ref();
        unsafe {
            ffi_try!(ffi::rocksdb_transaction_put_cf(
                self.inner,
                cf.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
                value.as_ptr() as *const c_char,
                value.len() as size_t,
            ));
            Ok(())
        }
    }

    /// Merge a key inside a transaction
    pub fn merge<K, V>(&self, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let key = key.as_ref();
        let value = value.as_ref();
        unsafe {
            ffi_try!(ffi::rocksdb_transaction_merge(
                self.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
                value.as_ptr() as *const c_char,
                value.len() as size_t,
            ));
            Ok(())
        }
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

    pub fn snapshot(&'a self) -> TransactionSnapshot<'a, T> {
        unsafe {
            let snapshot = ffi::rocksdb_transaction_get_snapshot(self.inner);
            TransactionSnapshot {
                inner: snapshot,
                db: self,
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

        if readopts.is_none() {
            default_readopts.replace(ReadOptions::default());
        }

        let ro_handle = readopts
            .or_else(|| default_readopts.as_ref())
            .map(|r| r.inner)
            .ok_or_else(|| Error::new("Unable to extract read options.".to_string()))?;

        if ro_handle.is_null() {
            return Err(Error::new(
                "Unable to create RocksDB read options. \
                 This is a fairly trivial call, and its \
                 failure may be indicative of a \
                 mis-compiled or mis-loaded RocksDB \
                 library."
                    .to_string(),
            ));
        }

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
                inner: ffi::rocksdb_transaction_create_iterator(self.inner, readopts.inner),
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
                    readopts.inner,
                    cf_handle.inner,
                ),
                db: PhantomData,
            })
        }
    }
}

pub struct TransactionSnapshot<'a, T> {
    db: &'a Transaction<'a, T>,
    pub(crate) inner: *const ffi::rocksdb_snapshot_t,
}

impl<'a, T> ConstHandle<ffi::rocksdb_snapshot_t> for TransactionSnapshot<'a, T> {
    fn const_handle(&self) -> *const ffi::rocksdb_snapshot_t {
        self.inner
    }
}

impl<'a, T> Read for TransactionSnapshot<'a, T> {}

impl<'a, T> GetCF<ReadOptionsFactory> for TransactionSnapshot<'a, T>
where
    Transaction<'a, T>: GetCF<ReadOptions>,
{
    fn get_cf_full<K: AsRef<[u8]>>(
        &self,
        cf: Option<&ColumnFamily>,
        key: K,
        readopts: Option<&ReadOptionsFactory>,
    ) -> Result<Option<DBVector>, Error> {
        let mut ro = if let Some(rof) = readopts {
            rof.build()
        } else {
            ReadOptions::default()
        };
        ro.set_snapshot(self.inner);
        self.db.get_cf_full(cf, key, Some(&ro))
    }
}

impl<'a, T> Drop for TransactionSnapshot<'a, T> {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_free(self.inner as *mut c_void);
        }
    }
}

impl<'a, T: Iterate> SnapshotIterate for TransactionSnapshot<'a, T> {
    fn get_raw_iter(&self, readopts: &ReadOptionsFactory) -> DBRawIterator {
        let mut readopts = readopts.build();
        readopts.set_snapshot(self.inner);
        self.db.get_raw_iter(&readopts)
    }
}

impl<'a, T: IterateCF> SnapshotIterateCF for TransactionSnapshot<'a, T> {
    fn get_raw_iter_cf(
        &self,
        cf_handle: &ColumnFamily,
        readopts: &ReadOptionsFactory,
    ) -> Result<DBRawIterator, Error> {
        let mut readopts = readopts.build();
        readopts.set_snapshot(self.inner);
        self.db.get_raw_iter_cf(cf_handle, &readopts)
    }
}
