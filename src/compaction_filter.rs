use crocksdb_ffi::{self, DBCompactionFilter};
use libc::{c_char, c_int, c_void, size_t};
use std::ffi::CString;
use std::slice;

/// `CompactionFilter` allows an application to modify/delete a key-value at
/// the time of compaction.
/// For more details, Please checkout rocksdb's documentation.
// TODO: support change value
pub trait CompactionFilter {
    /// The compaction process invokes this
    /// method for kv that is being compacted. A return value
    /// of false indicates that the kv should be preserved in the
    /// output of this compaction run and a return value of true
    /// indicates that this key-value should be removed from the
    /// output of the compaction.  The application can inspect
    /// the existing value of the key and make decision based on it.
    fn filter(&mut self, level: usize, key: &[u8], value: &[u8]) -> bool;
}

#[repr(C)]
pub struct CompactionFilterProxy {
    name: CString,
    filter: Box<dyn CompactionFilter>,
}

extern "C" fn name(filter: *mut c_void) -> *const c_char {
    unsafe { (*(filter as *mut CompactionFilterProxy)).name.as_ptr() }
}

extern "C" fn destructor(filter: *mut c_void) {
    unsafe {
        Box::from_raw(filter as *mut CompactionFilterProxy);
    }
}

extern "C" fn filter(
    filter: *mut c_void,
    level: c_int,
    key: *const u8,
    key_len: size_t,
    value: *const u8,
    value_len: size_t,
    _: *mut *mut u8,
    _: *mut size_t,
    value_changed: *mut bool,
) -> bool {
    unsafe {
        let filter = &mut *(filter as *mut CompactionFilterProxy);
        let key = slice::from_raw_parts(key, key_len);
        let value = slice::from_raw_parts(value, value_len);
        *value_changed = false;
        filter.filter.filter(level as usize, key, value)
    }
}

pub struct CompactionFilterHandle {
    pub inner: *mut DBCompactionFilter,
}

impl Drop for CompactionFilterHandle {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_compactionfilter_destroy(self.inner);
        }
    }
}

pub unsafe fn new_compaction_filter(
    c_name: CString,
    ignore_snapshots: bool,
    f: Box<dyn CompactionFilter>,
) -> Result<CompactionFilterHandle, String> {
    let proxy = Box::into_raw(Box::new(CompactionFilterProxy {
        name: c_name,
        filter: f,
    }));
    let filter = crocksdb_ffi::crocksdb_compactionfilter_create(
        proxy as *mut c_void,
        destructor,
        filter,
        name,
    );
    crocksdb_ffi::crocksdb_compactionfilter_set_ignore_snapshots(filter, ignore_snapshots);
    Ok(CompactionFilterHandle { inner: filter })
}
