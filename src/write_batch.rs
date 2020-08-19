use crocksdb_ffi::{self, DBValueType, DBWriteBatch, DBWriteBatchIterator};
use libc::{c_void, size_t};
use std::marker::PhantomData;
use std::slice;

pub struct WriteBatch {
    pub(crate) inner: *mut DBWriteBatch,
}

unsafe impl Send for WriteBatch {}
impl Default for WriteBatch {
    fn default() -> WriteBatch {
        WriteBatch {
            inner: unsafe { crocksdb_ffi::crocksdb_writebatch_create() },
        }
    }
}

pub struct WriteBatchCallback<'a> {
    cb_ptr: *mut c_void,
    cfs: &'a [&'a str],
}

impl<'a> WriteBatchCallback<'a> {
    fn invoke(&mut self, cf_id: u32, value_type: DBValueType, key: &[u8], value: Option<&[u8]>) {
        let cf = self.cfs[cf_id as usize];
        unsafe {
            let cb: &mut &mut dyn FnMut(&str, DBValueType, &[u8], Option<&[u8]>) =
                &mut *(self.cb_ptr as *mut _);
            cb(cf, value_type, key, value);
        }
    }
}

impl WriteBatch {
    pub fn new() -> WriteBatch {
        WriteBatch::default()
    }

    pub fn with_capacity(cap: usize) -> WriteBatch {
        WriteBatch {
            inner: unsafe { crocksdb_ffi::crocksdb_writebatch_create_with_capacity(cap) },
        }
    }

    pub fn count(&self) -> usize {
        unsafe { crocksdb_ffi::crocksdb_writebatch_count(self.inner) as usize }
    }

    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    pub fn data_size(&self) -> usize {
        unsafe {
            let mut data_size: usize = 0;
            let _ = crocksdb_ffi::crocksdb_writebatch_data(self.inner, &mut data_size);
            return data_size;
        }
    }

    pub fn clear(&self) {
        unsafe {
            crocksdb_ffi::crocksdb_writebatch_clear(self.inner);
        }
    }

    pub fn set_save_point(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_writebatch_set_save_point(self.inner);
        }
    }

    pub fn rollback_to_save_point(&mut self) -> Result<(), String> {
        unsafe {
            ffi_try!(crocksdb_writebatch_rollback_to_save_point(self.inner));
        }
        Ok(())
    }

    pub fn pop_save_point(&mut self) -> Result<(), String> {
        unsafe {
            ffi_try!(crocksdb_writebatch_pop_save_point(self.inner));
        }
        Ok(())
    }

    pub fn data(&self) -> &[u8] {
        let mut val_len: size_t = 0;
        let val_len_ptr: *mut size_t = &mut val_len;
        unsafe {
            let val_ptr = crocksdb_ffi::crocksdb_writebatch_data(self.inner, val_len_ptr);
            slice::from_raw_parts(val_ptr, val_len as usize)
        }
    }

    pub fn append(&mut self, src: &[u8]) {
        unsafe {
            crocksdb_ffi::crocksdb_writebatch_append_content(
                self.inner,
                src.as_ptr(),
                src.len() as size_t,
            );
        }
    }

    pub fn iterate<F>(&self, cfs: &[&str], mut iterator_fn: F)
    where
        F: FnMut(&str, DBValueType, &[u8], Option<&[u8]>),
    {
        unsafe {
            let mut cb: &mut dyn FnMut(&str, DBValueType, &[u8], Option<&[u8]>) = &mut iterator_fn;
            let cb_ptr = &mut cb;
            let cb_proxy = Box::new(WriteBatchCallback {
                cfs,
                cb_ptr: cb_ptr as *mut _ as *mut c_void,
            });
            let state = Box::into_raw(cb_proxy) as *mut c_void;
            crocksdb_ffi::crocksdb_writebatch_iterate_cf(
                self.inner,
                state,
                put_fn,
                put_cf_fn,
                delete_fn,
                delete_cf_fn,
            );
        }
    }

    pub fn iter(&self) -> WriteBatchIter {
        WriteBatchIter::new(self)
    }
}

pub struct WriteBatchIter<'a> {
    props: PhantomData<&'a DBWriteBatchIterator>,
    inner: *mut DBWriteBatchIterator,
}

impl<'a> Drop for WriteBatchIter<'a> {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_writebatch_iterator_destroy(self.inner);
        }
    }
}

