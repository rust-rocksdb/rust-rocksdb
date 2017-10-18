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
mod table_properties;
mod table_properties_collector;
mod table_properties_collector_factory;
mod event_listener;

pub use compaction_filter::CompactionFilter;
pub use event_listener::{CompactionJobInfo, EventListener, FlushJobInfo, IngestionInfo};
pub use librocksdb_sys::{self as crocksdb_ffi, new_bloom_filter, CompactionPriority,
                         DBCompactionStyle, DBCompressionType, DBEntryType, DBInfoLogLevel,
                         DBRecoveryMode, DBStatisticsHistogramType, DBStatisticsTickerType};
pub use merge_operator::MergeOperands;
pub use rocksdb::{BackupEngine, CFHandle, DBIterator, DBVector, Kv, Range, SeekKey, SstFileWriter,
                  Writable, WriteBatch, DB};
pub use rocksdb_options::{BlockBasedOptions, ColumnFamilyOptions, CompactOptions, DBOptions,
                          EnvOptions, HistogramData, IngestExternalFileOptions, ReadOptions,
                          RestoreOptions, WriteOptions, FifoCompactionOptions};
pub use slice_transform::SliceTransform;
pub use table_properties::{TableProperties, TablePropertiesCollection,
                           TablePropertiesCollectionView, UserCollectedProperties};
pub use table_properties_collector::TablePropertiesCollector;
pub use table_properties_collector_factory::TablePropertiesCollectorFactory;
