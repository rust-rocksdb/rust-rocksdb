// Copyright 2020 Tyler Neely
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

#![warn(clippy::pedantic)]
#![allow(
    // Next `cast_*` lints don't give alternatives.
    clippy::cast_possible_wrap, clippy::cast_possible_truncation, clippy::cast_sign_loss,
    // Next lints produce too much noise/false positives.
    clippy::module_name_repetitions, clippy::similar_names, clippy::must_use_candidate,
    // '... may panic' lints.
    // Too much work to fix.
    clippy::missing_errors_doc,
    // False positive: WebSocket
    clippy::doc_markdown,
    clippy::missing_safety_doc,
    clippy::needless_pass_by_value,
    clippy::ptr_as_ptr,
    clippy::missing_panics_doc,
    clippy::from_over_into,
)]

#[macro_use]
mod ffi_util;

pub mod backup;
pub mod checkpoint;
mod column_family;
pub mod compaction_filter;
pub mod compaction_filter_factory;
mod comparator;
mod db;
mod db_iterator;
mod db_options;
mod db_pinnable_slice;
mod env;
mod iter_range;
pub mod merge_operator;
pub mod perf;
pub mod properties;
mod slice_transform;
mod snapshot;
mod sst_file_writer;
mod transactions;
mod write_batch;
mod write_buffer_manager;

pub use crate::{
    column_family::{
        AsColumnFamilyRef, BoundColumnFamily, ColumnFamily, ColumnFamilyDescriptor,
        ColumnFamilyRef, DEFAULT_COLUMN_FAMILY_NAME,
    },
    compaction_filter::Decision as CompactionDecision,
    db::{
        DBAccess, DBCommon, DBWithThreadMode, LiveFile, MultiThreaded, SingleThreaded, ThreadMode,
        DB,
    },
    db_iterator::{
        DBIterator, DBIteratorWithThreadMode, DBRawIterator, DBRawIteratorWithThreadMode,
        DBWALIterator, Direction, IteratorMode,
    },
    db_options::{
        BlockBasedIndexType, BlockBasedOptions, BottommostLevelCompaction, Cache, ChecksumType,
        CompactOptions, CuckooTableOptions, DBCompactionStyle, DBCompressionType, DBPath,
        DBRecoveryMode, DataBlockIndexType, FifoCompactOptions, FlushOptions,
        IngestExternalFileOptions, LogLevel, MemtableFactory, Options, PlainTableFactoryOptions,
        ReadOptions, UniversalCompactOptions, UniversalCompactionStopStyle, WriteOptions,
    },
    db_pinnable_slice::DBPinnableSlice,
    env::Env,
    ffi_util::CStrLike,
    iter_range::{IterateBounds, PrefixRange},
    merge_operator::MergeOperands,
    perf::{PerfContext, PerfMetric, PerfStatsLevel},
    slice_transform::SliceTransform,
    snapshot::{Snapshot, SnapshotWithThreadMode},
    sst_file_writer::SstFileWriter,
    transactions::{
        OptimisticTransactionDB, OptimisticTransactionOptions, Transaction, TransactionDB,
        TransactionDBOptions, TransactionOptions,
    },
    write_batch::{WriteBatch, WriteBatchIterator, WriteBatchWithTransaction},
    write_buffer_manager::WriteBufferManager,
};

use librocksdb_sys as ffi;

use std::error;
use std::fmt;

/// RocksDB error kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    NotFound,
    Corruption,
    NotSupported,
    InvalidArgument,
    IOError,
    MergeInProgress,
    Incomplete,
    ShutdownInProgress,
    TimedOut,
    Aborted,
    Busy,
    Expired,
    TryAgain,
    CompactionTooLarge,
    ColumnFamilyDropped,
    Unknown,
}

/// A simple wrapper round a string, used for errors reported from
/// ffi calls.
#[derive(Debug, Clone, PartialEq, Eq)]
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

    /// Parse corresponding [`ErrorKind`] from error message.
    pub fn kind(&self) -> ErrorKind {
        match self.message.split(':').next().unwrap_or("") {
            "NotFound" => ErrorKind::NotFound,
            "Corruption" => ErrorKind::Corruption,
            "Not implemented" => ErrorKind::NotSupported,
            "Invalid argument" => ErrorKind::InvalidArgument,
            "IO error" => ErrorKind::IOError,
            "Merge in progress" => ErrorKind::MergeInProgress,
            "Result incomplete" => ErrorKind::Incomplete,
            "Shutdown in progress" => ErrorKind::ShutdownInProgress,
            "Operation timed out" => ErrorKind::TimedOut,
            "Operation aborted" => ErrorKind::Aborted,
            "Resource busy" => ErrorKind::Busy,
            "Operation expired" => ErrorKind::Expired,
            "Operation failed. Try again." => ErrorKind::TryAgain,
            "Compaction too large" => ErrorKind::CompactionTooLarge,
            "Column family dropped" => ErrorKind::ColumnFamilyDropped,
            _ => ErrorKind::Unknown,
        }
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
    use crate::{
        OptimisticTransactionDB, OptimisticTransactionOptions, Transaction, TransactionDB,
        TransactionDBOptions, TransactionOptions, WriteBufferManager,
    };

    use super::{
        column_family::UnboundColumnFamily,
        db_options::CacheWrapper,
        env::{Env, EnvWrapper},
        BlockBasedOptions, BoundColumnFamily, Cache, ColumnFamily, ColumnFamilyDescriptor,
        DBIterator, DBRawIterator, IngestExternalFileOptions, Options, PlainTableFactoryOptions,
        ReadOptions, Snapshot, SstFileWriter, WriteBatch, WriteOptions, DB,
    };

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
        is_send::<IngestExternalFileOptions>();
        is_send::<BlockBasedOptions>();
        is_send::<PlainTableFactoryOptions>();
        is_send::<ColumnFamilyDescriptor>();
        is_send::<ColumnFamily>();
        is_send::<BoundColumnFamily<'_>>();
        is_send::<UnboundColumnFamily>();
        is_send::<SstFileWriter>();
        is_send::<WriteBatch>();
        is_send::<Cache>();
        is_send::<CacheWrapper>();
        is_send::<Env>();
        is_send::<EnvWrapper>();
        is_send::<TransactionDB>();
        is_send::<OptimisticTransactionDB>();
        is_send::<Transaction<'_, TransactionDB>>();
        is_send::<TransactionDBOptions>();
        is_send::<OptimisticTransactionOptions>();
        is_send::<TransactionOptions>();
        is_send::<WriteBufferManager>();
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
        is_sync::<IngestExternalFileOptions>();
        is_sync::<BlockBasedOptions>();
        is_sync::<PlainTableFactoryOptions>();
        is_sync::<UnboundColumnFamily>();
        is_sync::<ColumnFamilyDescriptor>();
        is_sync::<SstFileWriter>();
        is_sync::<Cache>();
        is_sync::<CacheWrapper>();
        is_sync::<Env>();
        is_sync::<EnvWrapper>();
        is_sync::<TransactionDB>();
        is_sync::<OptimisticTransactionDB>();
        is_sync::<TransactionDBOptions>();
        is_sync::<OptimisticTransactionOptions>();
        is_sync::<TransactionOptions>();
        is_sync::<WriteBufferManager>();
    }
}
