// Copyright 2018 PingCAP, Inc.
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

use std::collections::HashMap;
use std::ops;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use tempdir::TempDir;

use rand::Rng;
use rocksdb::{
    CFHandle, ColumnFamilyOptions, CompactOptions, DBBottommostLevelCompaction, DBCompressionType,
    DBEntryType, DBOptions, Range, ReadOptions, SeekKey, TablePropertiesCollector,
    TablePropertiesCollectorFactory, TitanBlobIndex, TitanDBOptions, UserCollectedProperties,
    Writable, DB,
};

fn encode_u32(x: u32) -> Vec<u8> {
    let mut w = Vec::new();
    w.write_u32::<LittleEndian>(x).unwrap();
    w
}

fn decode_u32(mut x: &[u8]) -> u32 {
    x.read_u32::<LittleEndian>().unwrap()
}

#[derive(Default)]
struct TitanCollector {
    num_blobs: u32,
    num_entries: u32,
}

impl TitanCollector {
    fn add(&mut self, other: &TitanCollector) {
        self.num_blobs += other.num_blobs;
        self.num_entries += other.num_entries;
    }

    fn encode(&self) -> HashMap<Vec<u8>, Vec<u8>> {
        let mut props = HashMap::new();
        props.insert(vec![0], encode_u32(self.num_blobs));
        props.insert(vec![1], encode_u32(self.num_entries));
        props
    }

    fn decode(props: &UserCollectedProperties) -> TitanCollector {
        let mut c = TitanCollector::default();
        c.num_blobs = decode_u32(&props[&[0]]);
        c.num_entries = decode_u32(&props[&[1]]);
        c
    }
}

impl TablePropertiesCollector for TitanCollector {
    fn add(&mut self, _: &[u8], value: &[u8], entry_type: DBEntryType, _: u64, _: u64) {
        self.num_entries += 1;
        if let DBEntryType::BlobIndex = entry_type {
            self.num_blobs += 1;
            let index = TitanBlobIndex::decode(value).unwrap();
            assert!(index.file_number > 0);
            assert!(index.blob_size > 0);
        }
    }

    fn finish(&mut self) -> HashMap<Vec<u8>, Vec<u8>> {
        self.encode()
    }
}

#[derive(Default)]
struct TitanCollectorFactory {}

impl TablePropertiesCollectorFactory for TitanCollectorFactory {
    fn create_table_properties_collector(&mut self, _: u32) -> Box<TablePropertiesCollector> {
        Box::new(TitanCollector::default())
    }
}

fn check_table_properties(db: &DB, num_blobs: u32, num_entries: u32) {
    let cf = db.cf_handle("default").unwrap();
    let collection = db.get_properties_of_all_tables_cf(cf).unwrap();
    let mut res = TitanCollector::default();
    let props: HashMap<_, _> = collection.iter().collect();
    for (_, v) in &props {
        res.add(&TitanCollector::decode(v.user_collected_properties()));
    }
    assert_eq!(res.num_blobs, num_blobs);
    assert_eq!(res.num_entries, num_entries);
}

#[test]
fn test_titandb() {
    let max_value_size = 10;

    let path = TempDir::new("test_titandb").unwrap();
    let tdb_path = path.path().join("titandb");
    let mut tdb_opts = TitanDBOptions::new();
    tdb_opts.set_dirname(tdb_path.to_str().unwrap());
    tdb_opts.set_min_blob_size(max_value_size / 2 + 1);
    tdb_opts.set_blob_file_compression(DBCompressionType::No);
    tdb_opts.set_disable_background_gc(true);
    tdb_opts.set_purge_obsolete_files_period(10);

    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.set_titandb_options(&tdb_opts);
    let mut cf_opts = ColumnFamilyOptions::new();
    let f = TitanCollectorFactory::default();
    cf_opts.add_table_properties_collector_factory("titan-collector", Box::new(f));
    cf_opts.set_titandb_options(&tdb_opts);

    let db = DB::open_cf(
        opts,
        path.path().to_str().unwrap(),
        vec![("default", cf_opts)],
    )
    .unwrap();

    let n = 10;
    for i in 0..n {
        for size in 0..max_value_size {
            let k = (i * n + size) as u8;
            let v = vec![k; (size + 1) as usize];
            db.put(&[k], &v).unwrap();
        }
        db.flush(true).unwrap();
    }

    let mut iter = db.iter();
    iter.seek(SeekKey::Start);
    for i in 0..n {
        for j in 0..n {
            let k = (i * n + j) as u8;
            let v = vec![k; (j + 1) as usize];
            assert_eq!(db.get(&[k]).unwrap().unwrap(), &v);
            assert!(iter.valid());
            assert_eq!(iter.key(), &[k]);
            assert_eq!(iter.value(), v.as_slice());
            iter.next();
        }
    }

    let mut readopts = ReadOptions::new();
    readopts.set_titan_key_only(true);
    iter = db.iter_opt(readopts);
    iter.seek(SeekKey::Start);
    for i in 0..n {
        for j in 0..n {
            let k = (i * n + j) as u8;
            let v = vec![k; (j + 1) as usize];
            assert_eq!(db.get(&[k]).unwrap().unwrap(), &v);
            assert!(iter.valid());
            assert_eq!(iter.key(), &[k]);
            iter.next();
        }
    }

    let cf_handle = db.cf_handle("default").unwrap();
    readopts = ReadOptions::new();
    readopts.set_titan_key_only(true);
    iter = db.iter_cf_opt(&cf_handle, readopts);
    iter.seek(SeekKey::Start);
    for i in 0..n {
        for j in 0..n {
            let k = (i * n + j) as u8;
            let v = vec![k; (j + 1) as usize];
            assert_eq!(db.get(&[k]).unwrap().unwrap(), &v);
            assert!(iter.valid());
            assert_eq!(iter.key(), &[k]);
            iter.next();
        }
    }

    let num_entries = n as u32 * max_value_size as u32;
    check_table_properties(&db, num_entries / 2, num_entries);
}

