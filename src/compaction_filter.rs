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
//

use libc::{c_char, c_int, c_uchar, c_void, size_t};
use std::ffi::{CStr, CString};
use std::slice;

/// Decision about how to handle compacting an object
///
/// This is returned by a compaction filter callback. Depending
/// on the value, the object may be kept, removed, or changed
/// in the database during a compaction.
pub enum Decision {
    /// Keep the old value
    Keep,
    /// Remove the object from the database
    Remove,
    /// Change the value for the key
    Change(&'static [u8]),
}

/// CompactionFilter allows an application to modify/delete a key-value at
/// the time of compaction.
pub trait CompactionFilter {
    /// The compaction process invokes this
    /// method for kv that is being compacted. The application can inspect
    /// the existing value of the key and make decision based on it.
    ///
    /// Key-Values that are results of merge operation during compaction are not
    /// passed into this function. Currently, when you have a mix of Put()s and
    /// Merge()s on a same key, we only guarantee to process the merge operands
    /// through the compaction filters. Put()s might be processed, or might not.
    ///
    /// When the value is to be preserved, the application has the option
    /// to modify the existing_value and pass it back through new_value.
    /// value_changed needs to be set to true in this case.
    ///
    /// Note that RocksDB snapshots (i.e. call GetSnapshot() API on a
    /// DB* object) will not guarantee to preserve the state of the DB with
    /// CompactionFilter. Data seen from a snapshot might disappear after a
    /// compaction finishes. If you use snapshots, think twice about whether you
    /// want to use compaction filter and whether you are using it in a safe way.
    ///
    /// If the CompactionFilter was created by a factory, then it will only ever
    /// be used by a single thread that is doing the compaction run, and this
    /// call does not need to be thread-safe.  However, multiple filters may be
    /// in existence and operating concurrently.
    fn filter(&mut self, level: u32, key: &[u8], value: &[u8]) -> Decision;

    /// Returns a name that identifies this compaction filter.
    /// The name will be printed to LOG file on start up for diagnosis.
    fn name(&self) -> &CStr;
}

/// Function to filter compaction with.
///
/// This function takes the level of compaction, the key, and the existing value
/// and returns the decision about how to handle the Key-Value pair.
///
///  See [Options::set_compaction_filter][set_compaction_filter] for more details
///
///  [set_compaction_filter]: ../struct.Options.html#method.set_compaction_filter
pub trait CompactionFilterFn: FnMut(u32, &[u8], &[u8]) -> Decision {}
impl<F> CompactionFilterFn for F where F: FnMut(u32, &[u8], &[u8]) -> Decision + Send + 'static {}

pub struct CompactionFilterCallback<F>
where
    F: CompactionFilterFn,
{
    pub name: CString,
    pub filter_fn: F,
}

impl<F> CompactionFilter for CompactionFilterCallback<F>
where
    F: CompactionFilterFn,
{
    fn name(&self) -> &CStr {
        self.name.as_c_str()
    }

    fn filter(&mut self, level: u32, key: &[u8], value: &[u8]) -> Decision {
        (self.filter_fn)(level, key, value)
    }
}

pub unsafe extern "C" fn destructor_callback<F>(raw_cb: *mut c_void)
where
    F: CompactionFilter,
{
    drop(Box::from_raw(raw_cb as *mut F));
}

pub unsafe extern "C" fn name_callback<F>(raw_cb: *mut c_void) -> *const c_char
where
    F: CompactionFilter,
{
    let cb = &*(raw_cb as *mut F);
    cb.name().as_ptr()
}

pub unsafe extern "C" fn filter_callback<F>(
    raw_cb: *mut c_void,
    level: c_int,
    raw_key: *const c_char,
    key_length: size_t,
    existing_value: *const c_char,
    value_length: size_t,
    new_value: *mut *mut c_char,
    new_value_length: *mut size_t,
    value_changed: *mut c_uchar,
) -> c_uchar
where
    F: CompactionFilter,
{
    use self::Decision::{Change, Keep, Remove};

    let cb = &mut *(raw_cb as *mut F);
    let key = slice::from_raw_parts(raw_key as *const u8, key_length);
    let oldval = slice::from_raw_parts(existing_value as *const u8, value_length);
    let result = cb.filter(level as u32, key, oldval);
    match result {
        Keep => 0,
        Remove => 1,
        Change(newval) => {
            *new_value = newval.as_ptr() as *mut c_char;
            *new_value_length = newval.len() as size_t;
            *value_changed = 1_u8;
            0
        }
    }
}

#[cfg(test)]
#[allow(unused_variables)]
fn test_filter(level: u32, key: &[u8], value: &[u8]) -> Decision {
    use self::Decision::{Change, Keep, Remove};
    match key.first() {
        Some(&b'_') => Remove,
        Some(&b'%') => Change(b"secret"),
        _ => Keep,
    }
}

#[test]
fn compaction_filter_test() {
    use crate::{Options, DB};

    let path = "_rust_rocksdb_filter_test";
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.set_compaction_filter("test", test_filter);
    {
        let db = DB::open(&opts, path).unwrap();
        let _r = db.put(b"k1", b"a");
        let _r = db.put(b"_k", b"b");
        let _r = db.put(b"%k", b"c");
        db.compact_range(None::<&[u8]>, None::<&[u8]>);
        assert_eq!(&*db.get(b"k1").unwrap().unwrap(), b"a");
        assert!(db.get(b"_k").unwrap().is_none());
        assert_eq!(&*db.get(b"%k").unwrap().unwrap(), b"secret");
    }
    let result = DB::destroy(&opts, path);
    assert!(result.is_ok());
}
