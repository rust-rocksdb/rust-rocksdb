use crate::{
    ColumnFamily, CreateIter, DBRawIterator, DBVector, Error, OptimisticTransactionDB, ReadOptions,
};
use ffi;
use libc::{c_char, size_t};
use std::marker::PhantomData;

pub struct Transaction<'a> {
    pub(crate) inner: *mut ffi::rocksdb_transaction_t,
    pub(crate) db: PhantomData<&'a OptimisticTransactionDB>,
}

impl<'a> Transaction<'a> {
    pub fn new(inner: *mut ffi::rocksdb_transaction_t) -> Transaction<'a> {
        Transaction {
            inner,
            db: PhantomData,
        }
    }

    pub fn commit(&self) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_transaction_commit(self.inner,));
        }
        Ok(())
    }

    pub fn delete(&self, key: &[u8]) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_transaction_delete(
                self.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
            ));
            Ok(())
        }
    }

    pub fn delete_cf(&self, cf: ColumnFamily, key: &[u8]) -> Result<(), Error> {
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
            let val = ffi_try!(ffi::rocksdb_transaction_get(
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
            let val = ffi_try!(ffi::rocksdb_transaction_get_cf(
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

    /// Insert a value into the database under the given key.
    pub fn put(&self, key: &[u8], value: &[u8]) -> Result<(), Error> {
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

    pub fn put_cf(&self, cf: ColumnFamily, key: &[u8], value: &[u8]) -> Result<(), Error> {
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

    pub fn merge(&self, key: &[u8], value: &[u8]) -> Result<(), Error> {
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

    pub fn rollback(&self) -> Result<(), Error> {
        unsafe { ffi_try!(ffi::rocksdb_transaction_rollback(self.inner,)) }
        Ok(())
    }

    pub fn rollback_to_savepoint(&self) -> Result<(), Error> {
        unsafe { ffi_try!(ffi::rocksdb_transaction_rollback_to_savepoint(self.inner,)) }
        Ok(())
    }

    pub fn set_savepoint(&self) {
        unsafe { ffi::rocksdb_transaction_set_savepoint(self.inner) }
    }
}

impl<'a> Drop for Transaction<'a> {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_transaction_destroy(self.inner);
        }
    }
}

impl<'a> CreateIter for Transaction<'a> {
    fn raw_iter(&self, readopts: &ReadOptions) -> DBRawIterator {
        unsafe {
            DBRawIterator {
                inner: ffi::rocksdb_transaction_create_iterator(self.inner, readopts.inner),
                db: PhantomData,
            }
        }
    }

    fn raw_iter_cf(
        &self,
        cf_handle: ColumnFamily,
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
