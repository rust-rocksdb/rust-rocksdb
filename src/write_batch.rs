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

use ffi;

use crate::{handle::Handle, ColumnFamily, Error};

use libc::{c_char, size_t};

/// An atomic batch of write operations.
///
/// Making an atomic commit of several writes:
///
/// ```
/// use rocksdb::{prelude::*, WriteBatch};
/// # use rocksdb::TemporaryDBPath;
///
/// let path = "_path_for_rocksdb_storage1";
/// # let path = TemporaryDBPath::new();
/// # {
///
/// let db = DB::open_default(&path).unwrap();
///
/// let mut batch = WriteBatch::default();
/// batch.put(b"my key", b"my value");
/// batch.put(b"key2", b"value2");
/// batch.put(b"key3", b"value3");
///
/// db.write(batch); // Atomically commits the batch

/// # }
/// ```
pub struct WriteBatch {
    inner: *mut ffi::rocksdb_writebatch_t,
}

impl WriteBatch {
    pub fn len(&self) -> usize {
        unsafe { ffi::rocksdb_writebatch_count(self.inner) as usize }
    }

    /// Return WriteBatch serialized size (in bytes).
    pub fn size_in_bytes(&self) -> usize {
        unsafe {
            let mut batch_size: size_t = 0;
            ffi::rocksdb_writebatch_data(self.inner, &mut batch_size);
            batch_size as usize
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Insert a value into the database under the given key.
    pub fn put<K, V>(&mut self, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let key = key.as_ref();
        let value = value.as_ref();

        unsafe {
            ffi::rocksdb_writebatch_put(
                self.handle(),
                key.as_ptr() as *const c_char,
                key.len() as size_t,
                value.as_ptr() as *const c_char,
                value.len() as size_t,
            );
            Ok(())
        }
    }

    pub fn put_cf<K, V>(&mut self, cf: ColumnFamily, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let key = key.as_ref();
        let value = value.as_ref();

        unsafe {
            ffi::rocksdb_writebatch_put_cf(
                self.handle(),
                cf.handle(),
                key.as_ptr() as *const c_char,
                key.len() as size_t,
                value.as_ptr() as *const c_char,
                value.len() as size_t,
            );
            Ok(())
        }
    }

    pub fn merge<K, V>(&mut self, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let key = key.as_ref();
        let value = value.as_ref();

        unsafe {
            ffi::rocksdb_writebatch_merge(
                self.handle(),
                key.as_ptr() as *const c_char,
                key.len() as size_t,
                value.as_ptr() as *const c_char,
                value.len() as size_t,
            );
            Ok(())
        }
    }

    pub fn merge_cf<K, V>(&mut self, cf: ColumnFamily, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let key = key.as_ref();
        let value = value.as_ref();

        unsafe {
            ffi::rocksdb_writebatch_merge_cf(
                self.handle(),
                cf.handle(),
                key.as_ptr() as *const c_char,
                key.len() as size_t,
                value.as_ptr() as *const c_char,
                value.len() as size_t,
            );
            Ok(())
        }
    }

    /// Remove the database entry for key.
    ///
    /// Returns an error if the key was not found.
    pub fn delete<K: AsRef<[u8]>>(&mut self, key: K) -> Result<(), Error> {
        let key = key.as_ref();

        unsafe {
            ffi::rocksdb_writebatch_delete(
                self.handle(),
                key.as_ptr() as *const c_char,
                key.len() as size_t,
            );
            Ok(())
        }
    }

    pub fn delete_cf<K: AsRef<[u8]>>(&mut self, cf: ColumnFamily, key: K) -> Result<(), Error> {
        let key = key.as_ref();

        unsafe {
            ffi::rocksdb_writebatch_delete_cf(
                self.handle(),
                cf.handle(),
                key.as_ptr() as *const c_char,
                key.len() as size_t,
            );
            Ok(())
        }
    }

    /// Remove database entries from start key to end key.
    ///
    /// Removes the database entries in the range ["begin_key", "end_key"), i.e.,
    /// including "begin_key" and excluding "end_key". It is not an error if no
    /// keys exist in the range ["begin_key", "end_key").
    pub fn delete_range<K: AsRef<[u8]>>(&mut self, from: K, to: K) -> Result<(), Error> {
        let (start_key, end_key) = (from.as_ref(), to.as_ref());

        unsafe {
            ffi::rocksdb_writebatch_delete_range(
                self.handle(),
                start_key.as_ptr() as *const c_char,
                start_key.len() as size_t,
                end_key.as_ptr() as *const c_char,
                end_key.len() as size_t,
            );
            Ok(())
        }
    }

    /// Remove database entries in column family from start key to end key.
    ///
    /// Removes the database entries in the range ["begin_key", "end_key"), i.e.,
    /// including "begin_key" and excluding "end_key". It is not an error if no
    /// keys exist in the range ["begin_key", "end_key").
    pub fn delete_range_cf<K: AsRef<[u8]>>(
        &mut self,
        cf: ColumnFamily,
        from: K,
        to: K,
    ) -> Result<(), Error> {
        let (start_key, end_key) = (from.as_ref(), to.as_ref());

        unsafe {
            ffi::rocksdb_writebatch_delete_range_cf(
                self.handle(),
                cf.handle(),
                start_key.as_ptr() as *const c_char,
                start_key.len() as size_t,
                end_key.as_ptr() as *const c_char,
                end_key.len() as size_t,
            );
            Ok(())
        }
    }

    /// Clear all updates buffered in this batch.
    pub fn clear(&mut self) -> Result<(), Error> {
        unsafe {
            ffi::rocksdb_writebatch_clear(self.inner);
        }
        Ok(())
    }
}

impl Default for WriteBatch {
    fn default() -> WriteBatch {
        WriteBatch {
            inner: unsafe { ffi::rocksdb_writebatch_create() },
        }
    }
}

impl Drop for WriteBatch {
    fn drop(&mut self) {
        unsafe { ffi::rocksdb_writebatch_destroy(self.inner) }
    }
}

impl Handle<ffi::rocksdb_writebatch_t> for WriteBatch {
    fn handle(&self) -> *mut ffi::rocksdb_writebatch_t {
        self.inner
    }
}
