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

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{Options, DB};

/// Ensures that DB::Destroy is called and the directory is deleted
/// for this database when TemporaryDBPath is dropped.
#[derive(Default)]
pub struct TemporaryDBPath {
    path: PathBuf,
}

static PATH_NUM: AtomicUsize = AtomicUsize::new(0);

impl TemporaryDBPath {
    /// Suffixes the given `prefix` with a timestamp to ensure that subsequent test runs don't reuse
    /// an old database in case of panics prior to Drop being called.
    pub fn new() -> TemporaryDBPath {
        // needed to disambiguate directories when running in
        // a multi-threaded environment (eg. `cargo test`) since
        // there is no guarantee time will be unique
        let path_num = PATH_NUM.fetch_add(1, Ordering::SeqCst);

        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

        let path = format!(
            "temp_rocksdb.{}.{}.{}",
            path_num,
            current_time.as_secs(),
            current_time.subsec_nanos()
        );

        let path = PathBuf::from(path);
        TemporaryDBPath { path }
    }
}

impl Drop for TemporaryDBPath {
    fn drop(&mut self) {
        {
            let opts = Options::default();
            DB::destroy(&opts, &self.path).unwrap();
        }
        let _ = fs::remove_dir_all(self);
    }
}

impl AsRef<Path> for TemporaryDBPath {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}
