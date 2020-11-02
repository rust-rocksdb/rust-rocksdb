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

use libc::{c_char, size_t};

use crate::{db::DBInner, ffi, handle::Handle, ColumnFamily, Error, WriteOptions};
use ambassador::delegatable_trait;

#[delegatable_trait]
pub trait Delete {
    fn delete<K: AsRef<[u8]>>(&self, key: K) -> Result<(), Error>;
}

#[delegatable_trait]
pub trait DeleteOpt {
    fn delete_opt<K: AsRef<[u8]>>(&self, key: K, writeopts: &WriteOptions) -> Result<(), Error>;
}

#[delegatable_trait]
pub trait DeleteCF {
    fn delete_cf<K: AsRef<[u8]>>(&self, cf: &ColumnFamily, key: K) -> Result<(), Error>;
}

#[delegatable_trait]
pub trait DeleteCFOpt {
    fn delete_cf_opt<K: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        key: K,
        writeopts: &WriteOptions,
    ) -> Result<(), Error>;
}

impl<T> Delete for T
where
    T: DeleteOpt,
{
    fn delete<K: AsRef<[u8]>>(&self, key: K) -> Result<(), Error> {
        self.delete_opt(key, &WriteOptions::default())
    }
}

impl DeleteOpt for DBInner {
    fn delete_opt<K: AsRef<[u8]>>(&self, key: K, writeopts: &WriteOptions) -> Result<(), Error> {
        let key = key.as_ref();

        unsafe {
            ffi_try!(ffi::rocksdb_delete(
                self.handle(),
                writeopts.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
            ));
            Ok(())
        }
    }
}

impl<T> DeleteCF for T
where
    T: DeleteCFOpt,
{
    fn delete_cf<K: AsRef<[u8]>>(&self, cf: &ColumnFamily, key: K) -> Result<(), Error> {
        self.delete_cf_opt(cf, key, &WriteOptions::default())
    }
}

impl DeleteCFOpt for DBInner {
    fn delete_cf_opt<K: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        key: K,
        writeopts: &WriteOptions,
    ) -> Result<(), Error> {
        let key = key.as_ref();

        unsafe {
            ffi_try!(ffi::rocksdb_delete_cf(
                self.handle(),
                writeopts.inner,
                cf.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
            ));
            Ok(())
        }
    }
}
