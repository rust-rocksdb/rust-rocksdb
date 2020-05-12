// Copyright 2020 Lucjan Suski
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
//`

use crate::{ffi, ffi_util::to_cpath, Error, Options};

use libc::{self, c_char, size_t};
use std::ffi::CString;
use std::marker::PhantomData;
use std::path::Path;

pub struct SstFileWriter<'a> {
    pub(crate) inner: *mut ffi::rocksdb_sstfilewriter_t,
    // Options are needed to be alive when calling open(),
    // so let's make sure it doesn't get, dropped for the lifetime of SstFileWriter
    phantom: PhantomData<&'a Options>,
}

unsafe impl<'a> Send for SstFileWriter<'a> {}
unsafe impl<'a> Sync for SstFileWriter<'a> {}

pub struct EnvOptions {
    pub(crate) inner: *mut ffi::rocksdb_envoptions_t,
}

impl Drop for EnvOptions {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_envoptions_destroy(self.inner);
        }
    }
}

impl Default for EnvOptions {
    fn default() -> EnvOptions {
        let opts = unsafe { ffi::rocksdb_envoptions_create() };
        EnvOptions { inner: opts }
    }
}

impl<'a> SstFileWriter<'a> {
    pub fn create(opts: &'a Options) -> SstFileWriter {
        let env_options = EnvOptions::default();

        let writer = SstFileWriter::create_raw(opts, &env_options);

        SstFileWriter {
            inner: writer,
            phantom: PhantomData,
        }
    }

    pub fn create_raw(opts: &Options, env_opts: &EnvOptions) -> *mut ffi::rocksdb_sstfilewriter_t {
        unsafe { ffi::rocksdb_sstfilewriter_create(env_opts.inner, opts.inner) }
    }

    pub fn open<P: AsRef<Path>>(&'a self, path: P) -> Result<(), Error> {
        let cpath = to_cpath(&path)?;
        self.open_raw(&cpath)
    }

    pub fn open_raw(&'a self, cpath: &CString) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_sstfilewriter_open(
                self.inner,
                cpath.as_ptr() as *const _
            ));

            Ok(())
        }
    }

    pub fn finish(&mut self) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_sstfilewriter_finish(self.inner,));
            Ok(())
        }
    }

    pub fn file_size(&self) -> u64 {
        let mut file_size: u64 = 0;
        unsafe { ffi::rocksdb_sstfilewriter_file_size(self.inner, &mut file_size) };
        file_size
    }

    pub fn put<K, V>(&mut self, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let key = key.as_ref();
        let value = value.as_ref();

        unsafe {
            ffi_try!(ffi::rocksdb_sstfilewriter_put(
                self.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
                value.as_ptr() as *const c_char,
                value.len() as size_t,
            ));
            Ok(())
        }
    }

    pub fn merge<K, V>(&mut self, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let key = key.as_ref();
        let value = value.as_ref();

        unsafe {
            ffi_try!(ffi::rocksdb_sstfilewriter_merge(
                self.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
                value.as_ptr() as *const c_char,
                value.len() as size_t,
            ));
            Ok(())
        }
    }

    pub fn delete<K: AsRef<[u8]>>(&mut self, key: K) -> Result<(), Error> {
        let key = key.as_ref();

        unsafe {
            ffi_try!(ffi::rocksdb_sstfilewriter_delete(
                self.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
            ));
            Ok(())
        }
    }
}

impl<'a> Drop for SstFileWriter<'a> {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_sstfilewriter_destroy(self.inner);
        }
    }
}