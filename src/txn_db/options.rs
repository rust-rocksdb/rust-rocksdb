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

    /// Setting `set_snapshot(true)` is the same as calling `Txn::set_snapshot()`.
    pub fn set_snapshot(&mut self, snapshot: bool) {
        unsafe {
            ffi::rocksdb_transaction_options_set_set_snapshot(self.inner, snapshot as c_uchar);
        }
    }

    /// Specifies whether detect deadlock or not.
    /// 
    /// Setting to true means that before acquiring locks, this transaction will
    /// check if doing so will cause a deadlock. If so, it will return with
    /// Status::Busy.  The user should retry their transaction.
    /// 
    /// Default: false.
    pub fn set_deadlock_detect(&mut self, deadlock_detect: bool) {
        unsafe {
            ffi::rocksdb_transaction_options_set_deadlock_detect(
                self.inner,
                deadlock_detect as c_uchar,
            );
        }
    }

    /// Specifies the wait timeout in milliseconds when a transaction attempts to lock a key.
    /// 
    /// If 0, no waiting is done if a lock cannot instantly be acquired.
    /// If negative, transaction lock timeout in `TxnDBOptions` will be used.
    /// 
    /// Default: -1.
    pub fn set_lock_timeout(&mut self, lock_timeout: i64) {
        unsafe { ffi::rocksdb_transaction_options_set_lock_timeout(self.inner, lock_timeout); }
    }

    /// Specifies expiration duration in milliseconds.
    /// 
    /// If non-negative, transactions that last longer than this many milliseconds will fail to commit.
    /// If not set, a forgotten transaction that is never committed, rolled back, or deleted
    /// will never relinquish any locks it holds.  This could prevent keys from being by other writers.
    /// 
    /// Default: -1.
    pub fn set_expiration(&mut self, expiration: i64) {
        unsafe { ffi::rocksdb_transaction_options_set_expiration(self.inner, expiration); }
    }

    /// Specifies the number of traversals to make during deadlock detection.
    /// 
    /// Default: 50.
    pub fn set_deadlock_detect_depth(&mut self, depth: i64) {
        unsafe { ffi::rocksdb_transaction_options_set_deadlock_detect_depth(self.inner, depth); }
    }

    /// Specifies the maximum number of bytes used for the write batch. 0 means no limit.
    /// 
    /// Default: 0.
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

    /// Specifies the wait timeout in milliseconds when writing a key
    /// outside of a transaction (ie. by calling `TxnDB::put` directly).
    /// 
    /// If 0, no waiting is done if a lock cannot instantly be acquired.
    /// If negative, there is no timeout and will block indefinitely when acquiring
    /// a lock.
    /// 
    /// Not using a timeout can lead to deadlocks.  Currently, there
    /// is no deadlock-detection to recover from a deadlock.  While DB writes
    /// cannot deadlock with other DB writes, they can deadlock with a transaction.
    /// A negative timeout should only be used if all transactions have a small
    /// expiration set.
    /// 
    /// Default: 1000(1s).
    pub fn set_default_lock_timeout(&mut self, default_lock_timeout: i64) {
        unsafe {
            ffi::rocksdb_transactiondb_options_set_default_lock_timeout(
                self.inner,
                default_lock_timeout,
            );
        }
    }

    /// Specifies the default wait timeout in milliseconds when a stransaction
    /// attempts to lock a key if not secified in `TxnOptions`. 
    /// 
    /// If 0, no waiting is done if a lock cannot instantly be acquired.
    /// If negative, there is no timeout.  Not using a timeout is not recommended
    /// as it can lead to deadlocks.  Currently, there is no deadlock-detection to
    /// recover from a deadlock.
    /// 
    /// Default: 1000(1s).
    pub fn set_txn_lock_timeout(&mut self, txn_lock_timeout: i64) {
        unsafe {
            ffi::rocksdb_transactiondb_options_set_transaction_lock_timeout(
                self.inner,
                txn_lock_timeout,
            );
        }
    }

    /// Specifies the maximum number of keys that can be locked at the same time
    /// per column family.
    /// 
    /// If the number of locked keys is greater than `max_num_locks`, transaction
    /// `writes` (or `get_for_update`) will return an error.
    /// If this value is not positive, no limit will be enforced.
    /// 
    /// Default: -1.
    pub fn set_max_num_locks(&mut self, max_num_locks: i64) {
        unsafe { ffi::rocksdb_transactiondb_options_set_max_num_locks(self.inner, max_num_locks); }
    }

    /// Sepcifies lock table stripes count.
    /// 
    /// Increasing this value will increase the concurrency by dividing the lock
    /// table (per column family) into more sub-tables, each with their own
    /// separate mutex.
    /// 
    /// Default: 16.
    pub fn set_num_stripes(&mut self, num_stripes: usize) {
        unsafe { ffi::rocksdb_transactiondb_options_set_num_stripes(self.inner, num_stripes); }
    }
}

impl Drop for TxnDBOptions {
    fn drop(&mut self) {
        unsafe { ffi::rocksdb_transactiondb_options_destroy(self.inner); }
    }
}
