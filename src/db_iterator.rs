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
    db::{DBAccess, DB},
    ffi, Error, ReadOptions, WriteBatch,
};
use libc::{c_char, c_uchar, size_t};
use std::{marker::PhantomData, slice};

/// A type alias to keep compatibility. See [`DBRawIteratorWithThreadMode`] for details
pub type DBRawIterator<'a> = DBRawIteratorWithThreadMode<'a, DB>;

/// An iterator over a database or column family, with specifiable
/// ranges and direction.
///
/// This iterator is different to the standard ``DBIteratorWithThreadMode`` as it aims Into
/// replicate the underlying iterator API within RocksDB itself. This should
/// give access to more performance and flexibility but departs from the
/// widely recognised Rust idioms.
///
/// ```
/// use rocksdb::{DB, Options};
///
/// let path = "_path_for_rocksdb_storage4";
/// {
///     let db = DB::open_default(path).unwrap();
///     let mut iter = db.raw_iterator();
///
///     // Forwards iteration
///     iter.seek_to_first();
///     while iter.valid() {
///         println!("Saw {:?} {:?}", iter.key(), iter.value());
///         iter.next();
///     }
///
///     // Reverse iteration
///     iter.seek_to_last();
///     while iter.valid() {
///         println!("Saw {:?} {:?}", iter.key(), iter.value());
///         iter.prev();
///     }
///
///     // Seeking
///     iter.seek(b"my key");
///     while iter.valid() {
///         println!("Saw {:?} {:?}", iter.key(), iter.value());
///         iter.next();
///     }
///
///     // Reverse iteration from key
///     // Note, use seek_for_prev when reversing because if this key doesn't exist,
///     // this will make the iterator start from the previous key rather than the next.
///     iter.seek_for_prev(b"my key");
///     while iter.valid() {
///         println!("Saw {:?} {:?}", iter.key(), iter.value());
///         iter.prev();
///     }
/// }
/// let _ = DB::destroy(&Options::default(), path);
/// ```
pub struct DBRawIteratorWithThreadMode<'a, D: DBAccess> {
    inner: std::ptr::NonNull<ffi::rocksdb_iterator_t>,

    /// When iterate_lower_bound or iterate_upper_bound are set, the inner
    /// C iterator keeps a pointer to the upper bound inside `_readopts`.
    /// Storing this makes sure the upper bound is always alive when the
    /// iterator is being used.
    ///
    /// And yes, we need to store the entire ReadOptions structure since C++
    /// ReadOptions keep reference to C rocksdb_readoptions_t wrapper which
    /// point to vectors we own.  See issue #660.
    _readopts: ReadOptions,

    db: PhantomData<&'a D>,
}

impl<'a, D: DBAccess> DBRawIteratorWithThreadMode<'a, D> {
    pub(crate) fn new(db: &D, readopts: ReadOptions) -> Self {
        let inner = unsafe { db.create_iterator(&readopts) };
        Self::from_inner(inner, readopts)
    }

    pub(crate) fn new_cf(
        db: &'a D,
        cf_handle: *mut ffi::rocksdb_column_family_handle_t,
        readopts: ReadOptions,
    ) -> Self {
        let inner = unsafe { db.create_iterator_cf(cf_handle, &readopts) };
        Self::from_inner(inner, readopts)
    }

    fn from_inner(inner: *mut ffi::rocksdb_iterator_t, readopts: ReadOptions) -> Self {
        // This unwrap will never fail since rocksdb_create_iterator and
        // rocksdb_create_iterator_cf functions always return non-null. They
        // use new and deference the result so any nulls would end up with SIGSEGV
        // there and we would have a bigger issue.
        let inner = std::ptr::NonNull::new(inner).unwrap();
        Self {
            inner,
            _readopts: readopts,
            db: PhantomData,
        }
    }

    /// Returns `true` if the iterator is valid. An iterator is invalidated when
    /// it reaches the end of its defined range, or when it encounters an error.
    ///
    /// To check whether the iterator encountered an error after `valid` has
    /// returned `false`, use the [`status`](DBRawIteratorWithThreadMode::status) method. `status` will never
    /// return an error when `valid` is `true`.
    pub fn valid(&self) -> bool {
        unsafe { ffi::rocksdb_iter_valid(self.inner.as_ptr()) != 0 }
    }

