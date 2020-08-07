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

use rocksdb::{Options, DB};
use util::DBPath;

#[test]
fn test_pinnable_slice() {
    let path = DBPath::new("_rust_rocksdb_pinnable_slice_test");

    let mut opts = Options::default();
    opts.create_if_missing(true);
    let db = DB::open(&opts, &path).unwrap();

    db.put(b"k1", b"value12345").unwrap();

    let result = db.get_pinned(b"k1");
    assert!(result.is_ok());

    let value = result.unwrap();
    assert!(value.is_some());

    let pinnable_slice = value.unwrap();

    assert_eq!(b"12345", &pinnable_slice[5..10]);
}
