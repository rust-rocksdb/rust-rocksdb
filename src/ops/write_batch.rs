// Copyright 2019 Tyler Neely
//
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

use ambassador::delegatable_trait;

use crate::{ffi, handle::Handle, Error, WriteBatch, WriteOptions};

#[delegatable_trait]
pub trait WriteBatchWrite {
    fn write(&self, batch: WriteBatch) -> Result<(), Error>;

    fn write_without_wal(&self, batch: WriteBatch) -> Result<(), Error>;
}

#[delegatable_trait]
pub trait WriteBatchWriteOpt {
    fn write_opt(&self, batch: WriteBatch, writeopts: &WriteOptions) -> Result<(), Error>;
}

impl<T> WriteBatchWrite for T
where
    T: WriteBatchWriteOpt,
{
    fn write(&self, batch: WriteBatch) -> Result<(), Error> {
        self.write_opt(batch, &WriteOptions::default())
    }

    fn write_without_wal(&self, batch: WriteBatch) -> Result<(), Error> {
        let mut wo = WriteOptions::new();
        wo.disable_wal(true);
        self.write_opt(batch, &wo)
    }
}

impl<T> WriteBatchWriteOpt for T
where
    T: Handle<ffi::rocksdb_t> + super::Write,
{
    fn write_opt(&self, batch: WriteBatch, writeopts: &WriteOptions) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_write(
                self.handle(),
                writeopts.inner,
                batch.inner
            ));
        }
        Ok(())
    }
}
