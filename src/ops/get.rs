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

use ambassador::delegatable_trait;

use crate::{
    ops::{GetPinnedCFOpt, GetPinnedOpt},
    ColumnFamily, Error, ReadOptions,
};

#[delegatable_trait]
pub trait Get {
    /// Return the bytes associated with a key value.
    /// If you only intend to use the vector returned temporarily, consider
    /// using [`get_pinned`](#method.get_pinned) to avoid unnecessary memory copy.
    fn get<K: AsRef<[u8]>>(&self, key: K) -> Result<Option<Vec<u8>>, Error>;
}

#[delegatable_trait]
pub trait GetOpt<ReadOpts> {
    /// Return the bytes associated with a key value with read options.
    /// If you only intend to use the vector returned temporarily, consider
    /// using [`get_pinned_opt`](#method.get_pinned_opt) to avoid unnecessary memory copy.
    fn get_opt<K: AsRef<[u8]>>(&self, key: K, readopts: ReadOpts)
        -> Result<Option<Vec<u8>>, Error>;
}

#[delegatable_trait]
pub trait GetCF {
    /// Return the bytes associated with a key value and the given column family.
    /// If you only  intend to use the vector returned temporarily, consider using
    /// [`get_pinned_cf`](#method.get_pinned_cf) to avoid unnecessary memory.
    fn get_cf<K: AsRef<[u8]>>(&self, cf: &ColumnFamily, key: K) -> Result<Option<Vec<u8>>, Error>;
}

#[delegatable_trait]
pub trait GetCFOpt<ReadOpts> {
    /// Return the bytes associated with a key value and the given column
    /// family with read options. If you only intend to use the vector returned
    /// temporarily, consider using [`get_pinned_cf_opt`](#method.get_pinned_cf_opt)
    /// to avoid unnecessary memory.
    fn get_cf_opt<K: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        key: K,
        readopts: ReadOpts,
    ) -> Result<Option<Vec<u8>>, Error>;
}

impl<T> Get for T
where
    for<'a> T: GetOpt<&'a ReadOptions>,
{
    fn get<K: AsRef<[u8]>>(&self, key: K) -> Result<Option<Vec<u8>>, Error> {
        self.get_opt(key, &ReadOptions::default())
    }
}

impl<T> GetOpt<&ReadOptions> for T
where
    T: GetPinnedOpt,
{
    fn get_opt<K: AsRef<[u8]>>(
        &self,
        key: K,
        readopts: &ReadOptions,
    ) -> Result<Option<Vec<u8>>, Error> {
        self.get_pinned_opt(key, readopts)
            .map(|x| x.map(|v| v.as_ref().to_vec()))
    }
}

impl<T> GetCF for T
where
    for<'a> T: GetCFOpt<&'a ReadOptions>,
{
    fn get_cf<K: AsRef<[u8]>>(&self, cf: &ColumnFamily, key: K) -> Result<Option<Vec<u8>>, Error> {
        self.get_cf_opt(cf, key, &ReadOptions::default())
    }
}

impl<T> GetCFOpt<&ReadOptions> for T
where
    T: GetPinnedCFOpt,
{
    fn get_cf_opt<K: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        key: K,
        readopts: &ReadOptions,
    ) -> Result<Option<Vec<u8>>, Error> {
        self.get_pinned_cf_opt(cf, key, readopts)
            .map(|x| x.map(|v| v.as_ref().to_vec()))
    }
}
