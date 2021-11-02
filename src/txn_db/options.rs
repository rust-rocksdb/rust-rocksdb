use core::panic;

use libc::c_uchar;

use crate::ffi;

pub struct TxnOptions {
    pub(crate) inner: *mut ffi::rocksdb_transaction_options_t,
}

unsafe impl Send for TxnOptions {}
unsafe impl Sync for TxnOptions {}

impl Default for TxnOptions {
    fn default() -> Self {
        let txn_opts = unsafe { ffi::rocksdb_transaction_options_create() };
        if txn_opts.is_null() {
            panic!("Could not create RocksDB transaction options");
        }
        Self { inner: txn_opts }
    }
}

impl TxnOptions {
    pub fn new() -> TxnOptions {
        TxnOptions::default()
    }

    pub fn set_snapshot(&mut self, snapshot: bool) {
        unsafe {
            ffi::rocksdb_transaction_options_set_set_snapshot(self.inner, snapshot as c_uchar);
        }
    }

    pub fn set_deadlock_detect(&mut self, deadlock_detect: bool) {
        unsafe {
            ffi::rocksdb_transaction_options_set_deadlock_detect(
                self.inner,
                deadlock_detect as c_uchar,
            );
        }
    }

    pub fn set_lock_timeout(&mut self, lock_timeout: i64) {
        unsafe { ffi::rocksdb_transaction_options_set_lock_timeout(self.inner, lock_timeout); }
    }

    pub fn set_expiration(&mut self, expiration: i64) {
        unsafe { ffi::rocksdb_transaction_options_set_expiration(self.inner, expiration); }
    }

    pub fn set_deadlock_detect_depth(&mut self, depth: i64) {
        unsafe { ffi::rocksdb_transaction_options_set_deadlock_detect_depth(self.inner, depth); }
    }

    pub fn set_max_write_batch_size(&mut self, size: usize) {
        unsafe { ffi::rocksdb_transaction_options_set_max_write_batch_size(self.inner, size); }
    }
}

impl Drop for TxnOptions {
    fn drop(&mut self) {
        unsafe { ffi::rocksdb_transaction_options_destroy(self.inner); }
    }
}

pub struct TxnDBOptions {
    pub(crate) inner: *mut ffi::rocksdb_transactiondb_options_t,
}

unsafe impl Send for TxnDBOptions {}
unsafe impl Sync for TxnDBOptions {}

impl Default for TxnDBOptions {
    fn default() -> Self {
        let txn_db_opts = unsafe { ffi::rocksdb_transactiondb_options_create() };
        if txn_db_opts.is_null() {
            panic!("Could not create RocksDB transactiondb options");
        }
        Self { inner: txn_db_opts }
    }
}

impl TxnDBOptions {
    pub fn new() -> TxnDBOptions {
        TxnDBOptions::default()
    }

    pub fn set_default_lock_timeout(&mut self, default_lock_timeout: i64) {
        unsafe {
            ffi::rocksdb_transactiondb_options_set_default_lock_timeout(
                self.inner,
                default_lock_timeout,
            );
        }
    }

    pub fn set_txn_lock_timeout(&mut self, txn_lock_timeout: i64) {
        unsafe {
            ffi::rocksdb_transactiondb_options_set_transaction_lock_timeout(
                self.inner,
                txn_lock_timeout,
            );
        }
    }

    pub fn set_max_num_locks(&mut self, max_num_locks: i64) {
        unsafe { ffi::rocksdb_transactiondb_options_set_max_num_locks(self.inner, max_num_locks); }
    }

    pub fn set_num_stripes(&mut self, num_stripes: usize) {
        unsafe { ffi::rocksdb_transactiondb_options_set_num_stripes(self.inner, num_stripes); }
    }
}

impl Drop for TxnDBOptions {
    fn drop(&mut self) {
        unsafe { ffi::rocksdb_transactiondb_options_destroy(self.inner); }
    }
}
