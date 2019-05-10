use ffi;
use libc::{c_char, size_t};

use crate::{handle::Handle, ColumnFamily, Error, WriteOptions};

pub trait Merge<W> {
    fn merge_full<K, V>(&self, key: K, value: V, writeopts: Option<&W>) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>;

    fn merge<K, V>(&self, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.merge_full(key, value, None)
    }

    fn merge_opt<K, V>(&self, key: K, value: V, writeopts: &W) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.merge_full(key, value, Some(writeopts))
    }
}

pub trait MergeCF<W> {
    fn merge_cf_full<K, V>(
        &self,
        cf: Option<&ColumnFamily>,
        key: K,
        value: V,
        writeopts: Option<&W>,
    ) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>;

    fn merge_cf<K, V>(&self, cf: &ColumnFamily, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.merge_cf_full(Some(cf), key, value, None)
    }

    fn merge_cf_opt<K, V>(
        &self,
        cf: &ColumnFamily,
        key: K,
        value: V,
        writeopts: &W,
    ) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.merge_cf_full(Some(cf), key, value, Some(writeopts))
    }
}

impl<T, W> Merge<W> for T
where
    T: MergeCF<W>,
{
    fn merge_full<K: AsRef<[u8]>, V: AsRef<[u8]>>(
        &self,
        key: K,
        value: V,
        writeopts: Option<&W>,
    ) -> Result<(), Error> {
        self.merge_cf_full(None, key, value, writeopts)
    }
}

impl<T> MergeCF<WriteOptions> for T
where
    T: Handle<ffi::rocksdb_t> + super::Write,
{
    fn merge_cf_full<K, V>(
        &self,
        cf: Option<&ColumnFamily>,
        key: K,
        value: V,
        writeopts: Option<&WriteOptions>,
    ) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let mut default_writeopts = None;

        let wo_handle = WriteOptions::input_or_default(writeopts, &mut default_writeopts)?;

        let key = key.as_ref();
        let value = value.as_ref();
        let key_ptr = key.as_ptr() as *const c_char;
        let key_len = key.len() as size_t;
        let val_ptr = value.as_ptr() as *const c_char;
        let val_len = value.len() as size_t;

        unsafe {
            match cf {
                Some(cf) => ffi_try!(ffi::rocksdb_merge_cf(
                    self.handle(),
                    wo_handle,
                    cf.handle(),
                    key_ptr,
                    key_len,
                    val_ptr,
                    val_len,
                )),
                None => ffi_try!(ffi::rocksdb_merge(
                    self.handle(),
                    wo_handle,
                    key_ptr,
                    key_len,
                    val_ptr,
                    val_len,
                )),
            }

            Ok(())
        }
    }
}
