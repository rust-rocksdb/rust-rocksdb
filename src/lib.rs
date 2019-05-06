// Copyright 2014 Tyler Neely
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

//! Rust wrapper for RocksDB.
//!
//! # Examples
//!
//! ```
//! use rocksdb::prelude::*;
//! # use rocksdb::TemporaryDBPath;
//! // NB: db is automatically closed at end of lifetime
//!
//! let path = "_path_for_rocksdb_storage";
//! # let path = TemporaryDBPath::new();
//! # {
//!
//! let db = DB::open_default(&path).unwrap();
//! db.put(b"my key", b"my value").unwrap();
//! match db.get(b"my key") {
//!     Ok(Some(value)) => println!("retrieved value {}", value.to_utf8().unwrap()),
//!     Ok(None) => println!("value not found"),
//!     Err(e) => println!("operational problem encountered: {}", e),
//! }
//! db.delete(b"my key").unwrap();

//! # }
//! ```
//!
//! Opening a database and a single column family with custom options:
//!
//! ```
//! use rocksdb::{prelude::*, ColumnFamilyDescriptor};
//! # use rocksdb::TemporaryDBPath;
//!
//! let path = "_path_for_rocksdb_storage_with_cfs";
//! # let path = TemporaryDBPath::new();
//!
//! let mut cf_opts = Options::default();
//! cf_opts.set_max_write_buffer_number(16);
//! let cf = ColumnFamilyDescriptor::new("cf1", cf_opts);
//!
//! let mut db_opts = Options::default();
//! db_opts.create_missing_column_families(true);
//! db_opts.create_if_missing(true);
//! # {
//! let db = DB::open_cf_descriptors(&db_opts, &path, vec![cf]).unwrap();
//! # }
//! ```
//!

extern crate libc;
extern crate librocksdb_sys as ffi;

#[macro_use]
mod ffi_util;
mod util;

pub mod backup;
pub mod checkpoint;
pub mod column_family;
pub mod compaction_filter;
mod comparator;
mod db;
mod db_iterator;
mod db_options;
mod db_pinnable_slice;
mod db_vector;
mod handle;
pub mod merge_operator;
mod open_raw;
pub mod ops;
mod optimistic_transaction_db;
mod read_only_db;
mod slice_transform;
mod snapshot;
mod transaction;
mod transaction_db;
mod write_batch;

pub mod prelude;

pub use column_family::ColumnFamilyDescriptor;
pub use compaction_filter::Decision as CompactionDecision;
pub use db::DB;
pub use db_iterator::{DBIterator, DBRawIterator, Direction, IteratorMode};
pub use db_options::{DBCompactionStyle, DBCompressionType, DBRecoveryMode, ReadOptions};
pub use db_pinnable_slice::DBPinnableSlice;
pub use db_vector::DBVector;
pub use read_only_db::ReadOnlyDB;
pub use snapshot::Snapshot;
pub use util::TemporaryDBPath;
pub use write_batch::WriteBatch;

pub use slice_transform::SliceTransform;

pub use merge_operator::MergeOperands;
use std::error;
use std::fmt;

pub use optimistic_transaction_db::{OptimisticTransactionDB, OptimisticTransactionOptions};
pub use transaction::Transaction;
pub use transaction_db::{TransactionDB, TransactionDBOptions, TransactionOptions};

/// A simple wrapper round a string, used for errors reported from
/// ffi calls.
#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    message: String,
}

impl Error {
    fn new(message: String) -> Error {
        Error { message }
    }

    pub fn into_string(self) -> String {
        self.into()
    }
}

impl AsRef<str> for Error {
    fn as_ref(&self) -> &str {
        &self.message
    }
}

impl From<Error> for String {
    fn from(e: Error) -> String {
        e.message
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.message.fmt(formatter)
    }
}

/// For configuring block-based file storage.
pub struct BlockBasedOptions {
    inner: *mut ffi::rocksdb_block_based_table_options_t,
}

/// Used by BlockBasedOptions::set_index_type.
pub enum BlockBasedIndexType {
    /// A space efficient index block that is optimized for
    /// binary-search-based index.
    BinarySearch,

    /// The hash index, if enabled, will perform a hash lookup if
    /// a prefix extractor has been provided through Options::set_prefix_extractor.
    HashSearch,

