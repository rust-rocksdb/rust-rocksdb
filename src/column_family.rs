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

use std::marker::PhantomData;

use crate::{DB, Options};

/// A descriptor for a RocksDB column family.
///
/// A description of the column family, containing the name and `Options`.
pub struct ColumnFamilyDescriptor {
    pub(crate) name: String,
    pub(crate) options: Options,
}

/// An opaque type used to represent a column family. Returned from some functions, and used
/// in others
#[derive(Copy, Clone)]
pub struct ColumnFamily<'a> {
    pub(crate) inner: *mut ffi::rocksdb_column_family_handle_t,
    db: PhantomData<&'a DB>,
}

impl<'a> ColumnFamily<'a> {
  pub(crate) fn new(_: &DB, handle: *mut ffi::rocksdb_column_family_handle_t) -> ColumnFamily {
    ColumnFamily {
      inner: handle,
      db: PhantomData
    }
  }
}

unsafe impl<'a> Send for ColumnFamily<'a> {}
