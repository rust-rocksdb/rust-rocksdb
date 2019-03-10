// Copyright 2019 Tyler Neely
//
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
    ops::*, ColumnFamily, DBIterator, DBRawIterator, DBVector, Error, IteratorMode, ReadOptions, DB,
};

/// A consistent view of the database at the point of creation.
///
/// ```
/// use rocksdb::{prelude::*, IteratorMode};
/// # use rocksdb::TemporaryDBPath;
///
/// let path = "_path_for_rocksdb_storage3";
/// # let path = TemporaryDBPath::new();
/// # {
///
///     let db = DB::open_default(&path).unwrap();
///     let snapshot = db.snapshot(); // Creates a longer-term snapshot of the DB, but closed when goes out of scope
///     let mut iter = snapshot.iterator(IteratorMode::Start); // Make as many iterators as you'd like from one snapshot

/// # }
/// ```
///
pub struct Snapshot<'a> {
    db: &'a DB,
    pub(crate) inner: *const ffi::rocksdb_snapshot_t,
}

impl<'a> Snapshot<'a> {
    pub fn new(db: &DB) -> Snapshot {
        let snapshot = unsafe { ffi::rocksdb_create_snapshot(db.inner) };
        Snapshot {
            db,
            inner: snapshot,
        }
    }

    pub fn iterator(&self, mode: IteratorMode) -> DBIterator {
        let readopts = ReadOptions::default();
        self.iterator_opt(mode, readopts)
    }

    pub fn iterator_cf(
        &self,
        cf_handle: ColumnFamily,
        mode: IteratorMode,
    ) -> Result<DBIterator, Error> {
        let readopts = ReadOptions::default();
        self.iterator_cf_opt(cf_handle, readopts, mode)
    }

    pub fn iterator_opt(&self, mode: IteratorMode, mut readopts: ReadOptions) -> DBIterator {
        readopts.set_snapshot(self);
        DBIterator::new(self.db, &readopts, mode)
    }

    pub fn iterator_cf_opt(
        &self,
        cf_handle: ColumnFamily,
        mut readopts: ReadOptions,
        mode: IteratorMode,
    ) -> Result<DBIterator, Error> {
        readopts.set_snapshot(self);
        DBIterator::new_cf(self.db, cf_handle, &readopts, mode)
    }

    pub fn raw_iterator(&self) -> DBRawIterator {
        let readopts = ReadOptions::default();
        self.raw_iterator_opt(readopts)
    }

    pub fn raw_iterator_cf(&self, cf_handle: ColumnFamily) -> Result<DBRawIterator, Error> {
        let readopts = ReadOptions::default();
        self.raw_iterator_cf_opt(cf_handle, readopts)
    }

    pub fn raw_iterator_opt(&self, mut readopts: ReadOptions) -> DBRawIterator {
        readopts.set_snapshot(self);
        DBRawIterator::new(self.db, &readopts)
    }

    pub fn raw_iterator_cf_opt(
        &self,
        cf_handle: ColumnFamily,
        mut readopts: ReadOptions,
    ) -> Result<DBRawIterator, Error> {
        readopts.set_snapshot(self);
        DBRawIterator::new_cf(self.db, cf_handle, &readopts)
    }
}

impl<'a> Read for Snapshot<'a> {}

impl<'a> GetCF<'a> for Snapshot<'a> {
    type ColumnFamily = ColumnFamily<'a>;
    type ReadOptions = ReadOptions;

    fn get_cf_full<K: AsRef<[u8]>>(
        &self,
        cf: Option<Self::ColumnFamily>,
        key: K,
        readopts: Option<Self::ReadOptions>,
    ) -> Result<Option<DBVector>, Error> {
        let mut ro = readopts.unwrap_or_else(|| ReadOptions::default());
        ro.set_snapshot(self);

        self.db.get_cf_full(cf, key, Some(&ro))
    }
}

impl<'a> Drop for Snapshot<'a> {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_release_snapshot(self.db.inner, self.inner);
        }
    }
}