impl<'a> WriteBatchIter<'a> {
    fn new(wb: &'a WriteBatch) -> WriteBatchIter<'a> {
        unsafe {
            WriteBatchIter {
                props: PhantomData,
                inner: crocksdb_ffi::crocksdb_writebatch_iterator_create(wb.inner),
            }
        }
    }

    fn from_bytes(data: &'a [u8]) -> WriteBatchIter<'a> {
        unsafe {
            WriteBatchIter {
                props: PhantomData,
                inner: crocksdb_ffi::crocksdb_writebatch_ref_iterator_create(
                    data.as_ptr(),
                    data.len() as size_t,
                ),
            }
        }
    }
}

impl<'a> Iterator for WriteBatchIter<'a> {
    type Item = (DBValueType, u32, &'a [u8], &'a [u8]);

    fn next(&mut self) -> Option<(DBValueType, u32, &'a [u8], &'a [u8])> {
        unsafe {
            if !crocksdb_ffi::crocksdb_writebatch_iterator_valid(self.inner) {
                return None;
            }
            let mut klen: size_t = 0;
            let k = crocksdb_ffi::crocksdb_writebatch_iterator_key(self.inner, &mut klen);
            let key = slice::from_raw_parts(k, klen);

            let mut vlen: size_t = 0;
            let v = crocksdb_ffi::crocksdb_writebatch_iterator_value(self.inner, &mut vlen);
            let val = slice::from_raw_parts(v, vlen);

            let value_type = match crocksdb_ffi::crocksdb_writebatch_iterator_value_type(self.inner)
            {
                DBValueType::TypeColumnFamilyDeletion => DBValueType::TypeDeletion,
                DBValueType::TypeColumnFamilyValue => DBValueType::TypeValue,
                DBValueType::TypeColumnFamilyMerge => DBValueType::TypeMerge,
                DBValueType::TypeColumnFamilyRangeDeletion => DBValueType::TypeRangeDeletion,
                other => other,
            };
            let column_family =
                crocksdb_ffi::crocksdb_writebatch_iterator_column_family_id(self.inner);

            crocksdb_ffi::crocksdb_writebatch_iterator_next(self.inner);

            Some((value_type, column_family, key, val))
        }
    }
}

pub struct WriteBatchRef<'a> {
    data: &'a [u8],
}

impl<'a> WriteBatchRef<'a> {
    pub fn new(data: &'a [u8]) -> WriteBatchRef<'a> {
        WriteBatchRef { data }
    }

    pub fn count(&self) -> usize {
        unsafe {
            crocksdb_ffi::crocksdb_writebatch_ref_count(
                self.data.as_ptr(),
                self.data.len() as size_t,
            ) as usize
        }
    }

    pub fn iter(&self) -> WriteBatchIter<'a> {
        WriteBatchIter::from_bytes(self.data)
    }
}

pub unsafe extern "C" fn put_fn(
    state: *mut c_void,
    k: *const u8,
    klen: size_t,
    v: *const u8,
    vlen: size_t,
) {
    let proxy: &mut WriteBatchCallback = &mut *(state as *mut WriteBatchCallback);
    let a: &[u8] = slice::from_raw_parts(k as *const u8, klen as usize);
    let b: &[u8] = slice::from_raw_parts(v as *const u8, vlen as usize);
    proxy.invoke(0, DBValueType::TypeValue, a, Some(b));
}

pub unsafe extern "C" fn put_cf_fn(
    state: *mut c_void,
    cf_id: u32,
    k: *const u8,
    klen: size_t,
    v: *const u8,
    vlen: size_t,
) {
    let proxy: &mut WriteBatchCallback = &mut *(state as *mut WriteBatchCallback);
    let a: &[u8] = slice::from_raw_parts(k as *const u8, klen as usize);
    let b: &[u8] = slice::from_raw_parts(v as *const u8, vlen as usize);
    proxy.invoke(cf_id, DBValueType::TypeValue, a, Some(b));
}

pub unsafe extern "C" fn delete_fn(state: *mut c_void, k: *const u8, klen: size_t) {
    let proxy: &mut WriteBatchCallback = &mut *(state as *mut WriteBatchCallback);
    let k: &[u8] = slice::from_raw_parts(k as *const u8, klen as usize);
    proxy.invoke(0, DBValueType::TypeDeletion, k, None);
}

pub unsafe extern "C" fn delete_cf_fn(state: *mut c_void, cf_id: u32, k: *const u8, klen: size_t) {
    let proxy: &mut WriteBatchCallback = &mut *(state as *mut WriteBatchCallback);
    let k: &[u8] = slice::from_raw_parts(k as *const u8, klen as usize);
    proxy.invoke(cf_id, DBValueType::TypeDeletion, k, None);
}

impl Drop for WriteBatch {
    fn drop(&mut self) {
        unsafe { crocksdb_ffi::crocksdb_writebatch_destroy(self.inner) }
    }
}
