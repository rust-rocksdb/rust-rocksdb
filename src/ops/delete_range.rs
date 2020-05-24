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

use crate::{ffi, handle::Handle, ColumnFamily, Error, WriteOptions};
use libc::{c_char, size_t};

pub trait DeleteRangeCF {
    fn delete_range_cf<B: AsRef<[u8]>, E: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        begin: B,
        end: E,
    ) -> Result<(), Error>;
}

pub trait DeleteRangeCFOpt {
    /// Removes the database entries in the range `["begin", "end")`
    /// using given write options.
    fn delete_range_cf_opt<B: AsRef<[u8]>, E: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        begin: B,
        end: E,
        writeopts: &WriteOptions,
    ) -> Result<(), Error>;
}

impl<T> DeleteRangeCF for T
where
    T: DeleteRangeCFOpt,
{
    fn delete_range_cf<B: AsRef<[u8]>, E: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        begin: B,
        end: E,
    ) -> Result<(), Error> {
        self.delete_range_cf_opt(cf, begin, end, &WriteOptions::default())
    }
}

impl<T> DeleteRangeCFOpt for T
where
    T: Handle<ffi::rocksdb_t> + super::Write,
{
    fn delete_range_cf_opt<B: AsRef<[u8]>, E: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        begin: B,
        end: E,
        writeopts: &WriteOptions,
    ) -> Result<(), Error> {
        let begin = begin.as_ref();
        let end = end.as_ref();

        unsafe {
            ffi_try!(ffi::rocksdb_delete_range_cf(
                self.handle(),
                writeopts.inner,
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
