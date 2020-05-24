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

use crate::{ffi, handle::Handle, ColumnFamily, Error, FlushOptions};

pub trait Flush {
    /// Flush database memtable to SST files on the disk using default options.
    fn flush(&self) -> Result<(), Error>;
}

pub trait FlushOpt {
    /// Flush database memtable to SST files on disk.
    fn flush_opt(&self, flushopts: &FlushOptions) -> Result<(), Error>;
}

pub trait FlushCF {
    /// Flush database memtable to SST files on disk for a given column family
    /// using default options.
    fn flush_cf(&self, cf: &ColumnFamily) -> Result<(), Error>;
}

pub trait FlushCFOpt {
    /// Flush database memtable to SST files on disk for a given column family.
    fn flush_cf_opt(&self, cf: &ColumnFamily, flushopts: &FlushOptions) -> Result<(), Error>;
}

impl<T> Flush for T
where
    T: FlushOpt,
{
    fn flush(&self) -> Result<(), Error> {
        self.flush_opt(&FlushOptions::default())
    }
}

impl<T> FlushOpt for T
where
    T: Handle<ffi::rocksdb_t> + super::Write,
{
    fn flush_opt(&self, flushopts: &FlushOptions) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_flush(self.handle(), flushopts.inner,));
        }
        Ok(())
    }
}

impl<T> FlushCF for T
where
    T: FlushCFOpt,
{
    fn flush_cf(&self, cf: &ColumnFamily) -> Result<(), Error> {
        self.flush_cf_opt(cf, &FlushOptions::default())
    }
}

impl<T> FlushCFOpt for T
where
    T: Handle<ffi::rocksdb_t> + super::Write,
{
    fn flush_cf_opt(&self, cf: &ColumnFamily, flushopts: &FlushOptions) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_flush_cf(
                self.handle(),
                flushopts.inner,
                cf.inner,
            ));
        }
        Ok(())
    }
}
