// Copyright 2017 PingCAP, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// See the License for the specific language governing permissions and
// limitations under the License.

use rocksdb::{
    BlockBasedOptions, ColumnFamilyOptions, DBOptions, SeekKey, SliceTransform, Writable, DB,
};

use super::tempdir_with_prefix;

struct FixedPostfixTransform {
    pub postfix_len: usize,
}

impl SliceTransform for FixedPostfixTransform {
    fn transform<'a>(&mut self, key: &'a [u8]) -> &'a [u8] {
        let mid = key.len() - self.postfix_len;
        &key[..mid]
    }

    fn in_domain(&mut self, key: &[u8]) -> bool {
        key.len() >= self.postfix_len
    }
}

#[test]
fn test_slice_transform() {
    let path = tempdir_with_prefix("_rust_rocksdb_slice_transform_test");
    let mut opts = DBOptions::new();
    let mut cf_opts = ColumnFamilyOptions::new();

    let mut block_opts = BlockBasedOptions::new();
    block_opts.set_bloom_filter(10, false);
    block_opts.set_whole_key_filtering(false);

    cf_opts.set_block_based_table_factory(&block_opts);
    cf_opts.set_memtable_prefix_bloom_size_ratio(0.25);

    cf_opts
        .set_prefix_extractor("test", Box::new(FixedPostfixTransform { postfix_len: 2 }))
        .unwrap();
    opts.create_if_missing(true);

    let db = DB::open_cf(
        opts,
        path.path().to_str().unwrap(),
        vec![("default", cf_opts)],
    )
    .unwrap();
    let samples = vec![
        (b"key_01".to_vec(), b"1".to_vec()),
        (b"key_02".to_vec(), b"2".to_vec()),
        (b"key_0303".to_vec(), b"3".to_vec()),
        (b"key_0404".to_vec(), b"4".to_vec()),
    ];

    for &(ref k, ref v) in &samples {
        db.put(k, v).unwrap();
        assert_eq!(v.as_slice(), &*db.get(k).unwrap().unwrap());
    }

    let mut it = db.iter();

    let invalid_seeks = vec![
        b"key_".to_vec(),
        b"key_0".to_vec(),
        b"key_030".to_vec(),
        b"key_03000".to_vec(),
    ];

    for key in invalid_seeks {
        it.seek(SeekKey::Key(&key));
        assert!(!it.valid());
    }

    let valid_seeks = vec![
        (b"key_00".to_vec(), b"key_01".to_vec()),
        (b"key_03".to_vec(), b"key_0303".to_vec()),
        (b"key_0301".to_vec(), b"key_0303".to_vec()),
    ];

    for (key, expect_key) in valid_seeks {
        it.seek(SeekKey::Key(&key));
        assert!(it.valid());
        assert_eq!(it.key(), &*expect_key);
    }

    // TODO: support total_order mode and add test later.
}
