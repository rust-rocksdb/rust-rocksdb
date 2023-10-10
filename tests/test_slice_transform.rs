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

use rocksdb::{Options, SliceTransform, DB};
use util::{assert_iter, pair, DBPath};

#[test]
pub fn test_slice_transform() {
    let db_path = DBPath::new("_rust_rocksdb_slice_transform_test");
    {
        const A1: &[u8] = b"aaa1";
        const A2: &[u8] = b"aaa2";
        const B1: &[u8] = b"bbb1";
        const B2: &[u8] = b"bbb2";

        fn first_three(k: &[u8]) -> &[u8] {
            &k[..3]
        }

        let prefix_extractor = SliceTransform::create("first_three", first_three, None);

        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_prefix_extractor(prefix_extractor);

        let db = DB::open(&opts, &db_path).unwrap();

        assert!(db.put(A1, A1).is_ok());
        assert!(db.put(A2, A2).is_ok());
        assert!(db.put(B1, B1).is_ok());
        assert!(db.put(B2, B2).is_ok());

        assert_iter(db.prefix_iterator(b"aaa"), &[pair(A1, A1), pair(A2, A2)]);
        assert_iter(db.prefix_iterator(b"bbb"), &[pair(B1, B1), pair(B2, B2)]);
    }
}

#[test]
fn test_no_in_domain() {
    fn extract_suffix(slice: &[u8]) -> &[u8] {
        if slice.len() > 4 {
            &slice[slice.len() - 4..slice.len()]
        } else {
            slice
        }
    }

    let db_path = DBPath::new("_rust_rocksdb_prefix_test");
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_prefix_extractor(SliceTransform::create(
            "test slice transform",
            extract_suffix,
            None,
        ));
        opts.set_memtable_prefix_bloom_ratio(0.1);

        let db = DB::open(&opts, &db_path).unwrap();
        db.put(b"key_sfx1", b"a").unwrap();
        db.put(b"key_sfx2", b"b").unwrap();

        assert_eq!(db.get(b"key_sfx1").unwrap().unwrap(), b"a");
    }
}
