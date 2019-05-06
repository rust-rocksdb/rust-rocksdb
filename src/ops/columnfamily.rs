use ffi;

use crate::{ffi_util::to_cstring, handle::Handle, ColumnFamily, Error, Options};

use std::collections::BTreeMap;

pub trait GetColumnFamilys {
    fn get_cfs(&self) -> &BTreeMap<String, ColumnFamily>;

    fn get_mut_cfs(&mut self) -> &mut BTreeMap<String, ColumnFamily>;

    /// Return the underlying column family handle.
    fn cf_handle(&self, name: &str) -> Option<&ColumnFamily> {
        self.get_cfs().get(name)
    }
}

pub trait CreateCf {
    fn create_cf<N: AsRef<str>>(&mut self, name: N, opts: &Options) -> Result<(), Error>;
}

pub trait DropCf {
    fn drop_cf(&mut self, name: &str) -> Result<(), Error>;
}

impl<T> CreateCf for T
where
    T: Handle<ffi::rocksdb_t> + super::Write + GetColumnFamilys,
{
    fn create_cf<N: AsRef<str>>(&mut self, name: N, opts: &Options) -> Result<(), Error> {
        let cname = to_cstring(
            name.as_ref(),
            "Failed to convert path to CString when opening rocksdb",
        )?;
        unsafe {
            let cf_handle = ffi_try!(ffi::rocksdb_create_column_family(
                self.handle(),
                opts.inner,
                cname.as_ptr(),
            ));

            self.get_mut_cfs()
                .insert(name.as_ref().to_string(), ColumnFamily::new(cf_handle));
        };
        Ok(())
    }
}

impl<T> DropCf for T
where
    T: Handle<ffi::rocksdb_t> + super::Write + GetColumnFamilys,
{
    fn drop_cf(&mut self, name: &str) -> Result<(), Error> {
        let cf = self
            .get_mut_cfs()
            .remove(name)
            .ok_or_else(|| Error::new(format!("Invalid column family: {}", name).to_owned()))?;
        unsafe {
            ffi_try!(ffi::rocksdb_drop_column_family(self.handle(), cf.inner,));
        }
        Ok(())
    }
}