    /// Returns an error `Result` if the iterator has encountered an error
    /// during operation. When an error is encountered, the iterator is
    /// invalidated and [`valid`](DBRawIteratorWithThreadMode::valid) will return `false` when called.
    ///
    /// Performing a seek will discard the current status.
    pub fn status(&self) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_iter_get_error(self.inner.as_ptr()));
        }
        Ok(())
    }

    /// Seeks to the first key in the database.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rocksdb::{DB, Options};
    ///
    /// let path = "_path_for_rocksdb_storage5";
    /// {
    ///     let db = DB::open_default(path).unwrap();
    ///     let mut iter = db.raw_iterator();
    ///
    ///     // Iterate all keys from the start in lexicographic order
    ///     iter.seek_to_first();
    ///
    ///     while iter.valid() {
    ///         println!("{:?} {:?}", iter.key(), iter.value());
    ///         iter.next();
    ///     }
    ///
    ///     // Read just the first key
    ///     iter.seek_to_first();
    ///
    ///     if iter.valid() {
    ///         println!("{:?} {:?}", iter.key(), iter.value());
    ///     } else {
    ///         // There are no keys in the database
    ///     }
    /// }
    /// let _ = DB::destroy(&Options::default(), path);
    /// ```
    pub fn seek_to_first(&mut self) {
        unsafe {
            ffi::rocksdb_iter_seek_to_first(self.inner.as_ptr());
        }
    }

    /// Seeks to the last key in the database.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rocksdb::{DB, Options};
    ///
    /// let path = "_path_for_rocksdb_storage6";
    /// {
    ///     let db = DB::open_default(path).unwrap();
    ///     let mut iter = db.raw_iterator();
    ///
    ///     // Iterate all keys from the end in reverse lexicographic order
    ///     iter.seek_to_last();
    ///
    ///     while iter.valid() {
    ///         println!("{:?} {:?}", iter.key(), iter.value());
    ///         iter.prev();
    ///     }
    ///
    ///     // Read just the last key
    ///     iter.seek_to_last();
    ///
    ///     if iter.valid() {
    ///         println!("{:?} {:?}", iter.key(), iter.value());
    ///     } else {
    ///         // There are no keys in the database
    ///     }
    /// }
    /// let _ = DB::destroy(&Options::default(), path);
    /// ```
    pub fn seek_to_last(&mut self) {
        unsafe {
            ffi::rocksdb_iter_seek_to_last(self.inner.as_ptr());
        }
    }

    /// Seeks to the specified key or the first key that lexicographically follows it.
    ///
    /// This method will attempt to seek to the specified key. If that key does not exist, it will
    /// find and seek to the key that lexicographically follows it instead.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rocksdb::{DB, Options};
    ///
    /// let path = "_path_for_rocksdb_storage7";
    /// {
    ///     let db = DB::open_default(path).unwrap();
    ///     let mut iter = db.raw_iterator();
    ///
    ///     // Read the first key that starts with 'a'
    ///     iter.seek(b"a");
    ///
    ///     if iter.valid() {
    ///         println!("{:?} {:?}", iter.key(), iter.value());
    ///     } else {
    ///         // There are no keys in the database
    ///     }
    /// }
    /// let _ = DB::destroy(&Options::default(), path);
    /// ```
    pub fn seek<K: AsRef<[u8]>>(&mut self, key: K) {
        let key = key.as_ref();

        unsafe {
            ffi::rocksdb_iter_seek(
                self.inner.as_ptr(),
                key.as_ptr() as *const c_char,
                key.len() as size_t,
            );
        }
    }

    /// Seeks to the specified key, or the first key that lexicographically precedes it.
    ///
    /// Like ``.seek()`` this method will attempt to seek to the specified key.
    /// The difference with ``.seek()`` is that if the specified key do not exist, this method will
    /// seek to key that lexicographically precedes it instead.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rocksdb::{DB, Options};
    ///
    /// let path = "_path_for_rocksdb_storage8";
    /// {
    ///     let db = DB::open_default(path).unwrap();
    ///     let mut iter = db.raw_iterator();
    ///
    ///     // Read the last key that starts with 'a'
    ///     iter.seek_for_prev(b"b");
    ///
    ///     if iter.valid() {
    ///         println!("{:?} {:?}", iter.key(), iter.value());
    ///     } else {
    ///         // There are no keys in the database
    ///     }
    /// }
    /// let _ = DB::destroy(&Options::default(), path);
    /// ```
    pub fn seek_for_prev<K: AsRef<[u8]>>(&mut self, key: K) {
        let key = key.as_ref();

        unsafe {
            ffi::rocksdb_iter_seek_for_prev(
                self.inner.as_ptr(),
                key.as_ptr() as *const c_char,
                key.len() as size_t,
            );
        }
    }

    /// Seeks to the next key.
    pub fn next(&mut self) {
        if self.valid() {
            unsafe {
                ffi::rocksdb_iter_next(self.inner.as_ptr());
            }
        }
    }

    /// Seeks to the previous key.
    pub fn prev(&mut self) {
        if self.valid() {
            unsafe {
                ffi::rocksdb_iter_prev(self.inner.as_ptr());
            }
        }
    }

    /// Returns a slice of the current key.
    pub fn key(&self) -> Option<&[u8]> {
        if self.valid() {
            Some(self.key_impl())
        } else {
            None
        }
    }

    /// Returns a slice of the current value.
    pub fn value(&self) -> Option<&[u8]> {
        if self.valid() {
            Some(self.value_impl())
        } else {
            None
        }
    }

    /// Returns pair with slice of the current key and current value.
    pub fn item(&self) -> Option<(&[u8], &[u8])> {
        if self.valid() {
            Some((self.key_impl(), self.value_impl()))
        } else {
            None
        }
    }

    /// Returns a slice of the current key; assumes the iterator is valid.
    fn key_impl(&self) -> &[u8] {
        // Safety Note: This is safe as all methods that may invalidate the buffer returned
        // take `&mut self`, so borrow checker will prevent use of buffer after seek.
        unsafe {
            let mut key_len: size_t = 0;
            let key_len_ptr: *mut size_t = &mut key_len;
            let key_ptr = ffi::rocksdb_iter_key(self.inner.as_ptr(), key_len_ptr);
            slice::from_raw_parts(key_ptr as *const c_uchar, key_len)
        }
    }

    /// Returns a slice of the current value; assumes the iterator is valid.
    fn value_impl(&self) -> &[u8] {
        // Safety Note: This is safe as all methods that may invalidate the buffer returned
        // take `&mut self`, so borrow checker will prevent use of buffer after seek.
        unsafe {
            let mut val_len: size_t = 0;
            let val_len_ptr: *mut size_t = &mut val_len;
            let val_ptr = ffi::rocksdb_iter_value(self.inner.as_ptr(), val_len_ptr);
            slice::from_raw_parts(val_ptr as *const c_uchar, val_len)
        }
    }
}

