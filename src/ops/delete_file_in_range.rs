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

use crate::{ffi, handle::Handle, ColumnFamily, Error};
use ambassador::delegatable_trait;
use libc::{c_char, size_t};

#[delegatable_trait]
pub trait DeleteFileInRange {
    /// Delete sst files whose keys are entirely in the given range.
    ///
    /// Could leave some keys in the range which are in files which are not
    /// entirely in the range.
    ///
    /// Note: L0 files are left regardless of whether they're in the range.
    ///
    /// Snapshots before the delete might not see the data in the given range.
    fn delete_file_in_range<B: AsRef<[u8]>, E: AsRef<[u8]>>(
        &self,
        begin: B,
        end: E,
    ) -> Result<(), Error>;
}

#[delegatable_trait]
pub trait DeleteFileInRangeCF {
    /// Same as `delete_file_in_range` but only for specific column family
    fn delete_file_in_range_cf<B: AsRef<[u8]>, E: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        begin: B,
        end: E,
    ) -> Result<(), Error>;
}

impl<T> DeleteFileInRange for T
where
    T: Handle<ffi::rocksdb_t> + super::Write,
{
    fn delete_file_in_range<B: AsRef<[u8]>, E: AsRef<[u8]>>(
        &self,
        begin: B,
        end: E,
    ) -> Result<(), Error> {
        let begin = begin.as_ref();
        let end = end.as_ref();

        unsafe {
            ffi_try!(ffi::rocksdb_delete_file_in_range(
                self.handle(),
                begin.as_ptr() as *const c_char,
                begin.len() as size_t,
                end.as_ptr() as *const c_char,
                end.len() as size_t,
            ));
            Ok(())
        }
    }
}
impl<T> DeleteFileInRangeCF for T
where
    T: Handle<ffi::rocksdb_t> + super::Write,
{
    fn delete_file_in_range_cf<B: AsRef<[u8]>, E: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        begin: B,
        end: E,
    ) -> Result<(), Error> {
        let begin = begin.as_ref();
        let end = end.as_ref();

        unsafe {
            ffi_try!(ffi::rocksdb_delete_file_in_range_cf(
                self.handle(),
                cf.inner,
                begin.as_ptr() as *const c_char,
                begin.len() as size_t,
                end.as_ptr() as *const c_char,
                end.len() as size_t,
            ));
            Ok(())
        }
    }
}
