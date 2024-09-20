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

use crate::env::Env;
use crate::{db::DBInner, ffi, ffi_util::to_cpath, DBCommon, Error, ThreadMode};

use libc::c_uchar;
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
    _outlive: Env,
}

pub struct BackupEngineOptions {
    inner: *mut ffi::rocksdb_backup_engine_options_t,
}

pub struct RestoreOptions {
    inner: *mut ffi::rocksdb_restore_options_t,
}

// BackupEngine is a simple pointer wrapper, so it's safe to send to another thread
// since the underlying RocksDB backup engine is thread-safe.
unsafe impl Send for BackupEngine {}

impl BackupEngine {
    /// Open a backup engine with the specified options and RocksDB Env.
    pub fn open(opts: &BackupEngineOptions, env: &Env) -> Result<Self, Error> {
        let be: *mut ffi::rocksdb_backup_engine_t;
        unsafe {
            be = ffi_try!(ffi::rocksdb_backup_engine_open_opts(
                opts.inner,
                env.0.inner
            ));
        }

        if be.is_null() {
            return Err(Error::new("Could not initialize backup engine.".to_owned()));
        }

        Ok(Self {
            inner: be,
            _outlive: env.clone(),
        })
    }

    /// Captures the state of the database in the latest backup.
    ///
    /// Note: no flush before backup is performed. User might want to
    /// use `create_new_backup_flush` instead.
    pub fn create_new_backup<T: ThreadMode, D: DBInner>(
        &mut self,
        db: &DBCommon<T, D>,
    ) -> Result<(), Error> {
        self.create_new_backup_flush(db, false)
    }

    /// Captures the state of the database in the latest backup.
    ///
    /// Set flush_before_backup=true to avoid losing unflushed key/value
    /// pairs from the memtable.
    pub fn create_new_backup_flush<T: ThreadMode, D: DBInner>(
        &mut self,
        db: &DBCommon<T, D>,
        flush_before_backup: bool,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_backup_engine_create_new_backup_flush(
                self.inner,
                db.inner.inner(),
                c_uchar::from(flush_before_backup),
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
        let c_db_dir = to_cpath(db_dir)?;
        let c_wal_dir = to_cpath(wal_dir)?;

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

    /// Restore from a specified backup
    ///
    /// The specified backup id should be passed in as an additional parameter.
    pub fn restore_from_backup<D: AsRef<Path>, W: AsRef<Path>>(
        &mut self,
        db_dir: D,
        wal_dir: W,
        opts: &RestoreOptions,
        backup_id: u32,
    ) -> Result<(), Error> {
        let c_db_dir = to_cpath(db_dir)?;
        let c_wal_dir = to_cpath(wal_dir)?;

        unsafe {
            ffi_try!(ffi::rocksdb_backup_engine_restore_db_from_backup(
                self.inner,
                c_db_dir.as_ptr(),
                c_wal_dir.as_ptr(),
                opts.inner,
                backup_id,
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
                });
            }

            // destroy backup info object
            ffi::rocksdb_backup_engine_info_destroy(i);

            info
        }
    }
}

impl BackupEngineOptions {
    /// Initializes `BackupEngineOptions` with the directory to be used for storing/accessing the
    /// backup files.
    pub fn new<P: AsRef<Path>>(backup_dir: P) -> Result<Self, Error> {
        let backup_dir = backup_dir.as_ref();
        let c_backup_dir = CString::new(backup_dir.to_string_lossy().as_bytes()).map_err(|_| {
            Error::new(
                "Failed to convert backup_dir to CString \
                     when constructing BackupEngineOptions"
                    .to_owned(),
            )
        })?;

        unsafe {
            let opts = ffi::rocksdb_backup_engine_options_create(c_backup_dir.as_ptr());
            assert!(!opts.is_null(), "Could not create RocksDB backup options");

            Ok(Self { inner: opts })
        }
    }

    /// Sets the number of operations (such as file copies or file checksums) that `RocksDB` may
    /// perform in parallel when executing a backup or restore.
    ///
    /// Default: 1
    pub fn set_max_background_operations(&mut self, max_background_operations: i32) {
        unsafe {
            ffi::rocksdb_backup_engine_options_set_max_background_operations(
                self.inner,
                max_background_operations,
            );
        }
    }
}

impl RestoreOptions {
    /// Sets `keep_log_files`. If true, restore won't overwrite the existing log files in wal_dir.
    /// It will also move all log files from archive directory to wal_dir. Use this option in
    /// combination with BackupEngineOptions::backup_log_files = false for persisting in-memory
    /// databases.
    ///
    /// Default: false
    pub fn set_keep_log_files(&mut self, keep_log_files: bool) {
        unsafe {
            ffi::rocksdb_restore_options_set_keep_log_files(self.inner, i32::from(keep_log_files));
        }
    }
}

impl Default for RestoreOptions {
    fn default() -> Self {
        unsafe {
            let opts = ffi::rocksdb_restore_options_create();
            assert!(!opts.is_null(), "Could not create RocksDB restore options");

            Self { inner: opts }
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
            ffi::rocksdb_backup_engine_options_destroy(self.inner);
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