impl<'a, D: DBAccess> Drop for DBRawIteratorWithThreadMode<'a, D> {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_iter_destroy(self.inner.as_ptr());
        }
    }
}

unsafe impl<'a, D: DBAccess> Send for DBRawIteratorWithThreadMode<'a, D> {}
unsafe impl<'a, D: DBAccess> Sync for DBRawIteratorWithThreadMode<'a, D> {}

/// A type alias to keep compatibility. See [`DBIteratorWithThreadMode`] for details
pub type DBIterator<'a> = DBIteratorWithThreadMode<'a, DB>;

/// An iterator over a database or column family, with specifiable
/// ranges and direction.
///
/// ```
/// use rocksdb::{DB, Direction, IteratorMode, Options};
///
/// let path = "_path_for_rocksdb_storage2";
/// {
///     let db = DB::open_default(path).unwrap();
///     let mut iter = db.iterator(IteratorMode::Start); // Always iterates forward
///     for item in iter {
///         let (key, value) = item.unwrap();
///         println!("Saw {:?} {:?}", key, value);
///     }
///     iter = db.iterator(IteratorMode::End);  // Always iterates backward
///     for item in iter {
///         let (key, value) = item.unwrap();
///         println!("Saw {:?} {:?}", key, value);
///     }
///     iter = db.iterator(IteratorMode::From(b"my key", Direction::Forward)); // From a key in Direction::{forward,reverse}
///     for item in iter {
///         let (key, value) = item.unwrap();
///         println!("Saw {:?} {:?}", key, value);
///     }
///
///     // You can seek with an existing Iterator instance, too
///     iter = db.iterator(IteratorMode::Start);
///     iter.set_mode(IteratorMode::From(b"another key", Direction::Reverse));
///     for item in iter {
///         let (key, value) = item.unwrap();
///         println!("Saw {:?} {:?}", key, value);
///     }
/// }
/// let _ = DB::destroy(&Options::default(), path);
/// ```
pub struct DBIteratorWithThreadMode<'a, D: DBAccess> {
    raw: DBRawIteratorWithThreadMode<'a, D>,
    direction: Direction,
    done: bool,
}

#[derive(Copy, Clone)]
pub enum Direction {
    Forward,
    Reverse,
}

pub type KVBytes = (Box<[u8]>, Box<[u8]>);

#[derive(Copy, Clone)]
pub enum IteratorMode<'a> {
    Start,
    End,
    From(&'a [u8], Direction),
}

impl<'a, D: DBAccess> DBIteratorWithThreadMode<'a, D> {
    pub(crate) fn new(db: &D, readopts: ReadOptions, mode: IteratorMode) -> Self {
        Self::from_raw(DBRawIteratorWithThreadMode::new(db, readopts), mode)
    }

