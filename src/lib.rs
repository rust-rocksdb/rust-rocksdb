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
#![feature(plugin)]

extern crate libc;

#[cfg(test)]
extern crate tempdir;

pub mod rocksdb;
pub mod ffi;
pub mod rocksdb_options;
pub mod merge_operator;
pub mod comparator;

pub use ffi::{DBCompactionStyle, DBComparator, new_bloom_filter,
              self as rocksdb_ffi};
pub use rocksdb::{DB, DBIterator, DBVector, Direction, IteratorMode, Writable,
                  WriteBatch};
pub use rocksdb_options::{BlockBasedOptions, Options, WriteOptions};
pub use merge_operator::MergeOperands;
