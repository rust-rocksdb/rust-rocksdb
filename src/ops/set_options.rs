// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

use crate::{ffi, handle::Handle, Error};
use ambassador::delegatable_trait;
use libc::c_char;
use std::ffi::CString;

#[delegatable_trait]
pub trait SetOptions {
    fn set_options(&self, opts: &[(&str, &str)]) -> Result<(), Error>;
}

impl<T> SetOptions for T
where
    T: Handle<ffi::rocksdb_t> + super::Write,
{
    fn set_options(&self, opts: &[(&str, &str)]) -> Result<(), Error> {
        let copts = opts
            .iter()
            .map(|(name, value)| {
                let cname = CString::new(name.as_bytes())
                    .map_err(|e| Error::new(format!("Invalid option name `{}`", e)))?;
                let cvalue = CString::new(value.as_bytes())
                    .map_err(|e| Error::new(format!("Invalid option name `{}`", e)))?;
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