#[test]
fn test_titan_blob_index() {
    let mut index = TitanBlobIndex::default();
    let mut rng = rand::thread_rng();
    index.file_number = rng.gen();
    index.blob_size = rng.gen();
    index.blob_offset = rng.gen();
    let value = index.encode();
    let index2 = TitanBlobIndex::decode(&value).unwrap();
    assert_eq!(index2.file_number, index.file_number);
    assert_eq!(index2.blob_size, index.blob_size);
    assert_eq!(index2.blob_offset, index.blob_offset);
}

// Generates a file with `range` and put it to the bottommost level.
fn generate_file_bottom_level(db: &DB, handle: &CFHandle, range: ops::Range<u32>) {
    for i in range {
        let k = format!("key{}", i);
        let v = format!("value{}", i);
        db.put_cf(handle, k.as_bytes(), v.as_bytes()).unwrap();
    }
    db.flush_cf(handle, true).unwrap();

    let opts = db.get_options_cf(handle);
    let mut compact_opts = CompactOptions::new();
    compact_opts.set_change_level(true);
    compact_opts.set_target_level(opts.get_num_levels() as i32 - 1);
    compact_opts.set_bottommost_level_compaction(DBBottommostLevelCompaction::Skip);
    db.compact_range_cf_opt(handle, &compact_opts, None, None);
}

#[test]
fn test_titan_delete_files_in_ranges() {
    let path = TempDir::new("_rust_rocksdb_test_titan_delete_files_in_multi_ranges").unwrap();
    let tdb_path = path.path().join("titandb");
    let mut tdb_opts = TitanDBOptions::new();
    tdb_opts.set_dirname(tdb_path.to_str().unwrap());
    tdb_opts.set_min_blob_size(0);

    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.set_titandb_options(&tdb_opts);
    let mut cf_opts = ColumnFamilyOptions::new();
    let f = TitanCollectorFactory::default();
    cf_opts.add_table_properties_collector_factory("titan-collector", Box::new(f));
    cf_opts.set_titandb_options(&tdb_opts);

    let db = DB::open_cf(
        opts,
        path.path().to_str().unwrap(),
        vec![("default", cf_opts)],
    )
    .unwrap();

    let cf_handle = db.cf_handle("default").unwrap();
    generate_file_bottom_level(&db, cf_handle, 0..3);
    generate_file_bottom_level(&db, cf_handle, 3..6);
    generate_file_bottom_level(&db, cf_handle, 6..9);

    // Delete files in multiple overlapped ranges.
    // File ["key0", "key2"], ["key3", "key5"] should have been deleted,
    // but file ["key6", "key8"] should not be deleted because "key8" is exclusive.
    let mut ranges = Vec::new();
    ranges.push(Range::new(b"key0", b"key4"));
    ranges.push(Range::new(b"key2", b"key6"));
    ranges.push(Range::new(b"key4", b"key8"));

    db.delete_files_in_ranges_cf(cf_handle, &ranges, false)
        .unwrap();

    // Check that ["key0", "key5"] have been deleted, but ["key6", "key8"] still exist.
    let mut readopts = ReadOptions::new();
    readopts.set_titan_key_only(true);
    let mut iter = db.iter_cf_opt(&cf_handle, readopts);
    iter.seek(SeekKey::Start);
    for i in 6..9 {
        assert!(iter.valid());
        let k = format!("key{}", i);
        assert_eq!(iter.key(), k.as_bytes());
        iter.next();
    }
    assert!(!iter.valid());

    // Delete the last file.
    let ranges = vec![Range::new(b"key6", b"key8")];
    db.delete_files_in_ranges_cf(cf_handle, &ranges, true)
        .unwrap();
    let mut iter = db.iter();
    iter.seek(SeekKey::Start);
    assert!(!iter.valid());
}
