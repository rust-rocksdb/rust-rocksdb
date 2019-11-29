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

use rocksdb::*;

use super::tempdir_with_prefix;

struct FixedPrefixTransform {
    pub prefix_len: usize,
}

impl SliceTransform for FixedPrefixTransform {
    fn transform<'a>(&mut self, key: &'a [u8]) -> &'a [u8] {
        &key[..self.prefix_len]
    }

    fn in_domain(&mut self, key: &[u8]) -> bool {
        key.len() >= self.prefix_len
    }
}

#[test]
fn test_prefix_extractor_compatibility() {
    let path = tempdir_with_prefix("_rust_rocksdb_prefix_extractor_compatibility");
    let keys = vec![
        b"k1-0", b"k1-1", b"k1-2", b"k1-3", b"k1-4", b"k1-5", b"k1-6", b"k1-7", b"k1-8",
    ];

    // create db with no prefix extractor, and insert data
    {
        let mut opts = DBOptions::new();
        opts.create_if_missing(true);
        let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
        let wopts = WriteOptions::new();

        // sst1 with no prefix bloom.
        db.put_opt(b"k1-0", b"a", &wopts).unwrap();
        db.put_opt(b"k1-1", b"b", &wopts).unwrap();
        db.put_opt(b"k1-2", b"c", &wopts).unwrap();
        db.flush(true /* sync */).unwrap(); // flush memtable to sst file.
    }

    // open db with prefix extractor, and insert data
    {
        let mut bbto = BlockBasedOptions::new();
        bbto.set_bloom_filter(10, false);
        bbto.set_whole_key_filtering(false);
        let mut opts = DBOptions::new();
        let mut cf_opts = ColumnFamilyOptions::new();
        opts.create_if_missing(false);
        cf_opts.set_block_based_table_factory(&bbto);
        cf_opts
            .set_prefix_extractor(
                "FixedPrefixTransform",
                Box::new(FixedPrefixTransform { prefix_len: 2 }),
            )
            .unwrap();
        // also create prefix bloom for memtable
        cf_opts.set_memtable_prefix_bloom_size_ratio(0.1 as f64);
        let db = DB::open_cf(
            opts,
            path.path().to_str().unwrap(),
            vec![("default", cf_opts)],
        )
        .unwrap();
        let wopts = WriteOptions::new();

        // sst2 with prefix bloom.
        db.put_opt(b"k1-3", b"a", &wopts).unwrap();
        db.put_opt(b"k1-4", b"b", &wopts).unwrap();
        db.put_opt(b"k1-5", b"c", &wopts).unwrap();
        db.flush(true /* sync */).unwrap(); // flush memtable to sst file.

        // memtable with prefix bloom.
        db.put_opt(b"k1-6", b"a", &wopts).unwrap();
        db.put_opt(b"k1-7", b"b", &wopts).unwrap();
        db.put_opt(b"k1-8", b"c", &wopts).unwrap();

        let mut iter = db.iter();
        iter.seek(SeekKey::Key(b"k1-0"));
        let mut key_count = 0;
        while iter.valid() {
            // If sst file has no prefix bloom, don't use prefix seek model.
            assert_eq!(keys[key_count], iter.key());
            key_count = key_count + 1;
            iter.next();
        }
        assert!(key_count == 9);
    }
}
