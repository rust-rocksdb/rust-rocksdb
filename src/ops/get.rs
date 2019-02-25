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


use crate::{handle::Handle, ReadOptions, Error, DBVector, ColumnFamily};

pub trait Get<'a> {
    type ReadOptions;

    fn get_full<K: AsRef<[u8]>>(
        &'a self,
        key: K,
        readopts: Option<Self::ReadOptions>,
    ) -> Result<Option<DBVector>, Error>;

    fn get<K: AsRef<[u8]>>(&'a self, key: K) -> Result<Option<DBVector>, Error> {
        self.get_full(key, None)
    }

    fn get_opt<K: AsRef<[u8]>>(
        &'a self,
        key: K,
        readopts: Self::ReadOptions,
    ) -> Result<Option<DBVector>, Error> {
        self.get_full(key, Some(readopts))
    }
}

pub trait GetCF<'a> {
    type ColumnFamily;
    type ReadOptions;

    fn get_cf_full<K: AsRef<[u8]>>(
        &'a self,
        cf: Option<Self::ColumnFamily>,
        key: K,
        readopts: Option<Self::ReadOptions>,
    ) -> Result<Option<DBVector>, Error>;

    fn get_cf<K: AsRef<[u8]>>(
        &'a self,
        cf: Self::ColumnFamily,
        key: K,
    ) -> Result<Option<DBVector>, Error> {
        self.get_cf_full(Some(cf), key, None)
    }

    fn get_cf_opt<K: AsRef<[u8]>>(
        &'a self,
        cf: Self::ColumnFamily,
        key: K,
        readopts: Self::ReadOptions,
    ) -> Result<Option<DBVector>, Error> {
        self.get_cf_full(Some(cf), key, Some(readopts))
    }
}

impl<'a, T, R> Get<'a> for T
  where T: GetCF<'a, ReadOptions = R> {
      type ReadOptions = R;
    
      fn get_full<K: AsRef<[u8]>>(
          &'a self,
          key: K,
          readopts: Option<Self::ReadOptions>,
      ) -> Result<Option<DBVector>, Error> {
        self.get_cf_full(None, key, readopts)
      }
  }

impl<'a, T> GetCF<'a> for T
  where T: Handle<ffi::rocksdb_t> + super::Read {

    type ColumnFamily = ColumnFamily<'a>;
    type ReadOptions = &'a ReadOptions;

    fn get_cf_full<K: AsRef<[u8]>>(
        &'a self,
        cf: Option<Self::ColumnFamily>,
        key: K,
        readopts: Option<Self::ReadOptions>,
    ) -> Result<Option<DBVector>, Error> {

        let mut default_readopts = None;

        if readopts.is_none() {
            default_readopts.replace(ReadOptions::default());
        }

        let ro_handle = readopts
            .or(default_readopts.as_ref())
            .map(|r| r.inner)
            .ok_or(Error::new("Unable to extract read options.".to_string()))?;

        if ro_handle.is_null() {
            return Err(Error::new(
                "Unable to create RocksDB read options. \
                 This is a fairly trivial call, and its \
                 failure may be indicative of a \
                 mis-compiled or mis-loaded RocksDB \
                 library."
                    .to_string(),
            ));
        }

        let key = key.as_ref();
        let key_ptr = key.as_ptr() as *const c_char;
        let key_len = key.len() as size_t;

        unsafe {
            let mut val_len: size_t = 0;

            let val = match cf {
                Some(cf) => ffi_try!(ffi::rocksdb_get_cf(
                    self.handle(),
                    ro_handle,
                    cf.inner,
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
                ))
            } as *mut u8;
                
            if val.is_null() {
                Ok(None)
            } else {
                Ok(Some(DBVector::from_c(val, val_len)))
            }
        }
    }
}