    pub(crate) fn new_cf(
        db: &'a D,
        cf_handle: *mut ffi::rocksdb_column_family_handle_t,
        readopts: ReadOptions,
        mode: IteratorMode,
    ) -> Self {
        Self::from_raw(
            DBRawIteratorWithThreadMode::new_cf(db, cf_handle, readopts),
            mode,
        )
    }

    fn from_raw(raw: DBRawIteratorWithThreadMode<'a, D>, mode: IteratorMode) -> Self {
        let mut rv = DBIteratorWithThreadMode {
            raw,
            direction: Direction::Forward, // blown away by set_mode()
            done: false,
        };
        rv.set_mode(mode);
        rv
    }

    pub fn set_mode(&mut self, mode: IteratorMode) {
        self.done = false;
        self.direction = match mode {
            IteratorMode::Start => {
                self.raw.seek_to_first();
                Direction::Forward
            }
            IteratorMode::End => {
                self.raw.seek_to_last();
                Direction::Reverse
            }
            IteratorMode::From(key, Direction::Forward) => {
                self.raw.seek(key);
                Direction::Forward
            }
            IteratorMode::From(key, Direction::Reverse) => {
                self.raw.seek_for_prev(key);
                Direction::Reverse
            }
        };
    }
}

impl<'a, D: DBAccess> Iterator for DBIteratorWithThreadMode<'a, D> {
    type Item = Result<KVBytes, Error>;

    fn next(&mut self) -> Option<Result<KVBytes, Error>> {
        if self.done {
            None
        } else if let Some((key, value)) = self.raw.item() {
            let item = (Box::from(key), Box::from(value));
            match self.direction {
                Direction::Forward => self.raw.next(),
                Direction::Reverse => self.raw.prev(),
            }
            Some(Ok(item))
        } else {
            self.done = true;
            self.raw.status().err().map(Result::Err)
        }
    }
}

impl<'a, D: DBAccess> std::iter::FusedIterator for DBIteratorWithThreadMode<'a, D> {}

impl<'a, D: DBAccess> Into<DBRawIteratorWithThreadMode<'a, D>> for DBIteratorWithThreadMode<'a, D> {
    fn into(self) -> DBRawIteratorWithThreadMode<'a, D> {
        self.raw
    }
}

/// Iterates the batches of writes since a given sequence number.
///
/// `DBWALIterator` is returned by `DB::get_updates_since()` and will return the
/// batches of write operations that have occurred since a given sequence number
/// (see `DB::latest_sequence_number()`). This iterator cannot be constructed by
/// the application.
///
/// The iterator item type is a tuple of (`u64`, `WriteBatch`) where the first
/// value is the sequence number of the associated write batch.
///
pub struct DBWALIterator {
    pub(crate) inner: *mut ffi::rocksdb_wal_iterator_t,
    pub(crate) start_seq_number: u64,
}

impl DBWALIterator {
    /// Returns `true` if the iterator is valid. An iterator is invalidated when
    /// it reaches the end of its defined range, or when it encounters an error.
    ///
    /// To check whether the iterator encountered an error after `valid` has
    /// returned `false`, use the [`status`](DBWALIterator::status) method.
    /// `status` will never return an error when `valid` is `true`.
    pub fn valid(&self) -> bool {
        unsafe { ffi::rocksdb_wal_iter_valid(self.inner) != 0 }
    }

    /// Returns an error `Result` if the iterator has encountered an error
    /// during operation. When an error is encountered, the iterator is
    /// invalidated and [`valid`](DBWALIterator::valid) will return `false` when
    /// called.
    pub fn status(&self) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_wal_iter_status(self.inner));
        }
        Ok(())
    }
}

impl Iterator for DBWALIterator {
    type Item = Result<(u64, WriteBatch), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.valid() {
            return None;
        }

        let mut seq: u64 = 0;
        let mut batch = WriteBatch {
            inner: unsafe { ffi::rocksdb_wal_iter_get_batch(self.inner, &mut seq) },
        };

        // if the initial sequence number is what was requested we skip it to
        // only provide changes *after* it
        while seq <= self.start_seq_number {
            unsafe {
                ffi::rocksdb_wal_iter_next(self.inner);
            }

            if !self.valid() {
                return None;
            }

            // this drops which in turn frees the skipped batch
            batch = WriteBatch {
                inner: unsafe { ffi::rocksdb_wal_iter_get_batch(self.inner, &mut seq) },
            };
        }

        if !self.valid() {
            return self.status().err().map(Result::Err);
        }

        // Seek to the next write batch.
        // Note that WriteBatches live independently of the WAL iterator so this is safe to do
        unsafe {
            ffi::rocksdb_wal_iter_next(self.inner);
        }

        Some(Ok((seq, batch)))
    }
}

impl Drop for DBWALIterator {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_wal_iter_destroy(self.inner);
        }
    }
}
