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

use crate::{handle::Handle, ColumnFamily, DBPinnableSlice, Error, ReadOptions};

pub trait GetPinned<'a> {
    type ReadOptions;

    /// Return the value associated with a key using RocksDB's PinnableSlice
    /// so as to avoid unnecessary memory copy.
    fn get_pinned_full<K: AsRef<[u8]>>(
        &'a self,
        key: K,
        readopts: Option<Self::ReadOptions>,
    ) -> Result<Option<DBPinnableSlice<'a>>, Error>;

    /// Return the value associated with a key using RocksDB's PinnableSlice
    /// so as to avoid unnecessary memory copy. Similar to get_pinned_opt but
    /// leverages default options.
    fn get_pinned<K: AsRef<[u8]>>(&'a self, key: K) -> Result<Option<DBPinnableSlice<'a>>, Error> {
        self.get_pinned_full(key, None)
    }

    /// Return the value associated with a key using RocksDB's PinnableSlice
    /// so as to avoid unnecessary memory copy.
    fn get_pinned_opt<K: AsRef<[u8]>>(
        &'a self,
        key: K,
        readopts: Self::ReadOptions,
    ) -> Result<Option<DBPinnableSlice<'a>>, Error> {
        self.get_pinned_full(key, Some(readopts))
    }
}

pub trait GetPinnedCF<'a> {
    type ColumnFamily;
    type ReadOptions;

    /// Return the value associated with a key using RocksDB's PinnableSlice
    /// so as to avoid unnecessary memory copy.
    fn get_pinned_cf_full<K: AsRef<[u8]>>(
        &'a self,
        cf: Option<Self::ColumnFamily>,
        key: K,
        readopts: Option<Self::ReadOptions>,
    ) -> Result<Option<DBPinnableSlice<'a>>, Error>;

    /// Return the value associated with a key using RocksDB's PinnableSlice
    /// so as to avoid unnecessary memory copy.
    fn get_pinned_cf<K: AsRef<[u8]>>(
        &'a self,
        cf: Self::ColumnFamily,
        key: K,
    ) -> Result<Option<DBPinnableSlice<'a>>, Error> {
        self.get_pinned_cf_full(Some(cf), key, None)
    }

    /// Return the value associated with a key using RocksDB's PinnableSlice
    /// so as to avoid unnecessary memory copy.
    fn get_pinned_cf_opt<K: AsRef<[u8]>>(
        &'a self,
        cf: Self::ColumnFamily,
        key: K,
        readopts: Self::ReadOptions,
    ) -> Result<Option<DBPinnableSlice<'a>>, Error> {
        self.get_pinned_cf_full(Some(cf), key, Some(readopts))
    }
}

impl<'a, T, R> GetPinned<'a> for T
where
    T: GetPinnedCF<'a, ReadOptions = R>,
{
    type ReadOptions = R;

    fn get_pinned_full<K: AsRef<[u8]>>(
        &'a self,
        key: K,
        readopts: Option<Self::ReadOptions>,
    ) -> Result<Option<DBPinnableSlice<'a>>, Error> {
        self.get_pinned_cf_full(None, key, readopts)
    }
}

impl<'a, T> GetPinnedCF<'a> for T
where
    T: Handle<ffi::rocksdb_t> + super::Read,
{
    type ColumnFamily = &'a ColumnFamily;
    type ReadOptions = &'a ReadOptions;

    fn get_pinned_cf_full<K: AsRef<[u8]>>(
        &'a self,
        cf: Option<Self::ColumnFamily>,
        key: K,
        readopts: Option<Self::ReadOptions>,
    ) -> Result<Option<DBPinnableSlice<'a>>, Error> {
        let mut default_readopts = None;

        let ro_handle = ReadOptions::input_or_default(readopts, &mut default_readopts)?;

        let key = key.as_ref();
        let key_ptr = key.as_ptr() as *const c_char;
        let key_len = key.len() as size_t;

        unsafe {
            let val = match cf {
                Some(cf) => ffi_try!(ffi::rocksdb_get_pinned_cf(
                    self.handle(),
                    ro_handle,
                    cf.inner,
                    key_ptr,
                    key_len,
                )),
                None => ffi_try!(ffi::rocksdb_get_pinned(
                    self.handle(),
                    ro_handle,
                    key_ptr,
                    key_len,
                )),
            };

            if val.is_null() {
                Ok(None)
            } else {
                Ok(Some(DBPinnableSlice::from_c(val)))
            }
        }
    }
}
