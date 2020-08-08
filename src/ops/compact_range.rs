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

use crate::{ffi, ffi_util::opt_bytes_to_ptr, handle::Handle, ColumnFamily, CompactOptions};
use ambassador::delegatable_trait;
use libc::size_t;

#[delegatable_trait]
pub trait CompactRange {
    /// Runs a manual compaction on the Range of keys given.
    /// This us bit likely to be needed for typical usage.
    fn compact_range<B: AsRef<[u8]>, E: AsRef<[u8]>>(&self, begin: Option<B>, end: Option<E>);
}

#[delegatable_trait]
pub trait CompactRangeOpt {
    /// Same as `compact_range` but with custom options.
    fn compact_range_opt<B: AsRef<[u8]>, E: AsRef<[u8]>>(
        &self,
        begin: Option<B>,
        end: Option<E>,
        opts: &CompactOptions,
    );
}

#[delegatable_trait]
pub trait CompactRangeCF {
    /// Runs a manual compaction on the Range of keys given on the given column family.
    /// This us bit likely to be needed for typical usage.
    fn compact_range_cf<B: AsRef<[u8]>, E: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        begin: Option<B>,
        end: Option<E>,
    );
}

#[delegatable_trait]
pub trait CompactRangeCFOpt {
    /// Same as `compact_range_cf` but with custom options.
    fn compact_range_cf_opt<B: AsRef<[u8]>, E: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        begin: Option<B>,
        end: Option<E>,
        opts: &CompactOptions,
    );
}

impl<T> CompactRange for T
where
    T: CompactRangeOpt,
{
    fn compact_range<B: AsRef<[u8]>, E: AsRef<[u8]>>(&self, begin: Option<B>, end: Option<E>) {
        self.compact_range_opt(begin, end, &CompactOptions::default())
    }
}

impl<T> CompactRangeOpt for T
where
    T: Handle<ffi::rocksdb_t> + super::Write,
{
    fn compact_range_opt<B: AsRef<[u8]>, E: AsRef<[u8]>>(
        &self,
        begin: Option<B>,
        end: Option<E>,
        opts: &CompactOptions,
    ) {
        unsafe {
            let begin = begin.as_ref().map(AsRef::as_ref);
            let end = end.as_ref().map(AsRef::as_ref);

            ffi::rocksdb_compact_range_opt(
                self.handle(),
                opts.inner,
                opt_bytes_to_ptr(begin),
                begin.map_or(0, |s| s.len()) as size_t,
                opt_bytes_to_ptr(end),
                end.map_or(0, |e| e.len()) as size_t,
            );
        }
    }
}

impl<T> CompactRangeCF for T
where
    T: CompactRangeCFOpt,
{
    fn compact_range_cf<B: AsRef<[u8]>, E: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        begin: Option<B>,
        end: Option<E>,
    ) {
        self.compact_range_cf_opt(cf, begin, end, &CompactOptions::default())
    }
}

impl<T> CompactRangeCFOpt for T
where
    T: Handle<ffi::rocksdb_t> + super::Write,
{
    fn compact_range_cf_opt<B: AsRef<[u8]>, E: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        begin: Option<B>,
        end: Option<E>,
        opts: &CompactOptions,
    ) {
        unsafe {
            let begin = begin.as_ref().map(AsRef::as_ref);
            let end = end.as_ref().map(AsRef::as_ref);

            ffi::rocksdb_compact_range_cf_opt(
                self.handle(),
                cf.inner,
                opts.inner,
                opt_bytes_to_ptr(begin),
                begin.map_or(0, |s| s.len()) as size_t,
                opt_bytes_to_ptr(end),
                end.map_or(0, |e| e.len()) as size_t,
            );
        }
    }
}
