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
    db::DBAccess, ffi, AsColumnFamilyRef, DBIteratorWithThreadMode, DBRawIteratorWithThreadMode,
    Error, IteratorMode, ReadOptions, DB,
};

/// A type alias to keep compatibility. See [`SnapshotWithThreadMode`] for details
pub type Snapshot<'a> = SnapshotWithThreadMode<'a, DB>;

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
pub struct SnapshotWithThreadMode<'a, D: DBAccess> {
    db: &'a D,
    pub(crate) inner: *const ffi::rocksdb_snapshot_t,
}

impl<'a, D: DBAccess> SnapshotWithThreadMode<'a, D> {
    /// Creates a new `SnapshotWithThreadMode` of the database `db`.
    pub fn new(db: &'a D) -> Self {
        let snapshot = unsafe { ffi::rocksdb_create_snapshot(db.inner()) };
        Self {
            db,
            inner: snapshot,
        }
    }

    /// Creates an iterator over the data in this snapshot, using the default read options.
    pub fn iterator(&self, mode: IteratorMode) -> DBIteratorWithThreadMode<'a, D> {
        let readopts = ReadOptions::default();
        self.iterator_opt(mode, readopts)
    }

    /// Creates an iterator over the data in this snapshot under the given column family, using
    /// the default read options.
    pub fn iterator_cf(
        &self,
        cf_handle: &impl AsColumnFamilyRef,
        mode: IteratorMode,
    ) -> DBIteratorWithThreadMode<D> {
        let readopts = ReadOptions::default();
        self.iterator_cf_opt(cf_handle, readopts, mode)
    }

    /// Creates an iterator over the data in this snapshot, using the given read options.
    pub fn iterator_opt(
        &self,
        mode: IteratorMode,
        mut readopts: ReadOptions,
    ) -> DBIteratorWithThreadMode<'a, D> {
        readopts.set_snapshot(self);
        DBIteratorWithThreadMode::<D>::new(self.db, readopts, mode)
    }

    /// Creates an iterator over the data in this snapshot under the given column family, using
    /// the given read options.
    pub fn iterator_cf_opt(
        &self,
        cf_handle: &impl AsColumnFamilyRef,
        mut readopts: ReadOptions,
        mode: IteratorMode,
    ) -> DBIteratorWithThreadMode<D> {
        readopts.set_snapshot(self);
        DBIteratorWithThreadMode::new_cf(self.db, cf_handle.inner(), readopts, mode)
    }

    /// Creates a raw iterator over the data in this snapshot, using the default read options.
    pub fn raw_iterator(&self) -> DBRawIteratorWithThreadMode<D> {
        let readopts = ReadOptions::default();
        self.raw_iterator_opt(readopts)
    }

    /// Creates a raw iterator over the data in this snapshot under the given column family, using
    /// the default read options.
    pub fn raw_iterator_cf(
        &self,
        cf_handle: &impl AsColumnFamilyRef,
    ) -> DBRawIteratorWithThreadMode<D> {
        let readopts = ReadOptions::default();
        self.raw_iterator_cf_opt(cf_handle, readopts)
    }

    /// Creates a raw iterator over the data in this snapshot, using the given read options.
    pub fn raw_iterator_opt(&self, mut readopts: ReadOptions) -> DBRawIteratorWithThreadMode<D> {
        readopts.set_snapshot(self);
        DBRawIteratorWithThreadMode::new(self.db, readopts)
    }

    /// Creates a raw iterator over the data in this snapshot under the given column family, using
    /// the given read options.
    pub fn raw_iterator_cf_opt(
        &self,
        cf_handle: &impl AsColumnFamilyRef,
        mut readopts: ReadOptions,
    ) -> DBRawIteratorWithThreadMode<D> {
        readopts.set_snapshot(self);
        DBRawIteratorWithThreadMode::new_cf(self.db, cf_handle.inner(), readopts)
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
        cf: &impl AsColumnFamilyRef,
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
        cf: &impl AsColumnFamilyRef,
        key: K,
        mut readopts: ReadOptions,
    ) -> Result<Option<Vec<u8>>, Error> {
        readopts.set_snapshot(self);
        self.db.get_cf_opt(cf, key.as_ref(), &readopts)
    }

    /// Returns the bytes associated with the given key values and default read options.
    pub fn multi_get<K: AsRef<[u8]>, I>(&self, keys: I) -> Vec<Result<Option<Vec<u8>>, Error>>
    where
        I: IntoIterator<Item = K>,
    {
        let readopts = ReadOptions::default();
        self.multi_get_opt(keys, readopts)
    }

    /// Returns the bytes associated with the given key values and default read options.
    pub fn multi_get_cf<'b, K, I, W>(&self, keys_cf: I) -> Vec<Result<Option<Vec<u8>>, Error>>
    where
        K: AsRef<[u8]>,
        I: IntoIterator<Item = (&'b W, K)>,
        W: AsColumnFamilyRef + 'b,
    {
        let readopts = ReadOptions::default();
        self.multi_get_cf_opt(keys_cf, readopts)
    }

    /// Returns the bytes associated with the given key values and given read options.
    pub fn multi_get_opt<K, I>(
        &self,
        keys: I,
        mut readopts: ReadOptions,
    ) -> Vec<Result<Option<Vec<u8>>, Error>>
    where
        K: AsRef<[u8]>,
        I: IntoIterator<Item = K>,
    {
        readopts.set_snapshot(self);
        self.db.multi_get_opt(keys, &readopts)
    }

    /// Returns the bytes associated with the given key values, given column family and read options.
    pub fn multi_get_cf_opt<'b, K, I, W>(
        &self,
        keys_cf: I,
        mut readopts: ReadOptions,
    ) -> Vec<Result<Option<Vec<u8>>, Error>>
    where
        K: AsRef<[u8]>,
        I: IntoIterator<Item = (&'b W, K)>,
        W: AsColumnFamilyRef + 'b,
    {
        readopts.set_snapshot(self);
        self.db.multi_get_cf_opt(keys_cf, &readopts)
    }
}

impl<'a, D: DBAccess> Drop for SnapshotWithThreadMode<'a, D> {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_release_snapshot(self.db.inner(), self.inner);
        }
    }
}

/// `Send` and `Sync` implementations for `SnapshotWithThreadMode` are safe, because `SnapshotWithThreadMode` is
/// immutable and can be safely shared between threads.
unsafe impl<'a, D: DBAccess> Send for SnapshotWithThreadMode<'a, D> {}
unsafe impl<'a, D: DBAccess> Sync for SnapshotWithThreadMode<'a, D> {}
