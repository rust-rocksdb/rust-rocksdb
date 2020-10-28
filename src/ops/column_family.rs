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

use crate::{ffi, handle::Handle, ColumnFamily, Error, Options};
use ambassador::delegatable_trait;
use std::collections::BTreeMap;
use std::ffi::CString;

#[delegatable_trait]
pub trait GetColumnFamily {
    fn cf_handle(&self, name: &str) -> Option<&ColumnFamily>;
}

#[delegatable_trait]
pub trait CreateColumnFamily {
    fn create_cf<N: AsRef<str>>(&mut self, name: N, opts: &Options) -> Result<(), Error>;
}

#[delegatable_trait]
pub trait DropColumnFamily {
    fn drop_cf(&mut self, name: &str) -> Result<(), Error>;
}

#[delegatable_trait]
pub trait GetColumnFamilies {
    fn get_cfs(&self) -> &BTreeMap<String, ColumnFamily>;

    fn get_mut_cfs(&mut self) -> &mut BTreeMap<String, ColumnFamily>;
}

impl<T> GetColumnFamily for T
where
    T: GetColumnFamilies,
{
    /// Return the underlying column family handle.
    fn cf_handle(&self, name: &str) -> Option<&ColumnFamily> {
        self.get_cfs().get(name)
    }
}

impl<T> CreateColumnFamily for T
where
    T: Handle<ffi::rocksdb_t> + GetColumnFamilies,
{
    fn create_cf<N: AsRef<str>>(&mut self, name: N, opts: &Options) -> Result<(), Error> {
        let cname = CString::new(name.as_ref().as_bytes()).map_err(|_| {
            Error::new(format!(
                "Failed to convert path to CString when creating cf: {}",
                name.as_ref()
            ))
        })?;
        unsafe {
            let inner = ffi_try!(ffi::rocksdb_create_column_family(
                self.handle(),
                opts.inner,
                cname.as_ptr(),
            ));

            self.get_mut_cfs()
                .insert(name.as_ref().to_string(), ColumnFamily { inner });
        };
        Ok(())
    }
}

impl<T> DropColumnFamily for T
where
    T: Handle<ffi::rocksdb_t> + GetColumnFamilies,
{
    fn drop_cf(&mut self, name: &str) -> Result<(), Error> {
        if let Some(cf) = self.get_mut_cfs().remove(name) {
            unsafe {
                ffi_try!(ffi::rocksdb_drop_column_family(self.handle(), cf.inner));
            }
            Ok(())
        } else {
            Err(Error::new(format!("Invalid column family: {}", name)))
        }
    }
}
