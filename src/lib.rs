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
extern crate rocksdb_sys as ffi;

pub mod merge_operator;
pub mod comparator;

mod rocksdb;
mod rocksdb_options;

pub use rocksdb::{DB, DBCompressionType, DBCompactionStyle, DBRecoveryMode, DBIterator, DBVector, Direction, IteratorMode, Snapshot, Writable, WriteBatch, Error, new_bloom_filter};
pub use merge_operator::MergeOperands;

pub struct BlockBasedOptions {
    inner: *mut ffi::rocksdb_block_based_table_options_t,
}

pub struct Options {
    inner: *mut ffi::rocksdb_options_t,
}

pub struct WriteOptions {
    inner: *mut ffi::rocksdb_writeoptions_t,
}
