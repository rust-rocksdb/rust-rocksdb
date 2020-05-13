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

use crate::{ffi, ColumnFamily, DBIterator, DBRawIterator, Error, IteratorMode, ReadOptions, DB};

/// A consistent view of the database at the point of creation.
///
/// # Examples
///
/// ```
/// use rocksdb::{DB, IteratorMode, Options};
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

    /// Creates an iterator over the data in this snapshot, using the default read options.
    pub fn iterator(&self, mode: IteratorMode) -> DBIterator<'a> {
        let readopts = ReadOptions::default();
        self.iterator_opt(mode, readopts)
    }

    /// Creates an iterator over the data in this snapshot under the given column family, using
    /// the default read options.
    pub fn iterator_cf(&self, cf_handle: &ColumnFamily, mode: IteratorMode) -> DBIterator {
        let readopts = ReadOptions::default();
        self.iterator_cf_opt(cf_handle, readopts, mode)
    }

    /// Creates an iterator over the data in this snapshot, using the given read options.
    pub fn iterator_opt(&self, mode: IteratorMode, mut readopts: ReadOptions) -> DBIterator<'a> {
        readopts.set_snapshot(self);
        DBIterator::new(self.db, readopts, mode)
    }

    /// Creates an iterator over the data in this snapshot under the given column family, using
    /// the given read options.
    pub fn iterator_cf_opt(
        &self,
        cf_handle: &ColumnFamily,
        mut readopts: ReadOptions,
        mode: IteratorMode,
    ) -> DBIterator {
        readopts.set_snapshot(self);
        DBIterator::new_cf(self.db, cf_handle, readopts, mode)
    }

    /// Creates a raw iterator over the data in this snapshot, using the default read options.
    pub fn raw_iterator(&self) -> DBRawIterator {
        let readopts = ReadOptions::default();
        self.raw_iterator_opt(readopts)
    }

    /// Creates a raw iterator over the data in this snapshot under the given column family, using
    /// the default read options.
    pub fn raw_iterator_cf(&self, cf_handle: &ColumnFamily) -> DBRawIterator {
        let readopts = ReadOptions::default();
        self.raw_iterator_cf_opt(cf_handle, readopts)
    }

    /// Creates a raw iterator over the data in this snapshot, using the given read options.
    pub fn raw_iterator_opt(&self, mut readopts: ReadOptions) -> DBRawIterator {
        readopts.set_snapshot(self);
        DBRawIterator::new(self.db, readopts)
    }

    /// Creates a raw iterator over the data in this snapshot under the given column family, using
    /// the given read options.
    pub fn raw_iterator_cf_opt(
        &self,
        cf_handle: &ColumnFamily,
        mut readopts: ReadOptions,
    ) -> DBRawIterator {
        readopts.set_snapshot(self);
        DBRawIterator::new_cf(self.db, cf_handle, readopts)
    }

    /// Returns the bytes associated with a key value with default read options.
    pub fn get<K: AsRef<[u8]>>(&self, key: K) -> Result<Option<Vec<u8>>, Error> {
        let readopts = ReadOptions::default();
        self.get_opt(key, readopts)
    }

    /// Returns the bytes associated with a key value and given column family with default read
    /// options.
    pub fn get_cf<K: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        key: K,
    ) -> Result<Option<Vec<u8>>, Error> {
        let readopts = ReadOptions::default();
        self.get_cf_opt(cf, key.as_ref(), readopts)
    }

    /// Returns the bytes associated with a key value and given read options.
    pub fn get_opt<K: AsRef<[u8]>>(
        &self,
        key: K,
        mut readopts: ReadOptions,
    ) -> Result<Option<Vec<u8>>, Error> {
        readopts.set_snapshot(self);
        self.db.get_opt(key.as_ref(), &readopts)
    }

    /// Returns the bytes associated with a key value, given column family and read options.
    pub fn get_cf_opt<K: AsRef<[u8]>>(
        &self,
        cf: &ColumnFamily,
        key: K,
        mut readopts: ReadOptions,
    ) -> Result<Option<Vec<u8>>, Error> {
        readopts.set_snapshot(self);
        self.db.get_cf_opt(cf, key.as_ref(), &readopts)
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
