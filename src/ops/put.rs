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

pub trait Put<'a> {
    type WriteOptions;

    fn put_full<K, V>(
        &'a self,
        key: K,
        value: V,
        writeopts: Option<Self::WriteOptions>,
    ) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>;

    fn put<K, V>(&'a self, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.put_full(key, value, None)
    }

    fn put_opt<K, V>(&'a self, key: K, value: V, writeopts: Self::WriteOptions) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.put_full(key, value, Some(writeopts))
    }
}

pub trait PutCF<'a> {
    type ColumnFamily;
    type WriteOptions;

    fn put_cf_full<K, V>(
        &'a self,
        cf: Option<Self::ColumnFamily>,
        key: K,
        value: V,
        writeopts: Option<Self::WriteOptions>,
    ) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>;

    fn put_cf<K, V>(&'a self, cf: Self::ColumnFamily, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.put_cf_full(Some(cf), key, value, None)
    }

    fn put_cf_opt<K, V>(
        &'a self,
        cf: Self::ColumnFamily,
        key: K,
        value: V,
        writeopts: Self::WriteOptions,
    ) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.put_cf_full(Some(cf), key, value, Some(writeopts))
    }
}

impl<'a, T, W> Put<'a> for T
where
    T: PutCF<'a, WriteOptions = W>,
{
    type WriteOptions = W;

    fn put_full<K: AsRef<[u8]>, V: AsRef<[u8]>>(
        &'a self,
        key: K,
        value: V,
        writeopts: Option<Self::WriteOptions>,
    ) -> Result<(), Error> {
        self.put_cf_full(None, key, value, writeopts)
    }
}

impl<'a, T> PutCF<'a> for T
where
    T: Handle<ffi::rocksdb_t> + super::Write,
{
    type ColumnFamily = &'a ColumnFamily;
    type WriteOptions = &'a WriteOptions;

    fn put_cf_full<K, V>(
        &'a self,
        cf: Option<Self::ColumnFamily>,
        key: K,
        value: V,
        writeopts: Option<Self::WriteOptions>,
    ) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let mut default_writeopts = None;

        if default_writeopts.is_none() {
            default_writeopts.replace(WriteOptions::default());
        }

        let wo_handle = writeopts
            .or_else(|| default_writeopts.as_ref())
            .map(|r| r.inner)
            .ok_or_else(|| Error::new("Unable to extract write options.".to_string()))?;

        let key = key.as_ref();
        let value = value.as_ref();
        let key_ptr = key.as_ptr() as *const c_char;
        let key_len = key.len() as size_t;
        let val_ptr = value.as_ptr() as *const c_char;
        let val_len = value.len() as size_t;

        unsafe {
            match cf {
                Some(cf) => ffi_try!(ffi::rocksdb_put_cf(
                    self.handle(),
                    wo_handle,
                    cf.inner,
                    key_ptr,
                    key_len,
                    val_ptr,
                    val_len,
                )),
                None => ffi_try!(ffi::rocksdb_put(
                    self.handle(),
                    wo_handle,
                    key_ptr,
                    key_len,
                    val_ptr,
                    val_len,
                )),
            }

            Ok(())
        }
    }
}
