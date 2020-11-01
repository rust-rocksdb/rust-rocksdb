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
use libc::c_uchar;

/// This is an internal trait used to create and free a checkpoint
#[delegatable_trait]
pub trait BackupInternal {
    unsafe fn create_new_backup_flush<H: Handle<ffi::rocksdb_backup_engine_t>>(
        &self,
        backup_engine_handle: &H,
        flush_before_backup: bool,
    ) -> Result<(), Error>;
}

impl<T> BackupInternal for T
where
    T: Handle<ffi::rocksdb_t>,
{
    unsafe fn create_new_backup_flush<H: Handle<ffi::rocksdb_backup_engine_t>>(
        &self,
        backup_engine: &H,
        flush_before_backup: bool,
    ) -> Result<(), Error> {
        ffi_try!(ffi::rocksdb_backup_engine_create_new_backup_flush(
            backup_engine.handle(),
            self.handle(),
            flush_before_backup as c_uchar,
        ));
        Ok(())
    }
}
