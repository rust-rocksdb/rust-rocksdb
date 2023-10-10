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

use crate::ffi;

pub struct TransactionOptions {
    pub(crate) inner: *mut ffi::rocksdb_transaction_options_t,
}

unsafe impl Send for TransactionOptions {}
unsafe impl Sync for TransactionOptions {}

impl Default for TransactionOptions {
    fn default() -> Self {
        let txn_opts = unsafe { ffi::rocksdb_transaction_options_create() };
        assert!(
            !txn_opts.is_null(),
            "Could not create RocksDB transaction options"
        );
        Self { inner: txn_opts }
    }
}

impl TransactionOptions {
    pub fn new() -> TransactionOptions {
        TransactionOptions::default()
    }

    pub fn set_skip_prepare(&mut self, skip_prepare: bool) {
        unsafe {
            ffi::rocksdb_transaction_options_set_set_snapshot(self.inner, u8::from(skip_prepare));
        }
    }

    /// Specifies use snapshot or not.
    ///
    /// Default: false.
    ///
    /// If a transaction has a snapshot set, the transaction will ensure that
    /// any keys successfully written(or fetched via `get_for_update`) have not
    /// been modified outside this transaction since the time the snapshot was
    /// set.
    /// If a snapshot has not been set, the transaction guarantees that keys have
    /// not been modified since the time each key was first written (or fetched via
    /// `get_for_update`).
    ///
    /// Using snapshot will provide stricter isolation guarantees at the
    /// expense of potentially more transaction failures due to conflicts with
    /// other writes.
    ///
    /// Calling `set_snapshot` will not affect the version of Data returned by `get`
    /// methods.
    pub fn set_snapshot(&mut self, snapshot: bool) {
        unsafe {
            ffi::rocksdb_transaction_options_set_set_snapshot(self.inner, u8::from(snapshot));
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
                u8::from(deadlock_detect),
            );
        }
    }

    /// Specifies the wait timeout in milliseconds when a transaction attempts to lock a key.
    ///
    /// If 0, no waiting is done if a lock cannot instantly be acquired.
    /// If negative, transaction lock timeout in `TransactionDBOptions` will be used.
    ///
    /// Default: -1.
    pub fn set_lock_timeout(&mut self, lock_timeout: i64) {
        unsafe {
            ffi::rocksdb_transaction_options_set_lock_timeout(self.inner, lock_timeout);
        }
    }

    /// Specifies expiration duration in milliseconds.
    ///
    /// If non-negative, transactions that last longer than this many milliseconds will fail to commit.
    /// If not set, a forgotten transaction that is never committed, rolled back, or deleted
    /// will never relinquish any locks it holds.  This could prevent keys from being by other writers.
    ///
    /// Default: -1.
    pub fn set_expiration(&mut self, expiration: i64) {
        unsafe {
            ffi::rocksdb_transaction_options_set_expiration(self.inner, expiration);
        }
    }

    /// Specifies the number of traversals to make during deadlock detection.
    ///
    /// Default: 50.
    pub fn set_deadlock_detect_depth(&mut self, depth: i64) {
        unsafe {
            ffi::rocksdb_transaction_options_set_deadlock_detect_depth(self.inner, depth);
        }
    }

    /// Specifies the maximum number of bytes used for the write batch. 0 means no limit.
    ///
    /// Default: 0.
    pub fn set_max_write_batch_size(&mut self, size: usize) {
        unsafe {
            ffi::rocksdb_transaction_options_set_max_write_batch_size(self.inner, size);
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

pub struct TransactionDBOptions {
    pub(crate) inner: *mut ffi::rocksdb_transactiondb_options_t,
}

unsafe impl Send for TransactionDBOptions {}
unsafe impl Sync for TransactionDBOptions {}

impl Default for TransactionDBOptions {
    fn default() -> Self {
        let txn_db_opts = unsafe { ffi::rocksdb_transactiondb_options_create() };
        assert!(
            !txn_db_opts.is_null(),
            "Could not create RocksDB transaction_db options"
        );
        Self { inner: txn_db_opts }
    }
}

impl TransactionDBOptions {
    pub fn new() -> TransactionDBOptions {
        TransactionDBOptions::default()
    }

    /// Specifies the wait timeout in milliseconds when writing a key
    /// outside a transaction (i.e. by calling `TransactionDB::put` directly).
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

    /// Specifies the default wait timeout in milliseconds when a transaction
    /// attempts to lock a key if not specified in `TransactionOptions`.
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
        unsafe {
            ffi::rocksdb_transactiondb_options_set_max_num_locks(self.inner, max_num_locks);
        }
    }

    /// Specifies lock table stripes count.
    ///
    /// Increasing this value will increase the concurrency by dividing the lock
    /// table (per column family) into more sub-tables, each with their own
    /// separate mutex.
    ///
    /// Default: 16.
    pub fn set_num_stripes(&mut self, num_stripes: usize) {
        unsafe {
            ffi::rocksdb_transactiondb_options_set_num_stripes(self.inner, num_stripes);
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

pub struct OptimisticTransactionOptions {
    pub(crate) inner: *mut ffi::rocksdb_optimistictransaction_options_t,
}

unsafe impl Send for OptimisticTransactionOptions {}
unsafe impl Sync for OptimisticTransactionOptions {}

impl Default for OptimisticTransactionOptions {
    fn default() -> Self {
        let txn_opts = unsafe { ffi::rocksdb_optimistictransaction_options_create() };
        assert!(
            !txn_opts.is_null(),
            "Could not create RocksDB optimistic transaction options"
        );
        Self { inner: txn_opts }
    }
}

impl OptimisticTransactionOptions {
    pub fn new() -> OptimisticTransactionOptions {
        OptimisticTransactionOptions::default()
    }

    /// Specifies use snapshot or not.
    ///
    /// Default: false.
    ///
    /// If a transaction has a snapshot set, the transaction will ensure that
    /// any keys successfully written(or fetched via `get_for_update`) have not
    /// been modified outside the transaction since the time the snapshot was
    /// set.
    /// If a snapshot has not been set, the transaction guarantees that keys have
    /// not been modified since the time each key was first written (or fetched via
    /// `get_for_update`).
    ///
    /// Using snapshot will provide stricter isolation guarantees at the
    /// expense of potentially more transaction failures due to conflicts with
    /// other writes.
    ///
    /// Calling `set_snapshot` will not affect the version of Data returned by `get`
    /// methods.
    pub fn set_snapshot(&mut self, snapshot: bool) {
        unsafe {
            ffi::rocksdb_optimistictransaction_options_set_set_snapshot(
                self.inner,
                u8::from(snapshot),
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
