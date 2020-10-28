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

use crate::{
    ffi, ffi_util::to_cpath, handle::Handle, ColumnFamily, Error, IngestExternalFileOptions,
};
use ambassador::delegatable_trait;
use std::ffi::CString;
use std::path::Path;

#[delegatable_trait]
pub trait IngestExternalFile {
    /// Loads a list of external SST files created with SstFileWriter into the DB with default opts
    fn ingest_external_file<P: AsRef<Path>>(&self, paths: Vec<P>) -> Result<(), Error>;
}

#[delegatable_trait]
pub trait IngestExternalFileOpt {
    /// Loads a list of external SST files created with SstFileWriter into the DB
    fn ingest_external_file_opts<P: AsRef<Path>>(
        &self,
        opts: &IngestExternalFileOptions,
        paths: Vec<P>,
    ) -> Result<(), Error>;
}

#[delegatable_trait]
pub trait IngestExternalFileCF {
    /// Loads a list of external SST files created with SstFileWriter into
    /// the DB for given Column Family with default opts
    fn ingest_external_file_cf<P: AsRef<Path>>(
        &self,
        cf: &ColumnFamily,
        paths: Vec<P>,
    ) -> Result<(), Error>;
}

#[delegatable_trait]
pub trait IngestExternalFileCFOpt {
    /// Loads a list of external SST files created with SstFileWriter into
    /// the DB for given Column Family
    fn ingest_external_file_cf_opts<P: AsRef<Path>>(
        &self,
        cf: &ColumnFamily,
        opts: &IngestExternalFileOptions,
        paths: Vec<P>,
    ) -> Result<(), Error>;
}

impl<T> IngestExternalFile for T
where
    T: IngestExternalFileOpt,
{
    fn ingest_external_file<P: AsRef<Path>>(&self, paths: Vec<P>) -> Result<(), Error> {
        let opts = IngestExternalFileOptions::default();
        self.ingest_external_file_opts(&opts, paths)
    }
}

impl<T> IngestExternalFileCF for T
where
    T: IngestExternalFileCFOpt,
{
    fn ingest_external_file_cf<P: AsRef<Path>>(
        &self,
        cf: &ColumnFamily,
        paths: Vec<P>,
    ) -> Result<(), Error> {
        let opts = IngestExternalFileOptions::default();
        self.ingest_external_file_cf_opts(cf, &opts, paths)
    }
}

fn paths_to_cstr<P: AsRef<Path>>(paths: Vec<P>) -> Result<Vec<CString>, Error> {
    paths
        .iter()
        .map(|path| to_cpath(&path))
        .collect::<Result<Vec<_>, _>>()
}

impl<T> IngestExternalFileOpt for T
where
    T: Handle<ffi::rocksdb_t> + super::Write,
{
    fn ingest_external_file_opts<P: AsRef<Path>>(
        &self,
        opts: &IngestExternalFileOptions,
        paths: Vec<P>,
    ) -> Result<(), Error> {
        let paths_v = paths_to_cstr(paths)?;
        let cpaths: Vec<_> = paths_v.iter().map(|path| path.as_ptr()).collect();

        unsafe {
            ffi_try!(ffi::rocksdb_ingest_external_file(
                self.handle(),
                cpaths.as_ptr(),
                cpaths.len(),
                opts.inner as *const _
            ));
            Ok(())
        }
    }
}

impl<T> IngestExternalFileCFOpt for T
where
    T: Handle<ffi::rocksdb_t> + super::Write,
{
    fn ingest_external_file_cf_opts<P: AsRef<Path>>(
        &self,
        cf: &ColumnFamily,
        opts: &IngestExternalFileOptions,
        paths: Vec<P>,
    ) -> Result<(), Error> {
        let paths_v = paths_to_cstr(paths)?;
        let cpaths: Vec<_> = paths_v.iter().map(|path| path.as_ptr()).collect();

        unsafe {
            ffi_try!(ffi::rocksdb_ingest_external_file_cf(
                self.handle(),
                cf.inner,
                cpaths.as_ptr(),
                cpaths.len(),
                opts.inner as *const _
            ));
            Ok(())
        }
    }
}
