// Copyright 2020 Tyler Neely
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

use crate::{
    ffi,
    ops::{Get, GetCF, GetCFOpt, GetOpt, Iterate, IterateCF},
    ColumnFamily, DBRawIterator, Error, ReadOptions, DB,
};

/// A consistent view of the database at the point of creation.
///
/// # Examples
///
/// ```
/// use rocksdb::{prelude::*, IteratorMode};
///
/// let path = "_path_for_rocksdb_storage3";
/// {
///     let db = DB::open_default(path).unwrap();
///     let snapshot = db.snapshot(); // Creates a longer-term snapshot of the DB, but closed when goes out of scope
///     let mut iter = snapshot.iterator(IteratorMode::Start); // Make as many iterators as you'd like from one snapshot
/// }
/// let _ = DB::destroy(&Options::default(), path);
/// ```
///
pub struct Snapshot<'a> {
    db: &'a DB,
    pub(crate) inner: *const ffi::rocksdb_snapshot_t,
}

impl<'a> Snapshot<'a> {
    /// Creates a new `Snapshot` of the database `db`.
    pub fn new(db: &DB) -> Snapshot {
        let snapshot = unsafe { ffi::rocksdb_create_snapshot(db.inner) };
        Snapshot {
            db,
            inner: snapshot,
        }
    }
}

impl<'a> Get for Snapshot<'a> {
    fn get<K: AsRef<[u8]>>(&self, key: K) -> Result<Option<Vec<u8>>, Error> {
        self.get_opt(key, ReadOptions::default())
    }
}

impl<'a> GetOpt<ReadOptions> for Snapshot<'a> {
    fn get_opt<K: AsRef<[u8]>>(
        &self,
        key: K,
        mut readopts: ReadOptions,
    ) -> Result<Option<Vec<u8>>, Error> {
        readopts.set_snapshot(self);
        self.db.get_opt(key, &readopts)
    }
}

impl<'a> GetCF for Snapshot<'a> {
    fn get_cf<K: AsRef<[u8]>>(&self, cf: &ColumnFamily, key: K) -> Result<Option<Vec<u8>>, Error> {
        self.get_cf_opt(cf, key, ReadOptions::default())
    }
}

impl<'a> GetCFOpt<ReadOptions> for Snapshot<'a> {
    fn get_cf_opt<K: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        key: K,
        mut readopts: ReadOptions,
    ) -> Result<Option<Vec<u8>>, Error> {
        readopts.set_snapshot(self);
        self.db.get_cf_opt(cf, key, &readopts)
    }
}

impl<'s> Iterate for Snapshot<'s> {
    fn raw_iterator_opt<'a: 'b, 'b>(&'a self, mut readopts: ReadOptions) -> DBRawIterator<'b> {
        readopts.set_snapshot(self);
        self.db.raw_iterator_opt(readopts)
    }
}

impl<'s> IterateCF for Snapshot<'s> {
    fn raw_iterator_cf_opt<'a: 'b, 'b>(
        &'a self,
        cf_handle: &ColumnFamily,
        mut readopts: ReadOptions,
    ) -> DBRawIterator<'b> {
        readopts.set_snapshot(self);
        self.db.raw_iterator_cf_opt(cf_handle, readopts)
    }
}

impl<'a> Drop for Snapshot<'a> {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_release_snapshot(self.db.inner, self.inner);
        }
    }
}

/// `Send` and `Sync` implementations for `Snapshot` are safe, because `Snapshot` is
/// immutable and can be safely shared between threads.
unsafe impl<'a> Send for Snapshot<'a> {}
unsafe impl<'a> Sync for Snapshot<'a> {}
