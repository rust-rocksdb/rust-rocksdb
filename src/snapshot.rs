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
    handle::ConstHandle, ops::*, ColumnFamily, DBRawIterator, DBVector, Error, ReadOptions,
    ReadOptionsFactory, DB,
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
    pub(crate) db: &'a DB,
    pub(crate) inner: *const ffi::rocksdb_snapshot_t,
}

impl<'a> ConstHandle<ffi::rocksdb_snapshot_t> for Snapshot<'a> {
    fn const_handle(&self) -> *const ffi::rocksdb_snapshot_t {
        self.inner
    }
}

impl<'a> Read for Snapshot<'a> {}

impl<'a> GetCF<ReadOptionsFactory> for Snapshot<'a> {
    fn get_cf_full<K: AsRef<[u8]>>(
        &self,
        cf: Option<&ColumnFamily>,
        key: K,
        readopts: Option<&ReadOptionsFactory>,
    ) -> Result<Option<DBVector>, Error> {
        let mut ro = if let Some(rof) = readopts {
            rof.build()
        } else {
            ReadOptions::default()
        };
        ro.set_snapshot(self.inner);

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

impl<'a> SnapshotIterate for Snapshot<'a> {
    fn get_raw_iter(&self, readopts: &ReadOptionsFactory) -> DBRawIterator {
        let mut ro = readopts.build();
        ro.set_snapshot(self.inner);
        self.db.get_raw_iter(&ro)
    }
}

impl<'a> SnapshotIterateCF for Snapshot<'a> {
    fn get_raw_iter_cf(
        &self,
        cf_handle: &ColumnFamily,
        readopts: &ReadOptionsFactory,
    ) -> Result<DBRawIterator, Error> {
        let mut ro = readopts.build();
        ro.set_snapshot(self.inner);
        self.db.get_raw_iter_cf(cf_handle, &ro)
    }
}
