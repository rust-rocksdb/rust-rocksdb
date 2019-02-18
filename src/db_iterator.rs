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

use libc::{c_char, c_uchar, size_t};
use std::slice;
use std::marker::PhantomData;

use crate::{DB, ColumnFamily, Error, ReadOptions};

/// An iterator over a database or column family, with specifiable
/// ranges and direction.
///
/// This iterator is different to the standard ``DBIterator`` as it aims Into
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
pub struct DBRawIterator<'a> {
    inner: *mut ffi::rocksdb_iterator_t,
    db: PhantomData<&'a DB>,
}

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
pub struct DBIterator<'a> {
    raw: DBRawIterator<'a>,
    direction: Direction,
    just_seeked: bool,
}

unsafe impl<'a> Send for DBIterator<'a> {}

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

impl<'a> DBRawIterator<'a> {
    pub(crate) fn new(db: &DB, readopts: &ReadOptions) -> DBRawIterator<'a> {
        unsafe {
            DBRawIterator {
                inner: ffi::rocksdb_create_iterator(db.inner, readopts.inner),
                db: PhantomData,
            }
        }
    }

    pub(crate) fn new_cf(
        db: &DB,
        cf_handle: ColumnFamily,
        readopts: &ReadOptions,
    ) -> Result<DBRawIterator<'a>, Error> {
        unsafe {
            Ok(DBRawIterator {
                inner: ffi::rocksdb_create_iterator_cf(db.inner, readopts.inner, cf_handle.inner),
                db: PhantomData,
            })
        }
    }

    /// Returns true if the iterator is valid.
    pub fn valid(&self) -> bool {
        unsafe { ffi::rocksdb_iter_valid(self.inner) != 0 }
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
    ///
    /// Returns true if the iterator is valid after this operation.
    pub fn next(&mut self) {
        unsafe {
            ffi::rocksdb_iter_next(self.inner);
        }
    }

    /// Seeks to the previous key.
    ///
    /// Returns true if the iterator is valid after this operation.
    pub fn prev(&mut self) {
        unsafe {
            ffi::rocksdb_iter_prev(self.inner);
        }
    }

    /// Returns a slice to the internal buffer storing the current key.
    ///
    /// This may be slightly more performant to use than the standard ``.key()`` method
    /// as it does not copy the key. However, you must be careful to not use the buffer
    /// if the iterator's seek position is ever moved by any of the seek commands or the
    /// ``.next()`` and ``.previous()`` methods as the underlying buffer may be reused
    /// for something else or freed entirely.
    pub unsafe fn key_inner(&self) -> Option<&[u8]> {
        if self.valid() {
            let mut key_len: size_t = 0;
            let key_len_ptr: *mut size_t = &mut key_len;
            let key_ptr = ffi::rocksdb_iter_key(self.inner, key_len_ptr) as *const c_uchar;

            Some(slice::from_raw_parts(key_ptr, key_len as usize))
        } else {
            None
        }
    }

    /// Returns a copy of the current key.
    pub fn key(&self) -> Option<Vec<u8>> {
        unsafe { self.key_inner().map(|key| key.to_vec()) }
    }

    /// Returns a slice to the internal buffer storing the current value.
    ///
    /// This may be slightly more performant to use than the standard ``.value()`` method
    /// as it does not copy the value. However, you must be careful to not use the buffer
    /// if the iterator's seek position is ever moved by any of the seek commands or the
    /// ``.next()`` and ``.previous()`` methods as the underlying buffer may be reused
    /// for something else or freed entirely.
    pub unsafe fn value_inner(&self) -> Option<&[u8]> {
        if self.valid() {
            let mut val_len: size_t = 0;
            let val_len_ptr: *mut size_t = &mut val_len;
            let val_ptr = ffi::rocksdb_iter_value(self.inner, val_len_ptr) as *const c_uchar;

            Some(slice::from_raw_parts(val_ptr, val_len as usize))
        } else {
            None
        }
    }

    /// Returns a copy of the current value.
    pub fn value(&self) -> Option<Vec<u8>> {
        unsafe { self.value_inner().map(|value| value.to_vec()) }
    }
}

impl<'a> Drop for DBRawIterator<'a> {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_iter_destroy(self.inner);
        }
    }
}

impl<'a> DBIterator<'a> {
    pub(crate) fn new(db: &DB, readopts: &ReadOptions, mode: IteratorMode) -> DBIterator<'a> {
        let mut rv = DBIterator {
            raw: DBRawIterator::new(db, readopts),
            direction: Direction::Forward, // blown away by set_mode()
            just_seeked: false,
        };
        rv.set_mode(mode);
        rv
    }

    pub(crate) fn new_cf(
        db: &DB,
        cf_handle: ColumnFamily,
        readopts: &ReadOptions,
        mode: IteratorMode,
    ) -> Result<DBIterator<'a>, Error> {
        let mut rv = DBIterator {
            raw: DBRawIterator::new_cf(db, cf_handle, readopts)?,
            direction: Direction::Forward, // blown away by set_mode()
            just_seeked: false,
        };
        rv.set_mode(mode);
        Ok(rv)
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

    pub fn valid(&self) -> bool {
        self.raw.valid()
    }
}

impl<'a> Iterator for DBIterator<'a> {
    type Item = KVBytes;

    fn next(&mut self) -> Option<KVBytes> {
        // Initial call to next() after seeking should not move the iterator
        // or the first item will not be returned
        if !self.just_seeked {
            match self.direction {
                Direction::Forward => self.raw.next(),
                Direction::Reverse => self.raw.prev(),
            }
        } else {
            self.just_seeked = false;
        }

        if self.raw.valid() {
            // .key() and .value() only ever return None if valid == false, which we've just cheked
            Some((
                self.raw.key().unwrap().into_boxed_slice(),
                self.raw.value().unwrap().into_boxed_slice(),
            ))
        } else {
            None
        }
    }
}

impl<'a> Into<DBRawIterator<'a>> for DBIterator<'a> {
    fn into(self) -> DBRawIterator<'a> {
        self.raw
    }
}
