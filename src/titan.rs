use std::ffi::{CStr, CString};
use std::ops::Deref;

use crocksdb_ffi::{self, DBCompressionType, DBTitanBlobIndex, DBTitanDBOptions};
use std::os::raw::c_double;
use std::os::raw::c_int;
use std::os::raw::c_uchar;

pub struct TitanDBOptions {
    pub inner: *mut DBTitanDBOptions,
}

impl TitanDBOptions {
    pub fn new() -> Self {
        unsafe {
            Self {
                inner: crocksdb_ffi::ctitandb_options_create(),
            }
        }
    }

    pub fn dirname(&self) -> &str {
        unsafe {
            let name = crocksdb_ffi::ctitandb_options_dirname(self.inner);
            CStr::from_ptr(name).to_str().unwrap()
        }
    }

    pub fn set_dirname(&mut self, name: &str) {
        let s = CString::new(name).unwrap();
        unsafe {
            crocksdb_ffi::ctitandb_options_set_dirname(self.inner, s.into_raw());
        }
    }

    pub fn min_blob_size(&self) -> u64 {
        unsafe { crocksdb_ffi::ctitandb_options_min_blob_size(self.inner) }
    }

    pub fn set_min_blob_size(&mut self, size: u64) {
        unsafe {
            crocksdb_ffi::ctitandb_options_set_min_blob_size(self.inner, size);
        }
    }

    pub fn blob_file_compression(&self) -> DBCompressionType {
        unsafe { crocksdb_ffi::ctitandb_options_blob_file_compression(self.inner) }
    }

    pub fn set_blob_file_compression(&mut self, t: DBCompressionType) {
        unsafe {
            crocksdb_ffi::ctitandb_options_set_blob_file_compression(self.inner, t);
        }
    }

    pub fn set_disable_background_gc(&mut self, disable: bool) {
        unsafe {
            crocksdb_ffi::ctitandb_options_set_disable_background_gc(self.inner, disable);
        }
    }

    pub fn set_max_background_gc(&mut self, size: i32) {
        unsafe {
            crocksdb_ffi::ctitandb_options_set_max_background_gc(self.inner, size);
        }
    }

    pub fn set_min_gc_batch_size(&mut self, size: u64) {
        unsafe {
            crocksdb_ffi::ctitandb_options_set_min_gc_batch_size(self.inner, size);
        }
    }

    pub fn set_max_gc_batch_size(&mut self, size: u64) {
        unsafe {
            crocksdb_ffi::ctitandb_options_set_max_gc_batch_size(self.inner, size);
        }
    }

    pub fn set_blob_cache(
        &mut self,
        size: usize,
        shard_bits: c_int,
        capacity_limit: c_uchar,
        pri_ratio: c_double,
    ) {
        let cache = crocksdb_ffi::new_cache(size, shard_bits, capacity_limit, pri_ratio);
        unsafe {
            crocksdb_ffi::ctitandb_options_set_blob_cache(self.inner, cache);
            crocksdb_ffi::crocksdb_cache_destroy(cache);
        }
    }

    pub fn set_discardable_ratio(&mut self, ratio: f64) {
        unsafe {
            crocksdb_ffi::ctitandb_options_set_discardable_ratio(self.inner, ratio);
        }
    }

    pub fn set_sample_ratio(&mut self, ratio: f64) {
        unsafe {
            crocksdb_ffi::ctitandb_options_set_sample_ratio(self.inner, ratio);
        }
    }

    pub fn set_merge_small_file_threshold(&mut self, size: u64) {
        unsafe {
            crocksdb_ffi::ctitandb_options_set_merge_small_file_threshold(self.inner, size);
        }
    }
}

impl Drop for TitanDBOptions {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::ctitandb_options_destroy(self.inner);
        }
    }
}

#[derive(Debug, Default)]
pub struct TitanBlobIndex {
    inner: DBTitanBlobIndex,
}

impl TitanBlobIndex {
    pub fn decode_from(value: &[u8]) -> Result<Self, String> {
        let mut index = Self::default();
        unsafe {
            ffi_try!(ctitandb_decode_blob_index(
                value.as_ptr(),
                value.len() as u64,
                &mut index.inner as *mut DBTitanBlobIndex
            ));
        }
        Ok(index)
    }
}

impl Deref for TitanBlobIndex {
    type Target = DBTitanBlobIndex;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
