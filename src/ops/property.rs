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

use crate::{db::DBInner, ffi, handle::Handle, ColumnFamily, Error};
use ambassador::delegatable_trait;
use libc::{c_char, c_void};
use std::ffi::{CStr, CString};

#[delegatable_trait]
pub trait GetProperty {
    /// Retrieves a RocksDB property by name.
    ///
    /// For a full list of properties, see
    /// https://github.com/facebook/rocksdb/blob/08809f5e6cd9cc4bc3958dd4d59457ae78c76660/include/rocksdb/db.h#L428-L634
    fn property_value(&self, name: &str) -> Result<Option<String>, Error>;

    /// Retrieves a RocksDB property and casts it to an integer.
    ///
    /// For a full list of properties that return int values, see
    /// https://github.com/facebook/rocksdb/blob/08809f5e6cd9cc4bc3958dd4d59457ae78c76660/include/rocksdb/db.h#L654-L689
    fn property_int_value(&self, name: &str) -> Result<Option<u64>, Error> {
        property_int_value(self.property_value(name))
    }
}

#[delegatable_trait]
pub trait GetPropertyCF {
    /// Retrieves a RocksDB property by name, for a specific column family.
    ///
    /// For a full list of properties, see
    /// https://github.com/facebook/rocksdb/blob/08809f5e6cd9cc4bc3958dd4d59457ae78c76660/include/rocksdb/db.h#L428-L634
    fn property_value_cf(&self, cf: &ColumnFamily, name: &str) -> Result<Option<String>, Error>;

    /// Retrieves a RocksDB property for a specific column family and casts it to an integer.
    ///
    /// For a full list of properties that return int values, see
    /// https://github.com/facebook/rocksdb/blob/08809f5e6cd9cc4bc3958dd4d59457ae78c76660/include/rocksdb/db.h#L654-L689
    fn property_int_value_cf(&self, cf: &ColumnFamily, name: &str) -> Result<Option<u64>, Error> {
        property_int_value(self.property_value_cf(cf, name))
    }
}

#[inline]
fn property_int_value(value: Result<Option<String>, Error>) -> Result<Option<u64>, Error> {
    match value {
        Ok(Some(value)) => match value.parse::<u64>() {
            Ok(int_value) => Ok(Some(int_value)),
            Err(e) => Err(Error::new(format!(
                "Failed to convert property value to int: {}",
                e
            ))),
        },
        Ok(None) => Ok(None),
        Err(e) => Err(e),
    }
}

#[inline]
fn property_value<F>(get_value: F, name: &str) -> Result<Option<String>, Error>
where
    F: Fn(CString) -> *const c_char,
{
    let prop_name = CString::new(name)
        .map_err(|e| Error::new(format!("Failed to convert property name to CString: {}", e)))?;

    unsafe {
        let value = get_value(prop_name);
        if value.is_null() {
            return Ok(None);
        }

        let str_value = CStr::from_ptr(value)
            .to_str()
            .map(ToOwned::to_owned)
            .map_err(|e| {
                Error::new(format!("Failed to convert property value to string: {}", e))
            })?;

        libc::free(value as *mut c_void);
        Ok(Some(str_value))
    }
}

impl GetProperty for DBInner {
    fn property_value(&self, name: &str) -> Result<Option<String>, Error> {
        property_value(
            |prop_name| unsafe { ffi::rocksdb_property_value(self.handle(), prop_name.as_ptr()) },
            name,
        )
    }
}

impl GetPropertyCF for DBInner {
    fn property_value_cf(&self, cf: &ColumnFamily, name: &str) -> Result<Option<String>, Error> {
        property_value(
            |prop_name| unsafe {
                ffi::rocksdb_property_value_cf(self.handle(), cf.inner, prop_name.as_ptr())
            },
            name,
        )
    }
}
