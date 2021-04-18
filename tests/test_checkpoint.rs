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

use rocksdb::{checkpoint::Checkpoint, Options, DB};
use util::DBPath;

#[test]
pub fn test_single_checkpoint() {
    const PATH_PREFIX: &str = "_rust_rocksdb_cp_single_";

    // Create DB with some data
    let db_path = DBPath::new(&format!("{}db1", PATH_PREFIX));

    let mut opts = Options::default();
    opts.create_if_missing(true);
    let db = DB::open(&opts, &db_path).unwrap();

    db.put(b"k1", b"v1").unwrap();
    db.put(b"k2", b"v2").unwrap();
    db.put(b"k3", b"v3").unwrap();
    db.put(b"k4", b"v4").unwrap();

    // Create checkpoint
    let cp1 = Checkpoint::new(&db).unwrap();
    let cp1_path = DBPath::new(&format!("{}cp1", PATH_PREFIX));
    cp1.create_checkpoint(&cp1_path).unwrap();

    // Verify checkpoint
    let cp = DB::open_default(&cp1_path).unwrap();

    assert_eq!(cp.get(b"k1").unwrap().unwrap(), b"v1");
    assert_eq!(cp.get(b"k2").unwrap().unwrap(), b"v2");
    assert_eq!(cp.get(b"k3").unwrap().unwrap(), b"v3");
    assert_eq!(cp.get(b"k4").unwrap().unwrap(), b"v4");
}

#[test]
pub fn test_multi_checkpoints() {
    const PATH_PREFIX: &str = "_rust_rocksdb_cp_multi_";

    // Create DB with some data
    let db_path = DBPath::new(&format!("{}db1", PATH_PREFIX));

    let mut opts = Options::default();
    opts.create_if_missing(true);
    let db = DB::open(&opts, &db_path).unwrap();

    db.put(b"k1", b"v1").unwrap();
    db.put(b"k2", b"v2").unwrap();
    db.put(b"k3", b"v3").unwrap();
    db.put(b"k4", b"v4").unwrap();

    // Create first checkpoint
    let cp1 = Checkpoint::new(&db).unwrap();
    let cp1_path = DBPath::new(&format!("{}cp1", PATH_PREFIX));
    cp1.create_checkpoint(&cp1_path).unwrap();

    // Verify checkpoint
    let cp = DB::open_default(&cp1_path).unwrap();

    assert_eq!(cp.get(b"k1").unwrap().unwrap(), b"v1");
    assert_eq!(cp.get(b"k2").unwrap().unwrap(), b"v2");
    assert_eq!(cp.get(b"k3").unwrap().unwrap(), b"v3");
    assert_eq!(cp.get(b"k4").unwrap().unwrap(), b"v4");

    // Change some existing keys
    db.put(b"k1", b"modified").unwrap();
    db.put(b"k2", b"changed").unwrap();

    // Add some new keys
    db.put(b"k5", b"v5").unwrap();
    db.put(b"k6", b"v6").unwrap();

    // Create another checkpoint
    let cp2 = Checkpoint::new(&db).unwrap();
    let cp2_path = DBPath::new(&format!("{}cp2", PATH_PREFIX));
    cp2.create_checkpoint(&cp2_path).unwrap();

    // Verify second checkpoint
    let cp = DB::open_default(&cp2_path).unwrap();

    assert_eq!(cp.get(b"k1").unwrap().unwrap(), b"modified");
    assert_eq!(cp.get(b"k2").unwrap().unwrap(), b"changed");
    assert_eq!(cp.get(b"k5").unwrap().unwrap(), b"v5");
    assert_eq!(cp.get(b"k6").unwrap().unwrap(), b"v6");
}

#[test]
fn test_checkpoint_outlive_db() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail/checkpoint_outlive_db.rs");
}
