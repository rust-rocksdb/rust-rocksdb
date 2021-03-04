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

use crate::{ffi, Options};

/// The name of the default column family.
///
/// The column family with this name is created implicitly whenever column
/// families are used.
pub const DEFAULT_COLUMN_FAMILY_NAME: &str = "default";

/// A descriptor for a RocksDB column family.
///
/// A description of the column family, containing the name and `Options`.
pub struct ColumnFamilyDescriptor {
    pub(crate) name: String,
    pub(crate) options: Options,
}

impl ColumnFamilyDescriptor {
    // Create a new column family descriptor with the specified name and options.
    pub fn new<S>(name: S, options: Options) -> Self
    where
        S: Into<String>,
    {
        ColumnFamilyDescriptor {
            name: name.into(),
            options,
        }
    }
}

/// An opaque type used to represent a column family. Returned from some functions, and used
/// in others
pub struct ColumnFamily {
    pub(crate) inner: *mut ffi::rocksdb_column_family_handle_t,
}

unsafe impl Send for ColumnFamily {}
