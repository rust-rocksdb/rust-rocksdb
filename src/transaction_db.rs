use crate::ops::*;
use crate::{
    ColumnFamily, ColumnFamilyDescriptor, DBRawIterator, DBVector, Error, OptimisticTransactionDB,
    Options, ReadOptions, Transaction, WriteOptions, DB,open_raw::{OpenRawFFI,OpenRaw},handle::Handle
};
use ffi;
use libc::{c_int, c_uchar};
use std::collections::BTreeMap;
use std::ffi::CString;
use std::fs;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::ptr;
use std::str;
use std::sync::{Arc, RwLock};

pub struct TransactionDB {
    inner: *mut ffi::rocksdb_transactiondb_t,
    cfs: BTreeMap<String, ColumnFamily>,
    path: PathBuf,
}


impl Handle<ffi::rocksdb_transactiondb_t> for TransactionDB {
    fn handle(&self) -> *mut ffi::rocksdb_transactiondb_t {
        self.inner
    }
}

impl Open for TransactionDB {}

impl OpenRaw for TransactionDB {
    type Pointer = ffi::rocksdb_transactiondb_t;
    type Descriptor = TransactionDBOptions;

    fn open_ffi(input: OpenRawFFI<'_, Self::Descriptor>) -> Result<*mut Self::Pointer, Error> {
        let pointer = unsafe {
            ffi_try!(ffi::rocksdb_transactiondb_open(
                input.options,
                input.open_descriptor.inner,
                input.path,
            ))
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
        let cfs: BTreeMap<_, _> = column_families.into_iter().map(|(k,h)| (k,ColumnFamily::new(h))).collect();
        Ok(TransactionDB {
            inner: pointer,
            cfs: cfs,
            path,
        })
    }
}

impl Read for TransactionDB {}
impl Write for TransactionDB {}

unsafe impl Send for TransactionDB {}
unsafe impl Sync for TransactionDB {}

impl TransactionBegin for TransactionDB {
    type WriteOptions = WriteOptions;
    type TransactionOptions = TransactionOptions;
    fn transaction(
        &self,
        write_options: &WriteOptions,
        tx_options: &TransactionOptions,
    ) -> Transaction<TransactionDB> {
        unsafe {
            let inner = ffi::rocksdb_transaction_begin(
                self.inner,
                write_options.inner,
                tx_options.inner,
                ptr::null_mut(),
            );
            Transaction::new(inner)
        }
    }
}

impl Iterate for TransactionDB {
    fn get_raw_iter(&self, readopts: &ReadOptions) -> DBRawIterator {
        unsafe {
            DBRawIterator {
                inner: ffi::rocksdb_transactiondb_create_iterator(self.inner, readopts.inner),
                db: PhantomData,
            }
        }
    }
}

impl Drop for TransactionDB {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_transactiondb_close(self.inner);
        }
    }
}

pub struct TransactionDBOptions {
    inner: *mut ffi::rocksdb_transactiondb_options_t,
}

impl TransactionDBOptions {
    /// Create new transaction options
    pub fn new() -> TransactionDBOptions {
        unsafe {
            let inner = ffi::rocksdb_transactiondb_options_create();
            TransactionDBOptions { inner }
        }
    }
}

impl Drop for TransactionDBOptions {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_transactiondb_options_destroy(self.inner);
        }
    }
}

impl Default for TransactionDBOptions {
    fn default() -> TransactionDBOptions {
        TransactionDBOptions::new()
    }
}

pub struct TransactionOptions {
    inner: *mut ffi::rocksdb_transaction_options_t,
}

impl TransactionOptions {
    /// Create new transaction options
    pub fn new() -> TransactionOptions {
        unsafe {
            let inner = ffi::rocksdb_transaction_options_create();
            TransactionOptions { inner }
        }
    }

    pub fn set_deadlock_detect(&self, deadlock_detect: bool) {
        unsafe {
            ffi::rocksdb_transaction_options_set_deadlock_detect(
                self.inner,
                deadlock_detect as c_uchar,
            )
        }
    }

    pub fn set_deadlock_detect_depth(&self, depth: i64) {
        unsafe { ffi::rocksdb_transaction_options_set_deadlock_detect_depth(self.inner, depth) }
    }

    pub fn set_expiration(&self, expiration: i64) {
        unsafe { ffi::rocksdb_transaction_options_set_expiration(self.inner, expiration) }
    }

    pub fn set_lock_timeout(&self, lock_timeout: i64) {
        unsafe { ffi::rocksdb_transaction_options_set_lock_timeout(self.inner, lock_timeout) }
    }

    pub fn set_max_write_batch_size(&self, size: usize) {
        unsafe { ffi::rocksdb_transaction_options_set_max_write_batch_size(self.inner, size) }
    }

    pub fn set_snapshot(&mut self, set_snapshot: bool) {
        unsafe {
            ffi::rocksdb_transaction_options_set_set_snapshot(self.inner, set_snapshot as c_uchar);
        }
    }
}

impl Drop for TransactionOptions {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_transaction_options_destroy(self.inner);
        }
    }
}

impl Default for TransactionOptions {
    fn default() -> TransactionOptions {
        TransactionOptions::new()
    }
}
