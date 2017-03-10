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

extern crate libc;

#[cfg(test)]
extern crate tempdir;
#[macro_use]
pub extern crate librocksdb_sys;

pub mod rocksdb;
pub mod rocksdb_options;
pub mod merge_operator;
pub mod comparator;
mod compaction_filter;
mod slice_transform;

pub use compaction_filter::CompactionFilter;
pub use librocksdb_sys::{DBCompactionStyle, DBCompressionType, DBRecoveryMode,
                         DBStatisticsTickerType, DBStatisticsHistogramType, new_bloom_filter,
                         self as crocksdb_ffi};
pub use merge_operator::MergeOperands;
pub use rocksdb::{DB, DBIterator, DBVector, Kv, SeekKey, Writable, WriteBatch, CFHandle, Range,
                  BackupEngine, SstFileWriter};
pub use rocksdb_options::{BlockBasedOptions, Options, ReadOptions, WriteOptions, RestoreOptions,
                          IngestExternalFileOptions, EnvOptions, HistogramData};
pub use slice_transform::SliceTransform;
