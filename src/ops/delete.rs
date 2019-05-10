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
use libc::{c_char, size_t};

use crate::{handle::Handle, ColumnFamily, Error, WriteOptions};

pub trait Delete<W> {
    fn delete_full<K>(&self, key: K, writeopts: Option<&W>) -> Result<(), Error>
    where
        K: AsRef<[u8]>;

    /// Remove the database entry for key.
    ///
    /// Returns an error if the key was not found.
    fn delete<K>(&self, key: K) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
    {
        self.delete_full(key, None)
    }

    fn delete_opt<K>(&self, key: K, writeopts: &W) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
    {
        self.delete_full(key, Some(writeopts))
    }
}

pub trait DeleteCF<W> {
    fn delete_cf_full<K>(
        &self,
        cf: Option<&ColumnFamily>,
        key: K,
        writeopts: Option<&W>,
    ) -> Result<(), Error>
    where
        K: AsRef<[u8]>;

    fn delete_cf<K>(&self, cf: &ColumnFamily, key: K) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
    {
        self.delete_cf_full(Some(cf), key, None)
    }

    fn put_cf_opt<K>(&self, cf: &ColumnFamily, key: K, writeopts: &W) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
    {
        self.delete_cf_full(Some(cf), key, Some(writeopts))
    }
}

impl<T, W> Delete<W> for T
where
    T: DeleteCF<W>,
{
    fn delete_full<K: AsRef<[u8]>>(&self, key: K, writeopts: Option<&W>) -> Result<(), Error> {
        self.delete_cf_full(None, key, writeopts)
    }
}

impl<T> DeleteCF<WriteOptions> for T
where
    T: Handle<ffi::rocksdb_t> + super::Write,
{
    fn delete_cf_full<K>(
        &self,
        cf: Option<&ColumnFamily>,
        key: K,
        writeopts: Option<&WriteOptions>,
    ) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
    {
        let mut default_writeopts = None;

        let wo_handle = WriteOptions::input_or_default(writeopts, &mut default_writeopts)?;

        let key = key.as_ref();
        let key_ptr = key.as_ptr() as *const c_char;
        let key_len = key.len() as size_t;

        unsafe {
            match cf {
                Some(cf) => ffi_try!(ffi::rocksdb_delete_cf(
                    self.handle(),
                    wo_handle,
                    cf.handle(),
                    key_ptr,
                    key_len,
                )),
                None => ffi_try!(ffi::rocksdb_delete(
                    self.handle(),
                    wo_handle,
                    key_ptr,
                    key_len,
                )),
            }

            Ok(())
        }
    }
}
