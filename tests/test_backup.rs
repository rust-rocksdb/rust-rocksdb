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

mod util;
use crate::util::DBPath;

use rocksdb::{
    backup::{BackupEngine, BackupEngineOptions, RestoreOptions},
    Options, DB,
};

#[test]
fn backup_restore() {
    // create backup
    let path = DBPath::new("backup_test");
    let restore_path = DBPath::new("restore_from_backup_path");
    let mut opts = Options::default();
    opts.create_if_missing(true);
    {
        let db = DB::open(&opts, &path).unwrap();
        assert!(db.put(b"k1", b"v1111").is_ok());
        let value = db.get(b"k1");
        assert_eq!(value.unwrap().unwrap(), b"v1111");
        {
            let backup_path = "_rust_rocksdb_backup_path";
            let backup_opts = BackupEngineOptions::default();
            let mut backup_engine = BackupEngine::open(&backup_opts, &backup_path).unwrap();
            assert!(backup_engine.create_new_backup(&db).is_ok());

            let mut restore_option = RestoreOptions::default();
            restore_option.set_keep_log_files(false); // true to keep log files
            let restore_status = backup_engine.restore_from_latest_backup(
                &restore_path,
                &restore_path,
                &restore_option,
            );
            assert!(restore_status.is_ok());

            let db_restore = DB::open_default(&restore_path).unwrap();
            let value = db_restore.get(b"k1");
            assert_eq!(value.unwrap().unwrap(), b"v1111");
        }
    }
}
