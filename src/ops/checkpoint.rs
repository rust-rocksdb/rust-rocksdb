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

use crate::{db::DBInner, ffi, handle::Handle, Error};
use ambassador::delegatable_trait;

/// This is an internal trait used to create and free a checkpoint
#[delegatable_trait]
pub trait CheckpointInternal {
    unsafe fn create_checkpoint_object(&self) -> Result<*mut ffi::rocksdb_checkpoint_t, Error>;
}

impl CheckpointInternal for DBInner {
    unsafe fn create_checkpoint_object(&self) -> Result<*mut ffi::rocksdb_checkpoint_t, Error> {
        Ok(ffi_try!(ffi::rocksdb_checkpoint_object_create(
            self.handle()
        )))
    }
}
