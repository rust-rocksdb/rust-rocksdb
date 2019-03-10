// Copyright 2019 Tyler Neely
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

use ffi;

use crate::{
    handle::Handle,
    open_raw::{OpenRaw, OpenRawFFI},
    ops, Error,
};

use libc::c_uchar;
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

pub struct ReadOnlyDB {
    pub(crate) inner: *mut ffi::rocksdb_t,
    cfs: Arc<RwLock<BTreeMap<String, *mut ffi::rocksdb_column_family_handle_t>>>,
    path: PathBuf,
}

impl ReadOnlyDB {
    pub fn path(&self) -> &Path {
        &self.path.as_path()
    }
}

pub struct ReadOnlyOpenDescriptor {
    error_if_log_file_exists: bool,
}

impl Default for ReadOnlyOpenDescriptor {
    fn default() -> Self {
        ReadOnlyOpenDescriptor {
            error_if_log_file_exists: true,
        }
    }
}

impl ops::Open for ReadOnlyDB {}
impl ops::OpenCF for ReadOnlyDB {}

impl OpenRaw for ReadOnlyDB {
    type Pointer = ffi::rocksdb_t;
    type Descriptor = ReadOnlyOpenDescriptor;

    fn open_impl(input: OpenRawFFI<'_, Self::Descriptor>) -> Result<*mut Self::Pointer, Error> {
        let error_if_log_file_exists = input.open_descriptor.error_if_log_file_exists as c_uchar;
        let pointer = unsafe {
            if input.num_column_families <= 0 {
                ffi_try!(ffi::rocksdb_open_for_read_only(
                    input.options,
                    input.path,
                    error_if_log_file_exists,
                ))
            } else {
                ffi_try!(ffi::rocksdb_open_for_read_only_column_families(
                    input.options,
                    input.path,
                    input.num_column_families,
                    input.column_family_names,
                    input.column_family_options,
                    input.column_family_handles,
                    error_if_log_file_exists,
                ))
            }
        };

        Ok(pointer)
    }

    fn build<I>(
        path: PathBuf,
        _open_descriptor: Self::Descriptor,
        pointer: *mut Self::Pointer,
        column_families: I,
    ) -> Result<Self, Error>
    where
        I: IntoIterator<Item = (String, *mut ffi::rocksdb_column_family_handle_t)>,
    {
        let cfs: BTreeMap<_, _> = column_families.into_iter().collect();

        Ok(ReadOnlyDB {
            inner: pointer,
            cfs: Arc::new(RwLock::new(cfs)),
            path,
        })
    }
}

impl Handle<ffi::rocksdb_t> for ReadOnlyDB {
    fn handle(&self) -> *mut ffi::rocksdb_t {
        self.inner
    }
}

impl ops::Read for ReadOnlyDB {}

unsafe impl Send for ReadOnlyDB {}
unsafe impl Sync for ReadOnlyDB {}

impl Drop for ReadOnlyDB {
    fn drop(&mut self) {
        unsafe {
            if let Ok(cfs) = self.cfs.read() {
                for cf in cfs.values() {
                    ffi::rocksdb_column_family_handle_destroy(*cf);
                }
            }
            ffi::rocksdb_close(self.inner);
        }
    }
}

impl fmt::Debug for ReadOnlyDB {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Read-only RocksDB {{ path: {:?} }}", self.path())
    }
}
