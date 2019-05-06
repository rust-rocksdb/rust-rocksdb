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

use ffi;
use ffi_util::to_cpath;

use crate::{
    handle::Handle,
    open_raw::{OpenRaw, OpenRawFFI},
    ops,
    ops::*,
    ColumnFamily, DBRawIterator, Error, Options, ReadOptions, Snapshot,
};

use std::collections::BTreeMap;
use std::ffi::CStr;
use std::fmt;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::slice;

/// A RocksDB database.
///
/// See crate level documentation for a simple usage example.
pub struct DB {
    pub(crate) inner: *mut ffi::rocksdb_t,
    cfs: BTreeMap<String, ColumnFamily>,
    path: PathBuf,
}

impl Handle<ffi::rocksdb_t> for DB {
    fn handle(&self) -> *mut ffi::rocksdb_t {
        self.inner
    }
}

impl ops::Open for DB {}
impl ops::OpenCF for DB {}

impl OpenRaw for DB {
    type Pointer = ffi::rocksdb_t;
    type Descriptor = ();

    fn open_ffi(input: OpenRawFFI<'_, Self::Descriptor>) -> Result<*mut Self::Pointer, Error> {
        let pointer = unsafe {
            if input.num_column_families <= 0 {
                ffi_try!(ffi::rocksdb_open(input.options, input.path,))
            } else {
                ffi_try!(ffi::rocksdb_open_column_families(
                    input.options,
                    input.path,
                    input.num_column_families,
                    input.column_family_names,
                    input.column_family_options,
                    input.column_family_handles,
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
        let cfs: BTreeMap<_, _> = column_families
            .into_iter()
            .map(|(k, h)| (k, ColumnFamily::new(h)))
            .collect();

        Ok(DB {
            inner: pointer,
            cfs,
            path,
        })
    }
}

impl ops::Read for DB {}
impl ops::Write for DB {}

unsafe impl Send for DB {}
unsafe impl Sync for DB {}

impl DB {
    pub fn list_cf<P: AsRef<Path>>(opts: &Options, path: P) -> Result<Vec<String>, Error> {
        let cpath = to_cpath(
            path,
            "Failed to convert path to CString when opening database.",
        )?;
        let mut length = 0;

        unsafe {
            let ptr = ffi_try!(ffi::rocksdb_list_column_families(
                opts.inner,
                cpath.as_ptr() as *const _,
                &mut length,
            ));

            let vec = slice::from_raw_parts(ptr, length)
                .iter()
                .map(|ptr| CStr::from_ptr(*ptr).to_string_lossy().into_owned())
                .collect();
            ffi::rocksdb_list_column_families_destroy(ptr, length);
            Ok(vec)
        }
    }

    pub fn destroy<P: AsRef<Path>>(opts: &Options, path: P) -> Result<(), Error> {
        let cpath = to_cpath(
            path,
            "Failed to convert path to CString when opening database.",
        )?;
        unsafe {
            ffi_try!(ffi::rocksdb_destroy_db(opts.inner, cpath.as_ptr(),));
        }
        Ok(())
    }

    pub fn repair<P: AsRef<Path>>(opts: Options, path: P) -> Result<(), Error> {
        let cpath = to_cpath(
            path,
            "Failed to convert path to CString when opening database.",
        )?;
        unsafe {
            ffi_try!(ffi::rocksdb_repair_db(opts.inner, cpath.as_ptr(),));
        }
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path.as_path()
    }

    pub fn snapshot(&self) -> Snapshot {
        let snapshot = unsafe { ffi::rocksdb_create_snapshot(self.inner) };
        Snapshot {
            db: self,
            inner: snapshot,
        }
    }
}

impl Drop for DB {
    fn drop(&mut self) {
        unsafe {
            for cf in self.cfs.values() {
                ffi::rocksdb_column_family_handle_destroy(cf.inner);
            }
            ffi::rocksdb_close(self.inner);
        }
    }
}

impl fmt::Debug for DB {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RocksDB {{ path: {:?} }}", self.path())
    }
}

impl Iterate for DB {
    fn get_raw_iter(&self, readopts: &ReadOptions) -> DBRawIterator {
        unsafe {
            DBRawIterator {
                inner: ffi::rocksdb_create_iterator(self.inner, readopts.handle()),
                db: PhantomData,
            }
        }
    }
}

impl IterateCF for DB {
    fn get_raw_iter_cf(
        &self,
        cf_handle: &ColumnFamily,
        readopts: &ReadOptions,
    ) -> Result<DBRawIterator, Error> {
        unsafe {
            Ok(DBRawIterator {
                inner: ffi::rocksdb_create_iterator_cf(
                    self.inner,
                    readopts.handle(),
                    cf_handle.inner,
                ),
                db: PhantomData,
            })
        }
    }
}

impl GetColumnFamilys for DB {
    fn get_cfs(&self) -> &BTreeMap<String, ColumnFamily> {
        &self.cfs
    }
    fn get_mut_cfs(&mut self) -> &mut BTreeMap<String, ColumnFamily> {
        &mut self.cfs
    }
}

#[test]
fn test_db_vector() {
    use crate::prelude::*;
    use libc::size_t;
    use std::mem;

    let len: size_t = 4;
    let data = unsafe { libc::calloc(len, mem::size_of::<u8>()) as *mut u8 };
    let v = unsafe { DBVector::from_c(data, len) };
    let ctrl = [0u8, 0, 0, 0];
    assert_eq!(&*v, &ctrl[..]);
}

#[test]
fn external() {
    use crate::{prelude::*, TemporaryDBPath};

    let path = TemporaryDBPath::new();
    {
        let db = DB::open_default(&path).unwrap();
        let p = db.put(b"k1", b"v1111");
        assert!(p.is_ok());
        let r: Result<Option<DBVector>, Error> = db.get(b"k1");
        assert!(r.unwrap().unwrap().to_utf8().unwrap() == "v1111");
        assert!(db.delete(b"k1").is_ok());
        assert!(db.get(b"k1").unwrap().is_none());
    }
}

#[test]
fn errors_do_stuff() {
    use crate::{prelude::*, TemporaryDBPath};

    let path = TemporaryDBPath::new();
    {
        let _db = DB::open_default(&path).unwrap();
        let opts = Options::default();
        // The DB will still be open when we try to destroy it and the lock should fail.
        match DB::destroy(&opts, &path) {
            Err(s) => {
                let message = s.to_string();
                assert!(message.find("IO error:").is_some());
                assert!(message.find("/LOCK:").is_some());
            }
            Ok(_) => panic!("should fail"),
        }
    }
}

#[test]
fn writebatch_works() {
    use crate::{prelude::*, TemporaryDBPath, WriteBatch};

    let path = TemporaryDBPath::new();
    {
        let db = DB::open_default(&path).unwrap();
        {
            // test putx
            let mut batch = WriteBatch::default();
            assert!(db.get(b"k1").unwrap().is_none());
            assert_eq!(batch.len(), 0);
            assert!(batch.is_empty());
            let _ = batch.put(b"k1", b"v1111");
            let _ = batch.put(b"k2", b"v2222");
            let _ = batch.put(b"k3", b"v3333");
            assert_eq!(batch.len(), 3);
            assert!(!batch.is_empty());
            assert!(db.get(b"k1").unwrap().is_none());
            let p = db.write(batch);
            assert!(p.is_ok());
            let r: Result<Option<DBVector>, Error> = db.get(b"k1");
            assert!(r.unwrap().unwrap().to_utf8().unwrap() == "v1111");
        }
        {
            // test delete
            let mut batch = WriteBatch::default();
            let _ = batch.delete(b"k1");
            assert_eq!(batch.len(), 1);
            assert!(!batch.is_empty());
            let p = db.write(batch);
            assert!(p.is_ok());
            assert!(db.get(b"k1").unwrap().is_none());
        }
        {
            // test delete_range
            let mut batch = WriteBatch::default();
            let _ = batch.delete_range(b"k2", b"k4");
            assert_eq!(batch.len(), 1);
            assert!(!batch.is_empty());
            let p = db.write(batch);
            assert!(p.is_ok());
            assert!(db.get(b"k2").unwrap().is_none());
            assert!(db.get(b"k3").unwrap().is_none());
        }
        {
            // test size_in_bytes
            let mut batch = WriteBatch::default();
            let before = batch.size_in_bytes();
            let _ = batch.put(b"k1", b"v1234567890");
            let after = batch.size_in_bytes();
            assert!(before + 10 <= after);
        }
    }
}

#[test]
fn iterator_test() {
    use crate::{prelude::*, IteratorMode, TemporaryDBPath};
    use std::str;

    let path = TemporaryDBPath::new();
    {
        let db = DB::open_default(&path).unwrap();
        let p = db.put(b"k1", b"v1111");
        assert!(p.is_ok());
        let p = db.put(b"k2", b"v2222");
        assert!(p.is_ok());
        let p = db.put(b"k3", b"v3333");
        assert!(p.is_ok());
        let iter = db.iterator(IteratorMode::Start);
        for (k, v) in iter {
            println!(
                "Hello {}: {}",
                str::from_utf8(&*k).unwrap(),
                str::from_utf8(&*v).unwrap()
            );
        }
    }
}

#[test]
fn snapshot_test() {
    use crate::{prelude::*, TemporaryDBPath};

    let path = TemporaryDBPath::new();
    {
        let db = DB::open_default(&path).unwrap();
        let p = db.put(b"k1", b"v1111");
        assert!(p.is_ok());

        let snap = db.snapshot();
        let r: Result<Option<DBVector>, Error> = snap.get(b"k1");
        assert!(r.unwrap().unwrap().to_utf8().unwrap() == "v1111");

        let p = db.put(b"k2", b"v2222");
        assert!(p.is_ok());

        assert!(db.get(b"k2").unwrap().is_some());
        assert!(snap.get(b"k2").unwrap().is_none());
    }
}

#[test]
fn set_option_test() {
    use crate::{prelude::*, TemporaryDBPath};

    let path = TemporaryDBPath::new();
    {
        let db = DB::open_default(&path).unwrap();
        // set an option to valid values
        assert!(db
            .set_options(&[("disable_auto_compactions", "true")])
            .is_ok());
        assert!(db
            .set_options(&[("disable_auto_compactions", "false")])
            .is_ok());
        // invalid names/values should result in an error
        assert!(db
            .set_options(&[("disable_auto_compactions", "INVALID_VALUE")])
            .is_err());
        assert!(db
            .set_options(&[("INVALID_NAME", "INVALID_VALUE")])
            .is_err());
        // option names/values must not contain NULLs
        assert!(db
            .set_options(&[("disable_auto_compactions", "true\0")])
            .is_err());
        assert!(db
            .set_options(&[("disable_auto_compactions\0", "true")])
            .is_err());
        // empty options are not allowed
        assert!(db.set_options(&[]).is_err());
        // multiple options can be set in a single API call
        let multiple_options = [
            ("paranoid_file_checks", "true"),
            ("report_bg_io_stats", "true"),
        ];
        db.set_options(&multiple_options).unwrap();
    }
}
