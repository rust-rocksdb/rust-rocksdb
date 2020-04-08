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
//! use rocksdb::{DB, Options};
//! // NB: db is automatically closed at end of lifetime
//! let path = "_path_for_rocksdb_storage";
//! {
//!    let db = DB::open_default(path).unwrap();
//!    db.put(b"my key", b"my value").unwrap();
//!    match db.get(b"my key") {
//!        Ok(Some(value)) => println!("retrieved value {}", String::from_utf8(value).unwrap()),
//!        Ok(None) => println!("value not found"),
//!        Err(e) => println!("operational problem encountered: {}", e),
//!    }
//!    db.delete(b"my key").unwrap();
//! }
//! let _ = DB::destroy(&Options::default(), path);
//! ```
//!
//! Opening a database and a single column family with custom options:
//!
//! ```
//! use rocksdb::{DB, ColumnFamilyDescriptor, Options};
//!
//! let path = "_path_for_rocksdb_storage_with_cfs";
//! let mut cf_opts = Options::default();
//! cf_opts.set_max_write_buffer_number(16);
//! let cf = ColumnFamilyDescriptor::new("cf1", cf_opts);
//!
//! let mut db_opts = Options::default();
//! db_opts.create_missing_column_families(true);
//! db_opts.create_if_missing(true);
//! {
//!     let db = DB::open_cf_descriptors(&db_opts, path, vec![cf]).unwrap();
//! }
//! let _ = DB::destroy(&db_opts, path);
//! ```
//!

#[macro_use]
mod ffi_util;

pub mod backup;
pub mod checkpoint;
mod column_family;
pub mod compaction_filter;
mod comparator;
mod db;
mod db_iterator;
mod db_options;
mod db_pinnable_slice;
pub mod merge_operator;
mod slice_transform;
mod snapshot;
mod write_batch;

pub use crate::{
    column_family::{ColumnFamily, ColumnFamilyDescriptor},
    compaction_filter::Decision as CompactionDecision,
    db::DB,
    db_iterator::{DBIterator, DBRawIterator, DBWALIterator, Direction, IteratorMode},
    db_options::{
        BlockBasedIndexType, BlockBasedOptions, DBCompactionStyle, DBCompressionType,
        DBRecoveryMode, DataBlockIndexType, FlushOptions, MemtableFactory, Options,
        PlainTableFactoryOptions, ReadOptions, WriteOptions,
    },
    db_pinnable_slice::DBPinnableSlice,
    merge_operator::MergeOperands,
    slice_transform::SliceTransform,
    snapshot::Snapshot,
    write_batch::{WriteBatch, WriteBatchIterator},
};

use librocksdb_sys as ffi;

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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn is_send() {
        // test (at compile time) that certain types implement the auto-trait Send, either directly for
        // pointer-wrapping types or transitively for types with all Send fields

        fn is_send<T: Send>() {
            // dummy function just used for its parameterized type bound
        }

        is_send::<DB>();
        is_send::<DBIterator<'_>>();
        is_send::<DBRawIterator<'_>>();
        is_send::<Snapshot>();
        is_send::<Options>();
        is_send::<ReadOptions>();
        is_send::<WriteOptions>();
        is_send::<BlockBasedOptions>();
        is_send::<PlainTableFactoryOptions>();
        is_send::<ColumnFamilyDescriptor>();
        is_send::<ColumnFamily>();
    }

    #[test]
    fn is_sync() {
        // test (at compile time) that certain types implement the auto-trait Sync

        fn is_sync<T: Sync>() {
            // dummy function just used for its parameterized type bound
        }

        is_sync::<DB>();
        is_sync::<Snapshot>();
        is_sync::<Options>();
        is_sync::<ReadOptions>();
        is_sync::<WriteOptions>();
        is_sync::<BlockBasedOptions>();
        is_sync::<PlainTableFactoryOptions>();
        is_sync::<ColumnFamilyDescriptor>();
    }
}
