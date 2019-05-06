use ffi;

use crate::{handle::Handle, Error, WriteBatch, WriteOptions};

pub trait WriteOps {
    fn write_full(&self, batch: WriteBatch, writeopts: Option<&WriteOptions>) -> Result<(), Error>;

    fn write(&self, batch: WriteBatch) -> Result<(), Error> {
        self.write_full(batch, None)
    }

    fn write_opt(&self, batch: WriteBatch, writeopts: &WriteOptions) -> Result<(), Error> {
        self.write_full(batch, Some(writeopts))
    }

    fn write_without_wal(&self, batch: WriteBatch) -> Result<(), Error> {
        let mut wo = WriteOptions::new();
        wo.disable_wal(true);
        self.write_opt(batch, &wo)
    }
}

impl<T> WriteOps for T
where
    T: Handle<ffi::rocksdb_t> + super::Write,
{
    fn write_full(&self, batch: WriteBatch, writeopts: Option<&WriteOptions>) -> Result<(), Error> {
        let mut default_writeopts = None;

        if default_writeopts.is_none() {
            default_writeopts.replace(WriteOptions::default());
        }

        let wo_handle = writeopts
            .or_else(|| default_writeopts.as_ref())
            .map(|r| r.handle())
            .ok_or_else(|| Error::new("Unable to extract write options.".to_string()))?;

        unsafe {
            ffi_try!(ffi::rocksdb_write(self.handle(), wo_handle, batch.inner,));
            Ok(())
        }
    }
}
