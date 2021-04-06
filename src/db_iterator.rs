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

use crate::db::{DBAccess, DB};
use crate::{ffi, Error, ReadOptions, WriteBatch};
use libc::{c_char, c_uchar, size_t};
use std::marker::PhantomData;
use std::slice;

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
    inner: *mut ffi::rocksdb_iterator_t,

    /// When iterate_upper_bound is set, the inner C iterator keeps a pointer to the upper bound
    /// inside `_readopts`. Storing this makes sure the upper bound is always alive when the
    /// iterator is being used.
    _readopts: ReadOptions,

    db: PhantomData<&'a D>,
}

impl<'a, D: DBAccess> DBRawIteratorWithThreadMode<'a, D> {
    pub(crate) fn new(db: &D, readopts: ReadOptions) -> DBRawIteratorWithThreadMode<'a, D> {
        unsafe {
            DBRawIteratorWithThreadMode {
                inner: ffi::rocksdb_create_iterator(db.inner(), readopts.inner),
                _readopts: readopts,
                db: PhantomData,
            }
        }
    }

    pub(crate) fn new_cf(
        db: &'a D,
        cf_handle: *mut ffi::rocksdb_column_family_handle_t,
        readopts: ReadOptions,
    ) -> DBRawIteratorWithThreadMode<'a, D> {
        unsafe {
            DBRawIteratorWithThreadMode {
                inner: ffi::rocksdb_create_iterator_cf(db.inner(), readopts.inner, cf_handle),
                _readopts: readopts,
                db: PhantomData,
            }
        }
    }

    /// Returns `true` if the iterator is valid. An iterator is invalidated when
    /// it reaches the end of its defined range, or when it encounters an error.
    ///
    /// To check whether the iterator encountered an error after `valid` has
    /// returned `false`, use the [`status`](DBRawIteratorWithThreadMode::status) method. `status` will never
    /// return an error when `valid` is `true`.
    pub fn valid(&self) -> bool {
        unsafe { ffi::rocksdb_iter_valid(self.inner) != 0 }
    }

    /// Returns an error `Result` if the iterator has encountered an error
    /// during operation. When an error is encountered, the iterator is
    /// invalidated and [`valid`](DBRawIteratorWithThreadMode::valid) will return `false` when called.
    ///
    /// Performing a seek will discard the current status.
    pub fn status(&self) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_iter_get_error(self.inner));
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
            ffi::rocksdb_iter_seek_to_first(self.inner);
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
            ffi::rocksdb_iter_seek_to_last(self.inner);
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
                self.inner,
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
                self.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
            );
        }
    }

    /// Seeks to the next key.
    pub fn next(&mut self) {
        unsafe {
            ffi::rocksdb_iter_next(self.inner);
        }
    }

    /// Seeks to the previous key.
    pub fn prev(&mut self) {
        unsafe {
            ffi::rocksdb_iter_prev(self.inner);
        }
    }

    /// Returns a slice of the current key.
    pub fn key(&self) -> Option<&[u8]> {
        if self.valid() {
            // Safety Note: This is safe as all methods that may invalidate the buffer returned
            // take `&mut self`, so borrow checker will prevent use of buffer after seek.
            unsafe {
                let mut key_len: size_t = 0;
                let key_len_ptr: *mut size_t = &mut key_len;
                let key_ptr = ffi::rocksdb_iter_key(self.inner, key_len_ptr) as *const c_uchar;

                Some(slice::from_raw_parts(key_ptr, key_len as usize))
            }
        } else {
            None
        }
    }

    /// Returns a slice of the current value.
    pub fn value(&self) -> Option<&[u8]> {
        if self.valid() {
            // Safety Note: This is safe as all methods that may invalidate the buffer returned
            // take `&mut self`, so borrow checker will prevent use of buffer after seek.
            unsafe {
                let mut val_len: size_t = 0;
                let val_len_ptr: *mut size_t = &mut val_len;
                let val_ptr = ffi::rocksdb_iter_value(self.inner, val_len_ptr) as *const c_uchar;

                Some(slice::from_raw_parts(val_ptr, val_len as usize))
            }
        } else {
            None
        }
    }
}

