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
//! # let path = TemporaryDBPath::new(path);
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
//! use rocksdb::{DB, ColumnFamilyDescriptor, Options};
//! # use rocksdb::TemporaryDBPath;
//!
//! let path = "_path_for_rocksdb_storage_with_cfs";
//! # let path = TemporaryDBPath::new(path);
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
mod db_vector;
mod handle;
pub mod merge_operator;
pub mod ops;
mod slice_transform;

pub mod prelude;

pub use column_family::{ColumnFamily, ColumnFamilyDescriptor};
pub use compaction_filter::Decision as CompactionDecision;
pub use db::{DBPinnableSlice, Snapshot, WriteBatch, DB};
pub use db_iterator::{DBIterator, DBRawIterator, Direction, IteratorMode};
pub use db_options::{
    BlockBasedIndexType, BlockBasedOptions, DBCompactionStyle, DBCompressionType, DBRecoveryMode,
    MemtableFactory, Options, PlainTableFactoryOptions, ReadOptions, WriteOptions,
};
pub use db_vector::DBVector;
pub use util::TemporaryDBPath;

pub use slice_transform::SliceTransform;

pub use merge_operator::MergeOperands;
use std::error;
use std::fmt;

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