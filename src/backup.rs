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

use crate::{ffi, Error, DB};

use libc::{c_int, c_uchar};
use std::ffi::CString;
use std::path::Path;

/// Represents information of a backup including timestamp of the backup
/// and the size (please note that sum of all backups' sizes is bigger than the actual
/// size of the backup directory because some data is shared by multiple backups).
/// Backups are identified by their always-increasing IDs.
pub struct BackupEngineInfo {
    /// Timestamp of the backup
    pub timestamp: i64,
    /// ID of the backup
    pub backup_id: u32,
    /// Size of the backup
    pub size: u64,
    /// Number of files related to the backup
    pub num_files: u32,
}

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
    pub fn open<P: AsRef<Path>>(
        opts: &BackupEngineOptions,
        path: P,
    ) -> Result<BackupEngine, Error> {
        let path = path.as_ref();
        let cpath = if let Ok(e) = CString::new(path.to_string_lossy().as_bytes()) {
            e
        } else {
            return Err(Error::new(
                "Failed to convert path to CString \
                     when opening backup engine"
                    .to_owned(),
            ));
        };

        let be: *mut ffi::rocksdb_backup_engine_t;
        unsafe { be = ffi_try!(ffi::rocksdb_backup_engine_open(opts.inner, cpath.as_ptr())) }

        if be.is_null() {
            return Err(Error::new("Could not initialize backup engine.".to_owned()));
        }

        Ok(BackupEngine { inner: be })
    }

    /// Captures the state of the database in the latest backup.
    ///
    /// Note: no flush before backup is performed. User might want to
    /// use `create_new_backup_flush` instead.
    pub fn create_new_backup(&mut self, db: &DB) -> Result<(), Error> {
        self.create_new_backup_flush(db, false)
    }

    /// Captures the state of the database in the latest backup.
    ///
    /// Set flush_before_backup=true to avoid losing unflushed key/value
    /// pairs from the memtable.
    pub fn create_new_backup_flush(
        &mut self,
        db: &DB,
        flush_before_backup: bool,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_backup_engine_create_new_backup_flush(
                self.inner,
                db.inner,
                flush_before_backup as c_uchar,
            ));
            Ok(())
        }
    }

    pub fn purge_old_backups(&mut self, num_backups_to_keep: usize) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_backup_engine_purge_old_backups(
                self.inner,
                num_backups_to_keep as u32,
            ));
            Ok(())
        }
    }

    /// Restore from the latest backup
    ///
    /// # Arguments
    ///
    /// * `db_dir` - A path to the database directory
    /// * `wal_dir` - A path to the wal directory
    /// * `opts` - Restore options
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rocksdb::backup::{BackupEngine, BackupEngineOptions};
    /// let backup_opts = BackupEngineOptions::default();
    /// let mut backup_engine = BackupEngine::open(&backup_opts, &backup_path).unwrap();
    /// let mut restore_option = rocksdb::backup::RestoreOptions::default();
    /// restore_option.set_keep_log_files(true); /// true to keep log files
    /// if let Err(e) = backup_engine.restore_from_latest_backup(&db_path, &wal_dir, &restore_option) {
    ///     error!("Failed to restore from the backup. Error:{:?}", e);
    ///     return Err(e.to_string());
    ///  }
    /// ```

    pub fn restore_from_latest_backup<D: AsRef<Path>, W: AsRef<Path>>(
        &mut self,
        db_dir: D,
        wal_dir: W,
        opts: &RestoreOptions,
    ) -> Result<(), Error> {
        let db_dir = db_dir.as_ref();
        let c_db_dir = if let Ok(c) = CString::new(db_dir.to_string_lossy().as_bytes()) {
            c
        } else {
            return Err(Error::new(
                "Failed to convert db_dir to CString \
                     when restoring from latest backup"
                    .to_owned(),
            ));
        };

        let wal_dir = wal_dir.as_ref();
        let c_wal_dir = if let Ok(c) = CString::new(wal_dir.to_string_lossy().as_bytes()) {
            c
        } else {
            return Err(Error::new(
                "Failed to convert wal_dir to CString \
                     when restoring from latest backup"
                    .to_owned(),
            ));
        };

        unsafe {
            ffi_try!(ffi::rocksdb_backup_engine_restore_db_from_latest_backup(
                self.inner,
                c_db_dir.as_ptr(),
                c_wal_dir.as_ptr(),
                opts.inner,
            ));
        }
        Ok(())
    }

    /// Checks that each file exists and that the size of the file matches our
    /// expectations. it does not check file checksum.
    ///
    /// If this BackupEngine created the backup, it compares the files' current
    /// sizes against the number of bytes written to them during creation.
    /// Otherwise, it compares the files' current sizes against their sizes when
    /// the BackupEngine was opened.
    pub fn verify_backup(&self, backup_id: u32) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_backup_engine_verify_backup(
                self.inner, backup_id,
            ));
        }
        Ok(())
    }

    /// Get a list of all backups together with information on timestamp of the backup
    /// and the size (please note that sum of all backups' sizes is bigger than the actual
    /// size of the backup directory because some data is shared by multiple backups).
    /// Backups are identified by their always-increasing IDs.
    ///
    /// You can perform this function safely, even with other BackupEngine performing
    /// backups on the same directory
    pub fn get_backup_info(&self) -> Vec<BackupEngineInfo> {
        unsafe {
            let i = ffi::rocksdb_backup_engine_get_backup_info(self.inner);

            let n = ffi::rocksdb_backup_engine_info_count(i);

            let mut info = Vec::with_capacity(n as usize);
            for index in 0..n {
                info.push(BackupEngineInfo {
                    timestamp: ffi::rocksdb_backup_engine_info_timestamp(i, index),
                    backup_id: ffi::rocksdb_backup_engine_info_backup_id(i, index),
                    size: ffi::rocksdb_backup_engine_info_size(i, index),
                    num_files: ffi::rocksdb_backup_engine_info_number_files(i, index),
                })
            }

            // destroy backup info object
            ffi::rocksdb_backup_engine_info_destroy(i);

            info
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
                panic!("Could not create RocksDB backup options");
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
                panic!("Could not create RocksDB restore options");
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
