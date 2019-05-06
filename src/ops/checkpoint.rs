use crate::{checkpoint::Checkpoint, handle::Handle, Error};
use ffi;

pub trait CreateCheckpointObject {
    unsafe fn create_checkpoint_object_raw(&self) -> Result<*mut ffi::rocksdb_checkpoint_t, Error>;
    fn create_checkpoint_object(&self) -> Result<Checkpoint, Error> {
        let checkpoint: *mut ffi::rocksdb_checkpoint_t;

        unsafe { checkpoint = self.create_checkpoint_object_raw()? };

        if checkpoint.is_null() {
            return Err(Error::new("Could not create checkpoint object.".to_owned()));
        }

        Ok(Checkpoint { inner: checkpoint })
    }
}

impl<T> CreateCheckpointObject for T
where
    T: Handle<ffi::rocksdb_t>,
{
    unsafe fn create_checkpoint_object_raw(&self) -> Result<*mut ffi::rocksdb_checkpoint_t, Error> {
        Ok(ffi_try!(ffi::rocksdb_checkpoint_object_create(
            self.handle(),
        )))
    }
}
