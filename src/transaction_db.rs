use crate::ops::*;
use crate::{
    db_vector::DBVector,
    handle::{ConstHandle, Handle},
    open_raw::{OpenRaw, OpenRawFFI},
    write_batch::WriteBatch,
    DBRawIterator, Error, ReadOptions, Transaction, WriteOptions,
};
use ffi;
use libc::{c_char, c_uchar, size_t};
use std::marker::PhantomData;
use std::path::Path;
use std::path::PathBuf;
use std::ptr;

/// A transaction database.
pub struct TransactionDB {
    inner: *mut ffi::rocksdb_transactiondb_t,
    path: PathBuf,
}

impl TransactionDB {
    pub fn path(&self) -> &Path {
        &self.path.as_path()
    }
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
        _column_families: I,
    ) -> Result<Self, Error>
    where
        I: IntoIterator<Item = (String, *mut ffi::rocksdb_column_family_handle_t)>,
    {
        Ok(TransactionDB {
            inner: pointer,
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
                write_options.handle(),
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
                inner: ffi::rocksdb_transactiondb_create_iterator(self.inner, readopts.handle()),
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

impl CreateCheckpointObject for TransactionDB {
    unsafe fn create_checkpoint_object_raw(&self) -> Result<*mut ffi::rocksdb_checkpoint_t, Error> {
        Ok(ffi_try!(
            ffi::rocksdb_transactiondb_checkpoint_object_create(self.inner,)
        ))
    }
}

impl Get<ReadOptions> for TransactionDB {
    fn get_full<K: AsRef<[u8]>>(
        &self,
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

            let val = ffi_try!(ffi::rocksdb_transactiondb_get(
                self.inner,
                ro_handle,
                key_ptr,
                key_len,
                &mut val_len,
            )) as *mut u8;

            if val.is_null() {
                Ok(None)
            } else {
                Ok(Some(DBVector::from_c(val, val_len)))
            }
        }
    }
}

impl Put<WriteOptions> for TransactionDB {
    fn put_full<K: AsRef<[u8]>, V: AsRef<[u8]>>(
        &self,
        key: K,
        value: V,
        writeopts: Option<&WriteOptions>,
    ) -> Result<(), Error> {
        let mut default_writeopts = None;

        let wo_handle = WriteOptions::input_or_default(writeopts, &mut default_writeopts)?;

        let key = key.as_ref();
        let value = value.as_ref();
        let key_ptr = key.as_ptr() as *const c_char;
        let key_len = key.len() as size_t;
        let val_ptr = value.as_ptr() as *const c_char;
        let val_len = value.len() as size_t;

        unsafe {
            ffi_try!(ffi::rocksdb_transactiondb_put(
                self.inner, wo_handle, key_ptr, key_len, val_ptr, val_len,
            ));
            Ok(())
        }
    }
}

impl Delete<WriteOptions> for TransactionDB {
    fn delete_full<K: AsRef<[u8]>>(
        &self,
        key: K,
        writeopts: Option<&WriteOptions>,
    ) -> Result<(), Error> {
        let mut default_writeopts = None;

        let wo_handle = WriteOptions::input_or_default(writeopts, &mut default_writeopts)?;

        let key = key.as_ref();
        let key_ptr = key.as_ptr() as *const c_char;
        let key_len = key.len() as size_t;

        unsafe {
            ffi_try!(ffi::rocksdb_transactiondb_delete(
                self.inner, wo_handle, key_ptr, key_len,
            ));

            Ok(())
        }
    }
}

impl Merge<WriteOptions> for TransactionDB {
    fn merge_full<K, V>(
        &self,
        key: K,
        value: V,
        writeopts: Option<&WriteOptions>,
    ) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let mut default_writeopts = None;

        let wo_handle = WriteOptions::input_or_default(writeopts, &mut default_writeopts)?;

        let key = key.as_ref();
        let value = value.as_ref();
        let key_ptr = key.as_ptr() as *const c_char;
        let key_len = key.len() as size_t;
        let val_ptr = value.as_ptr() as *const c_char;
        let val_len = value.len() as size_t;

        unsafe {
            ffi_try!(ffi::rocksdb_transactiondb_merge(
                self.inner, wo_handle, key_ptr, key_len, val_ptr, val_len,
            ));
            Ok(())
        }
    }
}

impl TransactionDB {
    pub fn snapshot(&self) -> Snapshot {
        let snapshot = unsafe { ffi::rocksdb_transactiondb_create_snapshot(self.inner) };
        Snapshot {
            db: self,
            inner: snapshot,
        }
    }
}

pub struct Snapshot<'a> {
    db: &'a TransactionDB,
    inner: *const ffi::rocksdb_snapshot_t,
}

impl<'a> ConstHandle<ffi::rocksdb_snapshot_t> for Snapshot<'a> {
    fn const_handle(&self) -> *const ffi::rocksdb_snapshot_t {
        self.inner
    }
}

impl<'a> Read for Snapshot<'a> {}

impl<'a> Get<ReadOptions> for Snapshot<'a> {
    fn get_full<K: AsRef<[u8]>>(
        &self,
        key: K,
        readopts: Option<&ReadOptions>,
    ) -> Result<Option<DBVector>, Error> {
        let mut ro = readopts.cloned().unwrap_or_default();
        ro.set_snapshot(self);

        self.db.get_full(key, Some(&ro))
    }
}

impl<'a> Drop for Snapshot<'a> {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_transactiondb_release_snapshot(self.db.inner, self.inner);
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

impl WriteOps for TransactionDB {
    fn write_full(&self, batch: WriteBatch, writeopts: Option<&WriteOptions>) -> Result<(), Error> {
        let mut default_writeopts = None;

        let wo_handle = WriteOptions::input_or_default(writeopts, &mut default_writeopts)?;

        unsafe {
            ffi_try!(ffi::rocksdb_transactiondb_write(
                self.handle(),
                wo_handle,
                batch.inner,
            ));
            Ok(())
        }
    }
}
