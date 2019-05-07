use crate::{handle::Handle, Error, FlushOptions};
use ffi;

pub trait Flush {
    /// Flush database memtable to SST files on disk (with options).
    fn flush_opt(&self, flushopts: &FlushOptions) -> Result<(), Error>;

    /// Flush database memtable to SST files on disk.
    fn flush(&self) -> Result<(), Error> {
        self.flush_opt(&FlushOptions::default())
    }
}

impl<T> Flush for T
where
    T: Handle<ffi::rocksdb_t> + super::Write,
{
    fn flush_opt(&self, flushopts: &FlushOptions) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_flush(self.handle(), flushopts.inner,));
        }
        Ok(())
    }
}
