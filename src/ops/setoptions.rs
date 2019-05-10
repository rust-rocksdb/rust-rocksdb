use crate::{handle::Handle, Error};
use libc::c_char;
use std::ffi::CString;

pub trait SetOptions {
    fn set_options(&self, opts: &[(&str, &str)]) -> Result<(), Error>;
}

impl<T> SetOptions for T
where
    T: Handle<ffi::rocksdb_t>,
{
    fn set_options(&self, opts: &[(&str, &str)]) -> Result<(), Error> {
        let copts = opts
            .iter()
            .map(|(name, value)| {
                let cname = match CString::new(name.as_bytes()) {
                    Ok(cname) => cname,
                    Err(e) => return Err(Error::new(format!("Invalid option name `{}`", e))),
                };
                let cvalue = match CString::new(value.as_bytes()) {
                    Ok(cvalue) => cvalue,
                    Err(e) => return Err(Error::new(format!("Invalid option value: `{}`", e))),
                };
                Ok((cname, cvalue))
            })
            .collect::<Result<Vec<(CString, CString)>, Error>>()?;

        let cnames: Vec<*const c_char> = copts.iter().map(|opt| opt.0.as_ptr()).collect();
        let cvalues: Vec<*const c_char> = copts.iter().map(|opt| opt.1.as_ptr()).collect();
        let count = opts.len() as i32;
        unsafe {
            ffi_try!(ffi::rocksdb_set_options(
                self.handle(),
                count,
                cnames.as_ptr(),
                cvalues.as_ptr(),
            ));
        }
        Ok(())
    }
}