impl<'a, D: DBAccess> Drop for DBRawIteratorWithThreadMode<'a, D> {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_iter_destroy(self.inner);
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
///     for (key, value) in iter {
///         println!("Saw {:?} {:?}", key, value);
///     }
///     iter = db.iterator(IteratorMode::End);  // Always iterates backward
///     for (key, value) in iter {
///         println!("Saw {:?} {:?}", key, value);
///     }
///     iter = db.iterator(IteratorMode::From(b"my key", Direction::Forward)); // From a key in Direction::{forward,reverse}
///     for (key, value) in iter {
///         println!("Saw {:?} {:?}", key, value);
///     }
///
///     // You can seek with an existing Iterator instance, too
///     iter = db.iterator(IteratorMode::Start);
///     iter.set_mode(IteratorMode::From(b"another key", Direction::Reverse));
///     for (key, value) in iter {
///         println!("Saw {:?} {:?}", key, value);
///     }
/// }
/// let _ = DB::destroy(&Options::default(), path);
/// ```
pub struct DBIteratorWithThreadMode<'a, D: DBAccess> {
    raw: DBRawIteratorWithThreadMode<'a, D>,
    direction: Direction,
    just_seeked: bool,
}

pub enum Direction {
    Forward,
    Reverse,
}

pub type KVBytes = (Box<[u8]>, Box<[u8]>);

pub enum IteratorMode<'a> {
    Start,
    End,
    From(&'a [u8], Direction),
}

impl<'a, D: DBAccess> DBIteratorWithThreadMode<'a, D> {
    pub(crate) fn new(db: &D, readopts: ReadOptions, mode: IteratorMode) -> Self {
        let mut rv = DBIteratorWithThreadMode {
            raw: DBRawIteratorWithThreadMode::new(db, readopts),
            direction: Direction::Forward, // blown away by set_mode()
            just_seeked: false,
        };
        rv.set_mode(mode);
        rv
    }

    pub(crate) fn new_cf(
        db: &'a D,
        cf_handle: *mut ffi::rocksdb_column_family_handle_t,
        readopts: ReadOptions,
        mode: IteratorMode,
    ) -> Self {
        let mut rv = DBIteratorWithThreadMode {
            raw: DBRawIteratorWithThreadMode::new_cf(db, cf_handle, readopts),
            direction: Direction::Forward, // blown away by set_mode()
            just_seeked: false,
        };
        rv.set_mode(mode);
        rv
    }

    pub fn set_mode(&mut self, mode: IteratorMode) {
        match mode {
            IteratorMode::Start => {
                self.raw.seek_to_first();
                self.direction = Direction::Forward;
            }
            IteratorMode::End => {
                self.raw.seek_to_last();
                self.direction = Direction::Reverse;
            }
            IteratorMode::From(key, Direction::Forward) => {
                self.raw.seek(key);
                self.direction = Direction::Forward;
            }
            IteratorMode::From(key, Direction::Reverse) => {
                self.raw.seek_for_prev(key);
                self.direction = Direction::Reverse;
            }
        };

        self.just_seeked = true;
    }

    /// See [`valid`](DBRawIteratorWithThreadMode::valid)
    pub fn valid(&self) -> bool {
        self.raw.valid()
    }

    /// See [`status`](DBRawIteratorWithThreadMode::status)
    pub fn status(&self) -> Result<(), Error> {
        self.raw.status()
    }
}

impl<'a, D: DBAccess> Iterator for DBIteratorWithThreadMode<'a, D> {
    type Item = KVBytes;

    fn next(&mut self) -> Option<KVBytes> {
        if !self.raw.valid() {
            return None;
        }

        // Initial call to next() after seeking should not move the iterator
        // or the first item will not be returned
        if self.just_seeked {
            self.just_seeked = false;
        } else {
            match self.direction {
                Direction::Forward => self.raw.next(),
                Direction::Reverse => self.raw.prev(),
            }
        }

        if self.raw.valid() {
            // .key() and .value() only ever return None if valid == false, which we've just cheked
            Some((
                Box::from(self.raw.key().unwrap()),
                Box::from(self.raw.value().unwrap()),
            ))
        } else {
            None
        }
    }
}

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
    type Item = (u64, WriteBatch);

    fn next(&mut self) -> Option<(u64, WriteBatch)> {
        // Seek to the next write batch.
        unsafe {
            ffi::rocksdb_wal_iter_next(self.inner);
        }
        if self.valid() {
            let mut seq: u64 = 0;
            let inner = unsafe { ffi::rocksdb_wal_iter_get_batch(self.inner, &mut seq) };
            Some((seq, WriteBatch { inner }))
        } else {
            None
        }
    }
}

impl Drop for DBWALIterator {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_wal_iter_destroy(self.inner);
        }
    }
}
