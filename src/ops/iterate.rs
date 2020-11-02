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

use crate::{
    db::DBInner, ffi, handle::Handle, ColumnFamily, DBIterator, DBRawIterator, Direction,
    IteratorMode, ReadOptions,
};

#[delegatable_trait]
pub trait Iterate {
    /// Opens a raw iterator over the database, using the given read options
    fn raw_iterator_opt<'a: 'b, 'b>(&'a self, readopts: ReadOptions) -> DBRawIterator<'b>;

    /// Opens a raw iterator over the database, using the default read options
    fn raw_iterator<'a: 'b, 'b>(&'a self) -> DBRawIterator<'b> {
        let readopts = ReadOptions::default();
        self.raw_iterator_opt(readopts)
    }

    fn iterator_opt<'a: 'b, 'b>(
        &'a self,
        mode: IteratorMode,
        readopts: ReadOptions,
    ) -> DBIterator<'b> {
        DBIterator::new(self.raw_iterator_opt(readopts), mode)
    }

    fn iterator<'a: 'b, 'b>(&'a self, mode: IteratorMode) -> DBIterator<'b> {
        let readopts = ReadOptions::default();
        self.iterator_opt(mode, readopts)
    }

    /// Opens an interator with `set_total_order_seek` enabled.
    /// This must be used to iterate across prefixes when `set_memtable_factory`
    /// has been called with a Hash-based implementation.
    fn full_iterator<'a: 'b, 'b>(&'a self, mode: IteratorMode) -> DBIterator<'b> {
        let mut readopts = ReadOptions::default();
        readopts.set_total_order_seek(true);
        self.iterator_opt(mode, readopts)
    }

    fn prefix_iterator<'a: 'b, 'b, P: AsRef<[u8]>>(&'a self, prefix: P) -> DBIterator<'b> {
        let mut readopts = ReadOptions::default();
        readopts.set_prefix_same_as_start(true);
        self.iterator_opt(
            IteratorMode::From(prefix.as_ref(), Direction::Forward),
            readopts,
        )
    }
}

#[delegatable_trait]
pub trait IterateCF {
    /// Opens a raw iterator over the given column family, using the given read options
    fn raw_iterator_cf_opt<'a: 'b, 'b>(
        &'a self,
        cf_handle: &ColumnFamily,
        readopts: ReadOptions,
    ) -> DBRawIterator<'b>;

    /// Opens a raw iterator over the given column family, using the default read options
    fn raw_iterator_cf<'a: 'b, 'b>(&'a self, cf_handle: &ColumnFamily) -> DBRawIterator<'b> {
        let readopts = ReadOptions::default();
        self.raw_iterator_cf_opt(cf_handle, readopts)
    }

    /// Opens an interator using the provided ReadOptions.
    /// This is used when you want to iterate over a specific ColumnFamily
    /// with a modified ReadOptions
    fn iterator_cf_opt<'a: 'b, 'b>(
        &'a self,
        cf: &ColumnFamily,
        readopts: ReadOptions,
        mode: IteratorMode,
    ) -> DBIterator<'b> {
        DBIterator::new(self.raw_iterator_cf_opt(cf, readopts), mode)
    }

    fn iterator_cf<'a: 'b, 'b>(&'a self, cf: &ColumnFamily, mode: IteratorMode) -> DBIterator<'b> {
        let readopts = ReadOptions::default();
        self.iterator_cf_opt(cf, readopts, mode)
    }

    fn full_iterator_cf<'a: 'b, 'b>(
        &'a self,
        cf: &ColumnFamily,
        mode: IteratorMode,
    ) -> DBIterator<'b> {
        let mut readopts = ReadOptions::default();
        readopts.set_total_order_seek(true);
        self.iterator_cf_opt(cf, readopts, mode)
    }

    fn prefix_iterator_cf<'a: 'b, 'b, P: AsRef<[u8]>>(
        &'a self,
        cf: &ColumnFamily,
        prefix: P,
    ) -> DBIterator<'b> {
        let mut readopts = ReadOptions::default();
        readopts.set_prefix_same_as_start(true);
        self.iterator_cf_opt(
            cf,
            readopts,
            IteratorMode::From(prefix.as_ref(), Direction::Forward),
        )
    }
}

impl Iterate for DBInner {
    fn raw_iterator_opt<'a: 'b, 'b>(&'a self, readopts: ReadOptions) -> DBRawIterator<'b> {
        let iter = unsafe { ffi::rocksdb_create_iterator(self.handle(), readopts.inner) };
        DBRawIterator::new(iter, readopts)
    }
}

impl IterateCF for DBInner {
    fn raw_iterator_cf_opt<'a: 'b, 'b>(
        &'a self,
        cf: &ColumnFamily,
        readopts: ReadOptions,
    ) -> DBRawIterator<'b> {
        let iter =
            unsafe { ffi::rocksdb_create_iterator_cf(self.handle(), readopts.inner, cf.inner) };

        DBRawIterator::new(iter, readopts)
    }
}
