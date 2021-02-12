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

//! Implementation of bindings to RocksDB Checkpoint[1] API
//!
//! [1]: https://github.com/facebook/rocksdb/wiki/Checkpoints

use crate::{ffi, Error, DB};
use std::ffi::CString;
use std::marker::PhantomData;
use std::path::Path;

/// Undocumented parameter for `ffi::rocksdb_checkpoint_create` function. Zero by default.
const LOG_SIZE_FOR_FLUSH: u64 = 0_u64;

/// Database's checkpoint object.
/// Used to create checkpoints of the specified DB from time to time.
pub struct Checkpoint<'db> {
    inner: *mut ffi::rocksdb_checkpoint_t,
    _db: PhantomData<&'db ()>,
}

impl<'db> Checkpoint<'db> {
    /// Creates new checkpoint object for specific DB.
    ///
    /// Does not actually produce checkpoints, call `.create_checkpoint()` method to produce
    /// a DB checkpoint.
    pub fn new(db: &'db DB) -> Result<Checkpoint<'db>, Error> {
        let checkpoint: *mut ffi::rocksdb_checkpoint_t;

        unsafe { checkpoint = ffi_try!(ffi::rocksdb_checkpoint_object_create(db.inner)) };

        if checkpoint.is_null() {
            return Err(Error::new("Could not create checkpoint object.".to_owned()));
        }

        Ok(Checkpoint {
            inner: checkpoint,
            _db: PhantomData,
        })
    }

    /// Creates new physical DB checkpoint in directory specified by `path`.
    pub fn create_checkpoint<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        let path = path.as_ref();
        let cpath = if let Ok(c) = CString::new(path.to_string_lossy().as_bytes()) {
            c
        } else {
            return Err(Error::new(
                "Failed to convert path to CString when creating DB checkpoint".to_owned(),
            ));
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

impl<'db> Drop for Checkpoint<'db> {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_checkpoint_object_destroy(self.inner);
        }
    }
}
