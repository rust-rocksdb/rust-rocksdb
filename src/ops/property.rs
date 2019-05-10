use crate::{handle::Handle, ColumnFamily, Error};
use libc::c_void;
use std::ffi::{CStr, CString};

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
        match self.property_value(name) {
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
}

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
        match self.property_value_cf(cf, name) {
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
}

impl<T> GetProperty for T
where
    T: Handle<ffi::rocksdb_t>,
{
    fn property_value(&self, name: &str) -> Result<Option<String>, Error> {
        let prop_name = match CString::new(name) {
            Ok(c) => c,
            Err(e) => {
                return Err(Error::new(format!(
                    "Failed to convert property name to CString: {}",
                    e
                )));
            }
        };

        unsafe {
            let value = ffi::rocksdb_property_value(self.handle(), prop_name.as_ptr());
            if value.is_null() {
                return Ok(None);
            }

            let str_value = match CStr::from_ptr(value).to_str() {
                Ok(s) => s.to_owned(),
                Err(e) => {
                    return Err(Error::new(format!(
                        "Failed to convert property value to string: {}",
                        e
                    )));
                }
            };

            ffi::rocksdb_free(value as *mut c_void);
            Ok(Some(str_value))
        }
    }
}

impl<T> GetPropertyCF for T
where
    T: Handle<ffi::rocksdb_t>,
{
    fn property_value_cf(&self, cf: &ColumnFamily, name: &str) -> Result<Option<String>, Error> {
        let prop_name = match CString::new(name) {
            Ok(c) => c,
            Err(e) => {
                return Err(Error::new(format!(
                    "Failed to convert property name to CString: {}",
                    e
                )));
            }
        };

        unsafe {
            let value = ffi::rocksdb_property_value_cf(self.handle(), cf.inner, prop_name.as_ptr());
            if value.is_null() {
                return Ok(None);
            }

            let str_value = match CStr::from_ptr(value).to_str() {
                Ok(s) => s.to_owned(),
                Err(e) => {
                    return Err(Error::new(format!(
                        "Failed to convert property value to string: {}",
                        e
                    )));
                }
            };

            libc::free(value as *mut c_void);
            Ok(Some(str_value))
        }
    }
}
