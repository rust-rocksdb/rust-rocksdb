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

pub trait Delete<'a> {
    type WriteOptions;

    fn delete_full<K>(&'a self, key: K, writeopts: Option<Self::WriteOptions>) -> Result<(), Error>
    where
        K: AsRef<[u8]>;

    fn delete<K>(&'a self, key: K) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
    {
        self.delete_full(key, None)
    }

    fn delete_opt<K>(&'a self, key: K, writeopts: Self::WriteOptions) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
    {
        self.delete_full(key, Some(writeopts))
    }
}

pub trait DeleteCF<'a> {
    type ColumnFamily;
    type WriteOptions;

    fn delete_cf_full<K>(
        &'a self,
        cf: Option<Self::ColumnFamily>,
        key: K,
        writeopts: Option<Self::WriteOptions>,
    ) -> Result<(), Error>
    where
        K: AsRef<[u8]>;

    fn delete_cf<K>(&'a self, cf: Self::ColumnFamily, key: K) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
    {
        self.delete_cf_full(Some(cf), key, None)
    }

    fn put_cf_opt<K>(
        &'a self,
        cf: Self::ColumnFamily,
        key: K,
        writeopts: Self::WriteOptions,
    ) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
    {
        self.delete_cf_full(Some(cf), key, Some(writeopts))
    }
}

impl<'a, T, W> Delete<'a> for T
where
    T: DeleteCF<'a, WriteOptions = W>,
{
    type WriteOptions = W;

    fn delete_full<K: AsRef<[u8]>>(
        &'a self,
        key: K,
        writeopts: Option<Self::WriteOptions>,
    ) -> Result<(), Error> {
        self.delete_cf_full(None, key, writeopts)
    }
}

impl<'a, T> DeleteCF<'a> for T
where
    T: Handle<ffi::rocksdb_t> + super::Write,
{
    type ColumnFamily = &'a ColumnFamily;
    type WriteOptions = &'a WriteOptions;

    fn delete_cf_full<K>(
        &'a self,
        cf: Option<Self::ColumnFamily>,
        key: K,
        writeopts: Option<Self::WriteOptions>,
    ) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
    {
        let mut default_writeopts = None;

        if default_writeopts.is_none() {
            default_writeopts.replace(WriteOptions::default());
        }

        let wo_handle = writeopts
            .or(default_writeopts.as_ref())
            .map(|r| r.inner)
            .ok_or(Error::new("Unable to extract write options.".to_string()))?;

        let key = key.as_ref();
        let key_ptr = key.as_ptr() as *const c_char;
        let key_len = key.len() as size_t;

        unsafe {
            match cf {
                Some(cf) => ffi_try!(ffi::rocksdb_delete_cf(
                    self.handle(),
                    wo_handle,
                    cf.inner,
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
