// Copyright 2020 Tyler Neely
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

use pretty_assertions::assert_eq;

use rocksdb::{
    backup::{BackupEngine, BackupEngineOptions, RestoreOptions},
    DB,
};
use util::DBPath;

#[test]
fn backup_restore() {
    // create backup
    let path = DBPath::new("backup_test");
    let restore_path = DBPath::new("restore_from_backup_path");
    {
        let db = DB::open_default(&path).unwrap();
        assert!(db.put(b"k1", b"v1111").is_ok());
        let value = db.get(b"k1");
        assert_eq!(value.unwrap().unwrap(), b"v1111");
        {
            let backup_path = DBPath::new("backup_path");
            let backup_opts = BackupEngineOptions::default();
            let mut backup_engine = BackupEngine::open(&backup_opts, &backup_path).unwrap();
            assert!(backup_engine.create_new_backup(&db).is_ok());

            // check backup info
            let info = backup_engine.get_backup_info();
            assert!(!info.is_empty());
            info.iter().for_each(|i| {
                assert!(backup_engine.verify_backup(i.backup_id).is_ok());
                assert!(i.size > 0);
            });

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
