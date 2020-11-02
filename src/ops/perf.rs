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

use crate::{db::DBInner, ffi, handle::Handle};
use ambassador::delegatable_trait;

/// This is an internal trait used to create and free a checkpoint
#[delegatable_trait]
pub trait PerfInternal {
    unsafe fn memory_consumers_add_db(&self, ptr: *mut ffi::rocksdb_memory_consumers_t);
}

impl PerfInternal for DBInner {
    unsafe fn memory_consumers_add_db(&self, ptr: *mut ffi::rocksdb_memory_consumers_t) {
        ffi::rocksdb_memory_consumers_add_db(ptr, self.handle());
    }
}
