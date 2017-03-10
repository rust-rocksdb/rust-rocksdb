// Copyright 2016 Alex Regueiro
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


use {DB, Error};
use ffi;

use libc::{c_int, uint32_t};
use std::ffi::CString;
use std::path::Path;

pub struct BackupEngine {
    inner: *mut ffi::rocksdb_backup_engine_t,
}

pub struct BackupEngineOptions {
    inner: *mut ffi::rocksdb_options_t,
}

pub struct RestoreOptions {
    inner: *mut ffi::rocksdb_restore_options_t,
}

impl BackupEngine {
    /// Open a backup engine with the specified options.
    pub fn open<P: AsRef<Path>>(opts: &BackupEngineOptions,
                                path: P)
                                -> Result<BackupEngine, Error> {
        let path = path.as_ref();
        let cpath = match CString::new(path.to_string_lossy().as_bytes()) {
            Ok(c) => c,
            Err(_) => {
                return Err(Error::new("Failed to convert path to CString \
                                       when opening backup engine"
                    .to_owned()))
            }
        };

        let be: *mut ffi::rocksdb_backup_engine_t;
        unsafe { be = ffi_try!(ffi::rocksdb_backup_engine_open(opts.inner, cpath.as_ptr())) }

        if be.is_null() {
            return Err(Error::new("Could not initialize backup engine.".to_owned()));
        }

        Ok(BackupEngine { inner: be })
    }

    pub fn create_new_backup(&mut self, db: &DB) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_backup_engine_create_new_backup(self.inner, db.inner));
            Ok(())
        }
    }

    pub fn purge_old_backups(&mut self, num_backups_to_keep: usize) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_backup_engine_purge_old_backups(self.inner,
                                                                  num_backups_to_keep as uint32_t));
            Ok(())
        }
    }
}

impl BackupEngineOptions {
    //
}

impl RestoreOptions {
    pub fn set_keep_log_files(&mut self, keep_log_files: bool) {
        unsafe {
            ffi::rocksdb_restore_options_set_keep_log_files(self.inner, keep_log_files as c_int);
        }
    }
}

impl Default for BackupEngineOptions {
    fn default() -> BackupEngineOptions {
        unsafe {
            let opts = ffi::rocksdb_options_create();
            if opts.is_null() {
                panic!("Could not create RocksDB backup options".to_owned());
            }
            BackupEngineOptions { inner: opts }
        }
    }
}

impl Default for RestoreOptions {
    fn default() -> RestoreOptions {
        unsafe {
            let opts = ffi::rocksdb_restore_options_create();
            if opts.is_null() {
                panic!("Could not create RocksDB restore options".to_owned());
            }
            RestoreOptions { inner: opts }
        }
    }
}

impl Drop for BackupEngine {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_backup_engine_close(self.inner);
        }
    }
}

impl Drop for BackupEngineOptions {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_options_destroy(self.inner);
        }
    }
}

impl Drop for RestoreOptions {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_restore_options_destroy(self.inner);
        }
    }
}
