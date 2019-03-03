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

extern crate libc;
extern crate rocksdb;

use rocksdb::{prelude::*, ReadOnlyDB, TemporaryDBPath};

#[test]
fn open_existing_db_in_read_only() {
    let path = TemporaryDBPath::new();

    {
        let db = DB::open_default(&path).unwrap();
        assert!(db.put(b"k1", b"v1111").is_ok());
    }

    {
      let db = ReadOnlyDB::open_default(true, &path).unwrap();
      let r: Result<Option<DBVector>, Error> = db.get(b"k1");
      
      assert!(r.unwrap().unwrap().to_utf8().unwrap() == "v1111");
    }
}