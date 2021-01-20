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
use libc::{c_char, c_void};
use std::ptr;
use std::slice;

use crate::{db::DBInner, ffi, ColumnFamily, Error, ReadOptions};

#[delegatable_trait]
pub trait MultiGet {
    /// Return the values associated with the given keys.
    fn multi_get<K, I>(&self, keys: I) -> Result<Vec<Vec<u8>>, Error>
    where
        K: AsRef<[u8]>,
        I: IntoIterator<Item = K>;
}

#[delegatable_trait]
pub trait MultiGetOpt<ReadOpts> {
    /// Return the values associated with the given keys using read options.
    fn multi_get_opt<K, I>(&self, keys: I, readopts: ReadOpts) -> Result<Vec<Vec<u8>>, Error>
    where
        K: AsRef<[u8]>,
        I: IntoIterator<Item = K>;
}

#[delegatable_trait]
pub trait MultiGetCF {
    /// Return the values associated with the given keys.
    fn multi_get_cf<'c, K, I>(&self, keys: I) -> Result<Vec<Vec<u8>>, Error>
    where
        K: AsRef<[u8]>,
        I: IntoIterator<Item = (&'c ColumnFamily, K)>;
}

#[delegatable_trait]
pub trait MultiGetCFOpt<ReadOpts> {
    /// Return the values associated with the given keys and column families
    /// using read options
    fn multi_get_cf_opt<'c, K, I>(
        &self,
        keys: I,
        readopts: ReadOpts,
    ) -> Result<Vec<Vec<u8>>, Error>
    where
        K: AsRef<[u8]>,
        I: IntoIterator<Item = (&'c ColumnFamily, K)>;
}

impl<T> MultiGet for T
where
    for<'a> T: MultiGetOpt<&'a ReadOptions>,
{
    fn multi_get<K, I>(&self, keys: I) -> Result<Vec<Vec<u8>>, Error>
    where
        K: AsRef<[u8]>,
        I: IntoIterator<Item = K>,
    {
        self.multi_get_opt(keys, &ReadOptions::default())
    }
}

impl MultiGetOpt<&ReadOptions> for DBInner {
    fn multi_get_opt<K, I>(&self, keys: I, readopts: &ReadOptions) -> Result<Vec<Vec<u8>>, Error>
    where
        K: AsRef<[u8]>,
        I: IntoIterator<Item = K>,
    {
        let (keys, keys_sizes): (Vec<Box<[u8]>>, Vec<_>) = keys
            .into_iter()
            .map(|k| (Box::from(k.as_ref()), k.as_ref().len()))
            .unzip();
        let ptr_keys: Vec<_> = keys.iter().map(|k| k.as_ptr() as *const c_char).collect();

        let mut values = vec![ptr::null_mut(); keys.len()];
        let mut values_sizes = vec![0_usize; keys.len()];
        unsafe {
            ffi_try!(ffi::rocksdb_multi_get(
                self.inner,
                readopts.inner,
                ptr_keys.len(),
                ptr_keys.as_ptr(),
                keys_sizes.as_ptr(),
                values.as_mut_ptr(),
                values_sizes.as_mut_ptr(),
            ));
        }

        Ok(convert_values(values, values_sizes))
    }
}

impl<T> MultiGetCF for T
where
    for<'a> T: MultiGetCFOpt<&'a ReadOptions>,
{
    fn multi_get_cf<'c, K, I>(&self, keys: I) -> Result<Vec<Vec<u8>>, Error>
    where
        K: AsRef<[u8]>,
        I: IntoIterator<Item = (&'c ColumnFamily, K)>,
    {
        self.multi_get_cf_opt(keys, &ReadOptions::default())
    }
}

impl MultiGetCFOpt<&ReadOptions> for DBInner {
    fn multi_get_cf_opt<'c, K, I>(&self, keys: I, readopts: &ReadOptions) -> Result<Vec<Vec<u8>>, Error>
    where
        K: AsRef<[u8]>,
        I: IntoIterator<Item = (&'c ColumnFamily, K)>,
    {
        let mut boxed_keys: Vec<Box<[u8]>> = Vec::new();
        let mut keys_sizes = Vec::new();
        let mut column_families = Vec::new();
        for (cf, key) in keys {
            boxed_keys.push(Box::from(key.as_ref()));
            keys_sizes.push(key.as_ref().len());
            column_families.push(cf);
        }
        let ptr_keys: Vec<_> = boxed_keys
            .iter()
            .map(|k| k.as_ptr() as *const c_char)
            .collect();
        let ptr_cfs: Vec<_> = column_families
            .iter()
            .map(|c| c.inner as *const _)
            .collect();

        let mut values = vec![ptr::null_mut(); boxed_keys.len()];
        let mut values_sizes = vec![0_usize; boxed_keys.len()];
        unsafe {
            ffi_try!(ffi::rocksdb_multi_get_cf(
                self.inner,
                readopts.inner,
                ptr_cfs.as_ptr(),
                ptr_keys.len(),
                ptr_keys.as_ptr(),
                keys_sizes.as_ptr(),
                values.as_mut_ptr(),
                values_sizes.as_mut_ptr(),
            ));
        }

        Ok(convert_values(values, values_sizes))
    }
}

fn convert_values(values: Vec<*mut c_char>, values_sizes: Vec<usize>) -> Vec<Vec<u8>> {
    values
        .into_iter()
        .zip(values_sizes.into_iter())
        .map(|(v, s)| {
            let value = unsafe { slice::from_raw_parts(v as *const u8, s) }.into();
            unsafe {
                ffi::rocksdb_free(v as *mut c_void);
            }
            value
        })
        .collect()
}
