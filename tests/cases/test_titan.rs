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

use rand::Rng;
use rocksdb::{
    CFHandle, ColumnFamilyOptions, CompactOptions, DBBottommostLevelCompaction, DBCompressionType,
    DBEntryType, DBOptions, DBStatisticsHistogramType as HistogramType,
    DBStatisticsTickerType as TickerType, Range, ReadOptions, SeekKey, TablePropertiesCollector,
    TablePropertiesCollectorFactory, TitanBlobIndex, TitanDBOptions, UserCollectedProperties,
    Writable, WriteOptions, DB,
};

use super::tempdir_with_prefix;

fn encode_u32(x: u32) -> Vec<u8> {
    x.to_le_bytes().to_vec()
}

fn decode_u32(x: &[u8]) -> u32 {
    let mut dst = [0u8; 4];
    dst.copy_from_slice(&x[..4]);
    u32::from_le_bytes(dst)
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
    fn create_table_properties_collector(&mut self, _: u32) -> Box<dyn TablePropertiesCollector> {
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

    let path = tempdir_with_prefix("test_titandb");
    let tdb_path = path.path().join("titandb");
    let mut tdb_opts = TitanDBOptions::new();
    tdb_opts.set_dirname(tdb_path.to_str().unwrap());
    tdb_opts.set_min_blob_size(max_value_size / 2 + 1);
    tdb_opts.set_blob_file_compression(DBCompressionType::No);
    tdb_opts.set_disable_background_gc(true);
    tdb_opts.set_purge_obsolete_files_period(10);
    tdb_opts.set_level_merge(false);
    tdb_opts.set_range_merge(false);
    tdb_opts.set_max_sorted_runs(20);

    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.set_titandb_options(&tdb_opts);
    let mut cf_opts = ColumnFamilyOptions::new();
    let f = TitanCollectorFactory::default();
    cf_opts.set_titandb_options(&tdb_opts);
    cf_opts.add_table_properties_collector_factory("titan-collector", Box::new(f));

    let mut db = DB::open_cf(
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

    let mut cf_opts = ColumnFamilyOptions::new();
    cf_opts.set_num_levels(4);
    db.create_cf(("cf1", cf_opts)).unwrap();
    let cf1 = db.cf_handle("cf1").unwrap();
    assert_eq!(db.get_options_cf(cf1).get_num_levels(), 4);

    let mut iter = db.iter();
    iter.seek(SeekKey::Start).unwrap();
    for i in 0..n {
        for j in 0..n {
            let k = (i * n + j) as u8;
            let v = vec![k; (j + 1) as usize];
            assert_eq!(db.get(&[k]).unwrap().unwrap(), &v);
            assert!(iter.valid().unwrap());
            assert_eq!(iter.key(), &[k]);
            assert_eq!(iter.value(), v.as_slice());
            iter.next().unwrap();
        }
    }

    let mut readopts = ReadOptions::new();
    readopts.set_titan_key_only(true);
    iter = db.iter_opt(readopts);
    iter.seek(SeekKey::Start).unwrap();
    for i in 0..n {
        for j in 0..n {
            let k = (i * n + j) as u8;
            let v = vec![k; (j + 1) as usize];
            assert_eq!(db.get(&[k]).unwrap().unwrap(), &v);
            assert!(iter.valid().unwrap());
            assert_eq!(iter.key(), &[k]);
            iter.next().unwrap();
        }
    }

    let cf_handle = db.cf_handle("default").unwrap();
    readopts = ReadOptions::new();
    readopts.set_titan_key_only(true);
    iter = db.iter_cf_opt(&cf_handle, readopts);
    iter.seek(SeekKey::Start).unwrap();
    for i in 0..n {
        for j in 0..n {
            let k = (i * n + j) as u8;
            let v = vec![k; (j + 1) as usize];
            assert_eq!(db.get(&[k]).unwrap().unwrap(), &v);
            assert!(iter.valid().unwrap());
            assert_eq!(iter.key(), &[k]);
            iter.next().unwrap();
        }
    }

    let num_entries = n as u32 * max_value_size as u32;
    check_table_properties(&db, num_entries / 2, num_entries);
}

#[test]
fn test_titan_sequence_number() {
    let path = tempdir_with_prefix("test_titan_sequence_number");

    let tdb_path = path.path().join("titandb");
    let mut tdb_opts = TitanDBOptions::new();
    tdb_opts.set_dirname(tdb_path.to_str().unwrap());

    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.set_titandb_options(&tdb_opts);
    let db = DB::open(opts, path.path().to_str().unwrap()).unwrap();

    let snap = db.snapshot();
    let snap_seq = snap.get_sequence_number();
    let seq1 = db.get_latest_sequence_number();
    assert_eq!(snap_seq, seq1);

    db.put(b"key", b"value").unwrap();
    let seq2 = db.get_latest_sequence_number();
    assert!(seq2 > seq1);
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
    let path = tempdir_with_prefix("_rust_rocksdb_test_titan_delete_files_in_multi_ranges");
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
    iter.seek(SeekKey::Start).unwrap();
    for i in 6..9 {
        assert!(iter.valid().unwrap());
        let k = format!("key{}", i);
        assert_eq!(iter.key(), k.as_bytes());
        iter.next().unwrap();
    }
    assert!(!iter.valid().unwrap());

    // Delete the last file.
    let ranges = vec![Range::new(b"key6", b"key8")];
    db.delete_files_in_ranges_cf(cf_handle, &ranges, true)
        .unwrap();
    let mut iter = db.iter();
    iter.seek(SeekKey::Start).unwrap();
    assert!(!iter.valid().unwrap());
}

#[test]
fn test_get_blob_cache_usage() {
    let path = tempdir_with_prefix("_rust_rocksdb_set_blob_cache");
    let tdb_path = path.path().join("titandb");
    let mut tdb_opts = TitanDBOptions::new();
    tdb_opts.set_dirname(tdb_path.to_str().unwrap());
    tdb_opts.set_min_blob_size(0);
    tdb_opts.set_blob_cache(16 * 1024 * 1024, -1, false, 0.0);

    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.set_titandb_options(&tdb_opts);
    let mut cf_opts = ColumnFamilyOptions::new();
    cf_opts.set_titandb_options(&tdb_opts);
    assert_eq!(cf_opts.get_blob_cache_usage(), 0);

    let db = DB::open_cf(
        opts,
        path.path().to_str().unwrap(),
        vec![("default", cf_opts)],
    )
    .unwrap();

    for i in 0..200 {
        db.put(format!("k_{}", i).as_bytes(), b"v").unwrap();
    }
    db.flush(true).unwrap();
    for i in 0..200 {
        db.get(format!("k_{}", i).as_bytes()).unwrap();
    }

    assert!(db.get_options().get_blob_cache_usage() > 0);
}

#[test]
fn test_blob_cache_capacity() {
    let path = tempdir_with_prefix("_rust_rocksdb_set_and_get_blob_cache_capacity");
    let tdb_path = path.path().join("titandb");
    let mut tdb_opts = TitanDBOptions::new();
    tdb_opts.set_dirname(tdb_path.to_str().unwrap());
    tdb_opts.set_min_blob_size(0);
    tdb_opts.set_blob_cache(16 * 1024 * 1024, -1, false, 0.0);

    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    opts.set_titandb_options(&tdb_opts);
    let mut cf_opts = ColumnFamilyOptions::new();
    cf_opts.set_titandb_options(&tdb_opts);

    let db = DB::open_cf(
        opts,
        path.path().to_str().unwrap(),
        vec![("default", cf_opts)],
    )
    .unwrap();

    assert_eq!(db.get_options().get_blob_cache_capacity(), 16 * 1024 * 1024);

    let opt = db.get_options();
    opt.set_blob_cache_capacity(32 * 1024 * 1024).unwrap();

    assert_eq!(db.get_options().get_blob_cache_capacity(), 32 * 1024 * 1024);
}

#[test]
fn test_titan_statistics() {
    let path = tempdir_with_prefix("_rust_rocksdb_statistics");
    let mut tdb_opts = TitanDBOptions::new();
    tdb_opts.set_min_blob_size(0);
    let mut opts = DBOptions::new();
    opts.set_titandb_options(&tdb_opts);
    opts.enable_statistics(true);
    opts.create_if_missing(true);
    let mut cf_opts = ColumnFamilyOptions::new();
    cf_opts.set_titandb_options(&tdb_opts);

    let db = DB::open_cf(
        opts,
        path.path().to_str().unwrap(),
        vec![("default", cf_opts)],
    )
    .unwrap();
    let wopts = WriteOptions::new();

    db.put_opt(b"k0", b"a", &wopts).unwrap();
    db.put_opt(b"k1", b"b", &wopts).unwrap();
    db.put_opt(b"k2", b"c", &wopts).unwrap();
    db.flush(true /* sync */).unwrap(); // flush memtable to sst file.
    assert_eq!(db.get(b"k0").unwrap().unwrap(), b"a");
    assert_eq!(db.get(b"k1").unwrap().unwrap(), b"b");
    assert_eq!(db.get(b"k2").unwrap().unwrap(), b"c");

    assert!(db.get_statistics_ticker_count(TickerType::TitanNumGet) > 0);
    assert!(db.get_and_reset_statistics_ticker_count(TickerType::TitanNumGet) > 0);
    assert_eq!(db.get_statistics_ticker_count(TickerType::TitanNumGet), 0);
    assert!(db
        .get_statistics_histogram_string(HistogramType::TitanGetMicros)
        .is_some());
    assert!(db
        .get_statistics_histogram(HistogramType::TitanGetMicros)
        .is_some());

    let get_micros = db
        .get_statistics_histogram(HistogramType::TitanGetMicros)
        .unwrap();
    assert!(get_micros.max > 0.0);
    db.reset_statistics();
    let get_micros = db
        .get_statistics_histogram(HistogramType::TitanGetMicros)
        .unwrap();
    assert_eq!(
        db.get_property_int("rocksdb.titandb.num-discardable-ratio-le0-file")
            .unwrap(),
        1
    );

    assert_eq!(
        db.get_property_int("rocksdb.titandb.num-blob-files-at-level0")
            .unwrap(),
        1
    );

    assert_eq!(get_micros.max, 0.0);
}
