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

use crate::{handle::Handle, ColumnFamily, DBVector, Error, ReadOptions};

pub trait Get<R> {
    fn get_full<K: AsRef<[u8]>>(
        &self,
        key: K,
        readopts: Option<&R>,
    ) -> Result<Option<DBVector>, Error>;

    /// Return the bytes associated with a key value
    fn get<K: AsRef<[u8]>>(&self, key: K) -> Result<Option<DBVector>, Error> {
        self.get_full(key, None)
    }

    fn get_opt<K: AsRef<[u8]>>(&self, key: K, readopts: &R) -> Result<Option<DBVector>, Error> {
        self.get_full(key, Some(readopts))
    }
}

pub trait GetCF<R> {
    fn get_cf_full<K: AsRef<[u8]>>(
        &self,
        cf: Option<&ColumnFamily>,
        key: K,
        readopts: Option<&R>,
    ) -> Result<Option<DBVector>, Error>;

    fn get_cf<K: AsRef<[u8]>>(&self, cf: &ColumnFamily, key: K) -> Result<Option<DBVector>, Error> {
        self.get_cf_full(Some(cf), key, None)
    }

    fn get_cf_opt<K: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        key: K,
        readopts: &R,
    ) -> Result<Option<DBVector>, Error> {
        self.get_cf_full(Some(cf), key, Some(readopts))
    }
}

impl<T, R> Get<R> for T
where
    T: GetCF<R>,
{
    fn get_full<K: AsRef<[u8]>>(
        &self,
        key: K,
        readopts: Option<&R>,
    ) -> Result<Option<DBVector>, Error> {
        self.get_cf_full(None, key, readopts)
    }
}

impl<T> GetCF<ReadOptions> for T
where
    T: Handle<ffi::rocksdb_t> + super::Read,
{
    fn get_cf_full<K: AsRef<[u8]>>(
        &self,
        cf: Option<&ColumnFamily>,
        key: K,
        readopts: Option<&ReadOptions>,
    ) -> Result<Option<DBVector>, Error> {
        let mut default_readopts = None;

        let ro_handle = ReadOptions::input_or_default(readopts, &mut default_readopts)?;

        let key = key.as_ref();
        let key_ptr = key.as_ptr() as *const c_char;
        let key_len = key.len() as size_t;

        unsafe {
            let mut val_len: size_t = 0;

            let val = match cf {
                Some(cf) => ffi_try!(ffi::rocksdb_get_cf(
                    self.handle(),
                    ro_handle,
                    cf.handle(),
                    key_ptr,
                    key_len,
                    &mut val_len,
                )),
                None => ffi_try!(ffi::rocksdb_get(
                    self.handle(),
                    ro_handle,
                    key_ptr,
                    key_len,
                    &mut val_len,
                )),
            } as *mut u8;

            if val.is_null() {
                Ok(None)
            } else {
                Ok(Some(DBVector::from_c(val, val_len)))
            }
        }
    }
}
