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
    handle::Handle, ColumnFamilyDescriptor,  Error,
    Options, ops,
};

use libc::{c_int, c_uchar};
use std::collections::BTreeMap;
use std::ffi::{CString};
use std::path::{Path};
use std::ptr;
use std::str;
use std::sync::{Arc, RwLock};

pub struct ReadOnlyDB {
    pub(crate) inner: *mut ffi::rocksdb_t,
    cfs: Arc<RwLock<BTreeMap<String, *mut ffi::rocksdb_column_family_handle_t>>>,
}

impl ReadOnlyDB {
      /// Open a database with default options.
    pub fn open_default<P: AsRef<Path>>(error_if_log_file_exists: bool, path: P) -> Result<ReadOnlyDB, Error> {
        let opts = Options::default();
        ReadOnlyDB::open(error_if_log_file_exists, &opts, path)
    }

  /// Open the database with the specified options.
    pub fn open<P: AsRef<Path>>(
      error_if_log_file_exists: bool,
      opts: &Options,
       path: P) -> Result<ReadOnlyDB, Error> {

        ReadOnlyDB::open_cf(error_if_log_file_exists, opts, path, None::<&str>)
    }

    /// Open a database with the given database options and column family names.
    ///
    /// Column families opened using this function will be created with default `Options`.
    pub fn open_cf<P, I, N>(
        error_if_log_file_exists: bool, 
        opts: &Options,
        path: P,
        cfs: I) -> Result<ReadOnlyDB, Error>

    where
        P: AsRef<Path>,
        I: IntoIterator<Item = N>,
        N: AsRef<str>,
    {
        let cfs = cfs
            .into_iter()
            .map(|name| ColumnFamilyDescriptor::new(name.as_ref(), Options::default()));

        ReadOnlyDB::open_cf_descriptors(error_if_log_file_exists, opts, path, cfs)
    }

    /// Open a database with the given database options and column family descriptors.
    pub fn open_cf_descriptors<P, I>(
      error_if_log_file_exists: bool,
      opts: &Options,
      path: P,
      cfs: I) -> Result<ReadOnlyDB, Error>

    where
        P: AsRef<Path>,
        I: IntoIterator<Item = ColumnFamilyDescriptor>,
    {
        let cfs: Vec<_> = cfs.into_iter().collect();

        let path = path.as_ref();
        let cpath = match CString::new(path.to_string_lossy().as_bytes()) {
            Ok(c) => c,
            Err(_) => {
                return Err(Error::new(
                    "Failed to convert path to CString \
                     when opening DB."
                        .to_owned(),
                ));
            }
        };

        let db: *mut ffi::rocksdb_t;
        let cf_map = Arc::new(RwLock::new(BTreeMap::new()));

        if cfs.is_empty() {
            unsafe {
                db = ffi_try!(ffi::rocksdb_open_for_read_only(
                  opts.inner,
                  cpath.as_ptr() as *const _,
                  error_if_log_file_exists as c_uchar,));
            }
        } else {
            let mut cfs_v = cfs;
            // Always open the default column family.
            if !cfs_v.iter().any(|cf| cf.name == "default") {
                cfs_v.push(ColumnFamilyDescriptor {
                    name: String::from("default"),
                    options: Options::default(),
                });
            }
            // We need to store our CStrings in an intermediate vector
            // so that their pointers remain valid.
            let c_cfs: Vec<CString> = cfs_v
                .iter()
                .map(|cf| CString::new(cf.name.as_bytes()).unwrap())
                .collect();

            let mut cfnames: Vec<_> = c_cfs.iter().map(|cf| cf.as_ptr()).collect();

            // These handles will be populated by DB.
            let mut cfhandles: Vec<_> = cfs_v.iter().map(|_| ptr::null_mut()).collect();

            let mut cfopts: Vec<_> = cfs_v
                .iter()
                .map(|cf| cf.options.inner as *const _)
                .collect();

            unsafe {
                db = ffi_try!(ffi::rocksdb_open_for_read_only_column_families(
                    opts.inner,
                    cpath.as_ptr(),
                    cfs_v.len() as c_int,
                    cfnames.as_mut_ptr(),
                    cfopts.as_mut_ptr(),
                    cfhandles.as_mut_ptr(),
                    error_if_log_file_exists as c_uchar,
                ));
            }

            for handle in &cfhandles {
                if handle.is_null() {
                    return Err(Error::new(
                        "Received null column family \
                         handle from DB."
                            .to_owned(),
                    ));
                }
            }

            for (n, h) in cfs_v.iter().zip(cfhandles) {
                cf_map
                    .write()
                    .map_err(|e| Error::new(e.to_string()))?
                    .insert(n.name.clone(), h);
            }
        }

        if db.is_null() {
            return Err(Error::new("Could not initialize database.".to_owned()));
        }

        Ok(ReadOnlyDB {
            inner: db,
            cfs: cf_map,
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