    /// A two-level index implementation. Both levels are binary search indexes.
    TwoLevelIndexSearch,
}

/// Defines the underlying memtable implementation.
/// See https://github.com/facebook/rocksdb/wiki/MemTable for more information.
pub enum MemtableFactory {
    Vector,
    HashSkipList {
        bucket_count: usize,
        height: i32,
        branching_factor: i32,
    },
    HashLinkList {
        bucket_count: usize,
    },
}

/// Used with DBOptions::set_plain_table_factory.
/// See https://github.com/facebook/rocksdb/wiki/PlainTable-Format.
///
/// Defaults:
///  user_key_length: 0 (variable length)
///  bloom_bits_per_key: 10
///  hash_table_ratio: 0.75
///  index_sparseness: 16
pub struct PlainTableFactoryOptions {
    pub user_key_length: u32,
    pub bloom_bits_per_key: i32,
    pub hash_table_ratio: f64,
    pub index_sparseness: usize,
}

/// Database-wide options around performance and behavior.
///
/// Please read [the official tuning guide](https://github.com/facebook/rocksdb/wiki/RocksDB-Tuning-Guide), and most importantly, measure performance under realistic workloads with realistic hardware.
///
/// # Examples
///
/// ```
/// use rocksdb::{Options, DB, prelude::*};
/// use rocksdb::DBCompactionStyle;
///
/// fn badly_tuned_for_somebody_elses_disk() -> DB {
///    let path = "path/for/rocksdb/storageX";
///    let mut opts = Options::default();
///    opts.create_if_missing(true);
///    opts.set_max_open_files(10000);
///    opts.set_use_fsync(false);
///    opts.set_bytes_per_sync(8388608);
///    opts.optimize_for_point_lookup(1024);
///    opts.set_table_cache_num_shard_bits(6);
///    opts.set_max_write_buffer_number(32);
///    opts.set_write_buffer_size(536870912);
///    opts.set_target_file_size_base(1073741824);
///    opts.set_min_write_buffer_number_to_merge(4);
///    opts.set_level_zero_stop_writes_trigger(2000);
///    opts.set_level_zero_slowdown_writes_trigger(0);
///    opts.set_compaction_style(DBCompactionStyle::Universal);
///    opts.set_max_background_compactions(4);
///    opts.set_max_background_flushes(4);
///    opts.set_disable_auto_compactions(true);
///
///    DB::open(&opts, path).unwrap()
/// }
/// ```
pub struct Options {
    inner: *mut ffi::rocksdb_options_t,
}

/// Optionally wait for the memtable flush to be performed.
///
/// # Examples
///
/// Manually flushing the memtable:
///
/// ```
/// use rocksdb::{DB, Options, FlushOptions, prelude::*};
///
/// let path = "_path_for_rocksdb_storageY";
/// {
///
///     let db = DB::open_default(path).unwrap();
///
///     let mut flush_options = FlushOptions::default();
///     flush_options.set_wait(true);
///
///     db.flush_opt(&flush_options);
/// }
///
/// let _ = DB::destroy(&Options::default(), path);
/// ```
pub struct FlushOptions {
    inner: *mut ffi::rocksdb_flushoptions_t,
}

/// Optionally disable WAL or sync for this write.
///
/// # Examples
///
/// Making an unsafe write of a batch:
///
/// ```
/// use rocksdb::{DB, Options, WriteBatch, WriteOptions, prelude::*};
///
/// let path = "_path_for_rocksdb_storageZ";
/// {
///     let db = DB::open_default(path).unwrap();
///     let mut batch = WriteBatch::default();
///     batch.put(b"my key", b"my value");
///     batch.put(b"key2", b"value2");
///     batch.put(b"key3", b"value3");
///
///     let mut write_options = WriteOptions::default();
///     write_options.set_sync(false);
///     write_options.disable_wal(true);
///
///     db.write_opt(batch, &write_options);
/// }
/// let _ = DB::destroy(&Options::default(), path);
/// ```
pub struct WriteOptions {
    option_set_sync: Option<bool>,
    option_disable_wal: Option<bool>,
    inner: *mut ffi::rocksdb_writeoptions_t,
}

/// An opaque type used to represent a column family. Returned from some functions, and used
/// in others
pub struct ColumnFamily {
    inner: *mut ffi::rocksdb_column_family_handle_t,
}

unsafe impl Send for ColumnFamily {}
