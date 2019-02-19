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

use std::ffi::CString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{Error, Options, DB};

/// Ensures that DB::Destroy is called and the directory is deleted
/// for this database when TemporaryDBPath is dropped.
pub struct TemporaryDBPath {
    path: PathBuf,
}

impl TemporaryDBPath {
    /// Suffixes the given `prefix` with a timestamp to ensure that subsequent test runs don't reuse
    /// an old database in case of panics prior to Drop being called.
    pub fn new(prefix: &str) -> TemporaryDBPath {
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let path = format!(
            "{}.{}.{}",
            prefix,
            current_time.as_secs(),
            current_time.subsec_nanos()
        );

        TemporaryDBPath {
            path: PathBuf::from(path),
        }
    }
}

impl Drop for TemporaryDBPath {
    fn drop(&mut self) {
        {
            let opts = Options::default();
            DB::destroy(&opts, &self.path).unwrap();
        }

        fs::remove_dir_all(self).unwrap();
    }
}

impl AsRef<Path> for TemporaryDBPath {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

pub fn to_cpath<P: AsRef<Path>>(path: P) -> Result<CString, Error> {
    match CString::new(path.as_ref().to_string_lossy().as_bytes()) {
        Ok(c) => Ok(c),
        Err(_) => Err(Error::new(
            "Failed to convert path to CString when opening DB.".to_owned(),
        )),
    }
}
