use libc::{c_char, size_t};

use ffi;

use crate::{handle::Handle, ReadOptions, Error, DBVector, ColumnFamily};

pub trait Get {
    fn get_full<K: AsRef<[u8]>>(
        &self,
        cf: Option<ColumnFamily>,
        key: K,
        readopts: Option<&ReadOptions>,
    ) -> Result<Option<DBVector>, Error>;

    /// Return the bytes associated with a key value
    fn get<K: AsRef<[u8]>>(&self, key: K) -> Result<Option<DBVector>, Error> {
        self.get_full(None, key, None)
    }

    fn get_opt<K: AsRef<[u8]>>(
        &self,
        key: K,
        readopts: &ReadOptions,
    ) -> Result<Option<DBVector>, Error> {
        self.get_full(None, key, Some(readopts))
    }

    fn get_cf<K: AsRef<[u8]>>(
        &self,
        cf: ColumnFamily,
        key: K,
    ) -> Result<Option<DBVector>, Error> {
        self.get_full(Some(cf), key, None)
    }

    fn get_cf_opt<K: AsRef<[u8]>>(
        &self,
        cf: ColumnFamily,
        key: K,
        readopts: &ReadOptions,
    ) -> Result<Option<DBVector>, Error> {
        self.get_full(Some(cf), key, Some(readopts))
    }
}

impl<T> Get for T
  where T: Handle<ffi::rocksdb_t> + super::Read {

    fn get_full<K: AsRef<[u8]>>(
        &self,
        cf: Option<ColumnFamily>,
        key: K,
        readopts: Option<&ReadOptions>,
    ) -> Result<Option<DBVector>, Error> {

        let mut default_readopts = None;

        if readopts.is_none() {
            default_readopts.replace(ReadOptions::default());
        }

        let ro_handle = readopts
            .or(default_readopts.as_ref())
            .map(|r| r.inner)
            .ok_or(Error::new("Unable to extract read options.".to_string()))?;

        if ro_handle.is_null() {
            return Err(Error::new(
                "Unable to create RocksDB read options. \
                 This is a fairly trivial call, and its \
                 failure may be indicative of a \
                 mis-compiled or mis-loaded RocksDB \
                 library."
                    .to_string(),
            ));
        }

        let key = key.as_ref();
        let key_ptr = key.as_ptr() as *const c_char;
        let key_len = key.len() as size_t;

        unsafe {
            let mut val_len: size_t = 0;

            let val = match cf {
                Some(cf) => ffi_try!(ffi::rocksdb_get_cf(
                    self.handle(),
                    ro_handle,
                    cf.inner,
                    key_ptr,
                    key_len,
                    &mut val_len,
                )),
                None => ffi_try!(ffi::rocksdb_get(
                    self.handle(),
                    ro_handle,
                    key_ptr,
                    key_len,
                    &mut val_len,
                ))
            } as *mut u8;
                
            if val.is_null() {
                Ok(None)
            } else {
                Ok(Some(DBVector::from_c(val, val_len)))
            }
        }
    }
}