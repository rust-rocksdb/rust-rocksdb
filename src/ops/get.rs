use libc::{c_char, size_t};

use ffi;

use crate::{handle::Handle, ReadOptions, Error, DBVector, ColumnFamily};

pub trait Get {
  fn get_opt<K: AsRef<[u8]>>(
        &self,
        key: K,
        readopts: &ReadOptions,
    ) -> Result<Option<DBVector>, Error>;

    /// Return the bytes associated with a key value
    fn get<K: AsRef<[u8]>>(&self, key: K) -> Result<Option<DBVector>, Error> {
        self.get_opt(key.as_ref(), &ReadOptions::default())
    }
}

pub trait GetCF {
    fn get_cf_opt<K: AsRef<[u8]>>(
        &self,
        cf: ColumnFamily,
        key: K,
        readopts: &ReadOptions,
    ) -> Result<Option<DBVector>, Error>;


    fn get_cf<K: AsRef<[u8]>>(
        &self,
        cf: ColumnFamily,
        key: K,
    ) -> Result<Option<DBVector>, Error> {
        self.get_cf_opt(cf, key.as_ref(), &ReadOptions::default())
    }
}

impl<T> Get for T
  where T: Handle<ffi::rocksdb_t> + super::Read {

fn get_opt<K: AsRef<[u8]>>(
        &self,
        key: K,
        readopts: &ReadOptions,
    ) -> Result<Option<DBVector>, Error> {

        if readopts.inner.is_null() {
            return Err(Error::new(
                "Unable to create RocksDB read options. \
                 This is a fairly trivial call, and its \
                 failure may be indicative of a \
                 mis-compiled or mis-loaded RocksDB \
                 library."
                    .to_owned(),
            ));
        }

        let key = key.as_ref();

        unsafe {
            let mut val_len: size_t = 0;
            let val = ffi_try!(ffi::rocksdb_get(
                self.handle(),
                readopts.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
                &mut val_len,
            )) as *mut u8;
            if val.is_null() {
                Ok(None)
            } else {
                Ok(Some(DBVector::from_c(val, val_len)))
            }
        }
    }
}

impl<T> GetCF for T
  where T: Handle<ffi::rocksdb_t> + super::Read {

fn get_cf_opt<K: AsRef<[u8]>>(
        &self,
        cf: ColumnFamily,
        key: K,
        readopts: &ReadOptions,
    ) -> Result<Option<DBVector>, Error> {
        if readopts.inner.is_null() {
            return Err(Error::new(
                "Unable to create RocksDB read options. \
                 This is a fairly trivial call, and its \
                 failure may be indicative of a \
                 mis-compiled or mis-loaded RocksDB \
                 library."
                    .to_owned(),
            ));
        }

        let key = key.as_ref();

        unsafe {
            let mut val_len: size_t = 0;
            let val = ffi_try!(ffi::rocksdb_get_cf(
                self.handle(),
                readopts.inner,
                cf.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
                &mut val_len,
            )) as *mut u8;
            if val.is_null() {
                Ok(None)
            } else {
                Ok(Some(DBVector::from_c(val, val_len)))
            }
        }
    }


    fn get_cf<K: AsRef<[u8]>>(
        &self,
        cf: ColumnFamily,
        key: K,
    ) -> Result<Option<DBVector>, Error> {
        self.get_cf_opt(cf, key.as_ref(), &ReadOptions::default())
    }
}