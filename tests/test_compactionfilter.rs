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

use rocksdb::{CompactionDecision, Options, DB};
use util::DBPath;

#[cfg(test)]
#[allow(unused_variables)]
fn test_filter(level: u32, key: &[u8], value: &[u8]) -> CompactionDecision {
    use self::CompactionDecision::*;
    match key.first() {
        Some(&b'_') => Remove,
        Some(&b'%') => Change(b"secret"),
        _ => Keep,
    }
}

#[test]
fn compaction_filter_test() {
    let path = DBPath::new("_rust_rocksdb_filter_test");
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.set_compaction_filter("test", test_filter);
    {
        let db = DB::open(&opts, &path).unwrap();
        let _ = db.put(b"k1", b"a");
        let _ = db.put(b"_k", b"b");
        let _ = db.put(b"%k", b"c");
        db.compact_range(None::<&[u8]>, None::<&[u8]>);
        assert_eq!(&*db.get(b"k1").unwrap().unwrap(), b"a");
        assert!(db.get(b"_k").unwrap().is_none());
        assert_eq!(&*db.get(b"%k").unwrap().unwrap(), b"secret");
    }
}
