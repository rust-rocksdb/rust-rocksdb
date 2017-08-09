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
//!  use rocksdb::DB;
//!  // NB: db is automatically closed at end of lifetime
//!  let db = DB::open_default("path/for/rocksdb/storage").unwrap();
//!  db.put(b"my key", b"my value");
//!  match db.get(b"my key") {
//!     Ok(Some(value)) => println!("retrieved value {}", value.to_utf8().unwrap()),
//!     Ok(None) => println!("value not found"),
//!     Err(e) => println!("operational problem encountered: {}", e),
//!  }
//!  db.delete(b"my key").unwrap();
//! ```
//!

extern crate libc;
extern crate librocksdb_sys as ffi;

#[macro_use]
mod ffi_util;

pub mod backup;
mod comparator;
pub mod merge_operator;
pub mod compaction_filter;
mod db;
mod db_options;

pub use db::{DBCompactionStyle, DBCompressionType, DBIterator, DBRawIterator, DBRecoveryMode, DBVector,
             ReadOptions, Direction, IteratorMode, Snapshot, WriteBatch, new_bloom_filter};

pub use merge_operator::MergeOperands;
pub use compaction_filter::Decision as CompactionDecision;
use std::collections::BTreeMap;
use std::error;
use std::fmt;
use std::path::PathBuf;

/// A RocksDB database.
///
/// See crate level documentation for a simple usage example.
pub struct DB {
    inner: *mut ffi::rocksdb_t,
    cfs: BTreeMap<String, ColumnFamily>,
    path: PathBuf,
}

/// A simple wrapper round a string, used for errors reported from
/// ffi calls.
#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    message: String,
}

impl Error {
    fn new(message: String) -> Error {
        Error { message: message }
    }

    pub fn to_string(self) -> String {
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

/// Database-wide options around performance and behavior.
///
/// Please read [the official tuning guide](https://github.com/facebook/rocksdb/wiki/RocksDB-Tuning-Guide), and most importantly, measure performance under realistic workloads with realistic hardware.
///
/// # Examples
///
/// ```
/// use rocksdb::{Options, DB};
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

/// Optionally disable WAL or sync for this write.
///
/// # Examples
///
/// Making an unsafe write of a batch:
///
/// ```
/// use rocksdb::{DB, WriteBatch, WriteOptions};
///
/// let db = DB::open_default("path/for/rocksdb/storageY").unwrap();
///
/// let mut batch = WriteBatch::default();
/// batch.put(b"my key", b"my value");
/// batch.put(b"key2", b"value2");
/// batch.put(b"key3", b"value3");
///
/// let mut write_options = WriteOptions::default();
/// write_options.set_sync(false);
/// write_options.disable_wal(true);
///
/// db.write_opt(batch, &write_options);
/// ```
pub struct WriteOptions {
    inner: *mut ffi::rocksdb_writeoptions_t,
}

/// An opaque type used to represent a column family. Returned from some functions, and used
/// in others
#[derive(Copy, Clone)]
pub struct ColumnFamily {
    inner: *mut ffi::rocksdb_column_family_handle_t,
}
