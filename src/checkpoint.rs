// Copyright 2018 Eugene P.
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

use crate::ops::*;
use ffi;
use std::ffi::CString;
use std::path::Path;
///! Implementation of bindings to RocksDB Checkpoint[1] API
///
/// [1]: https://github.com/facebook/rocksdb/wiki/Checkpoints
use {Error, DB};

/// Undocumented parameter for `ffi::rocksdb_checkpoint_create` function. Zero by default.
const LOG_SIZE_FOR_FLUSH: u64 = 0_u64;

/// Database's checkpoint object.
/// Used to create checkpoints of the specified DB from time to time.
pub struct Checkpoint {
    pub(crate) inner: *mut ffi::rocksdb_checkpoint_t,
}

impl Checkpoint {
    /// Creates new checkpoint object for specific DB.
    ///
    /// Does not actually produce checkpoints, call `.create_checkpoint()` method to produce
    /// a DB checkpoint.
    pub fn new(db: &DB) -> Result<Checkpoint, Error> {
        db.create_checkpoint_object()
    }

    /// Creates new physical DB checkpoint in directory specified by `path`.
    pub fn create_checkpoint<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        let path = path.as_ref();
        let cpath = match CString::new(path.to_string_lossy().as_bytes()) {
            Ok(c) => c,
            Err(_) => {
                return Err(Error::new(
                    "Failed to convert path to CString when creating DB checkpoint".to_owned(),
                ));
            }
        };

        unsafe {
            ffi_try!(ffi::rocksdb_checkpoint_create(
                self.inner,
                cpath.as_ptr(),
                LOG_SIZE_FOR_FLUSH,
            ));

            Ok(())
        }
    }
}

impl Drop for Checkpoint {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_checkpoint_object_destroy(self.inner);
        }
    }
}
