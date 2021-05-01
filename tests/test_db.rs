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

use std::{mem, sync::Arc, thread, time::Duration};

use pretty_assertions::assert_eq;

use rocksdb::{
    perf::get_memory_usage_stats, BlockBasedOptions, BottommostLevelCompaction, Cache,
    CompactOptions, DBCompactionStyle, DBWithThreadMode, Env, Error, FifoCompactOptions,
    IteratorMode, MultiThreaded, Options, PerfContext, PerfMetric, ReadOptions, SingleThreaded,
    SliceTransform, Snapshot, UniversalCompactOptions, UniversalCompactionStopStyle, WriteBatch,
    DB,
};
use util::DBPath;

#[test]
fn external() {
    let path = DBPath::new("_rust_rocksdb_externaltest");

    {
        let db = DB::open_default(&path).unwrap();

        assert!(db.put(b"k1", b"v1111").is_ok());

        let r: Result<Option<Vec<u8>>, Error> = db.get(b"k1");

        assert_eq!(r.unwrap().unwrap(), b"v1111");
        assert!(db.delete(b"k1").is_ok());
        assert!(db.get(b"k1").unwrap().is_none());
    }
}

#[test]
fn db_vector_as_ref_byte_slice() {
    let path = DBPath::new("_rust_rocksdb_db_vector_as_ref_byte_slice");

    {
        let db = DB::open_default(&path).unwrap();

        assert!(db.put(b"k1", b"v1111").is_ok());

        let result = db.get(b"k1").unwrap().unwrap();

        assert_eq!(get_byte_slice(&result), b"v1111");
    }
}

fn get_byte_slice<T: AsRef<[u8]>>(source: &'_ T) -> &'_ [u8] {
    source.as_ref()
}

#[test]
fn errors_do_stuff() {
    let path = DBPath::new("_rust_rocksdb_error");
    let _db = DB::open_default(&path).unwrap();
    let opts = Options::default();
    // The DB will still be open when we try to destroy it and the lock should fail.
    match DB::destroy(&opts, &path) {
        Err(s) => {
            let message = s.to_string();
            assert!(message.find("IO error:").is_some());
            assert!(message.find("_rust_rocksdb_error").is_some());
            assert!(message.find("/LOCK:").is_some());
        }
        Ok(_) => panic!("should fail"),
    }
}

#[test]
fn writebatch_works() {
    let path = DBPath::new("_rust_rocksdb_writebacktest");
    {
        let db = DB::open_default(&path).unwrap();
        {
            // test put
            let mut batch = WriteBatch::default();
            assert!(db.get(b"k1").unwrap().is_none());
            assert_eq!(batch.len(), 0);
            assert!(batch.is_empty());
            batch.put(b"k1", b"v1111");
            batch.put(b"k2", b"v2222");
            batch.put(b"k3", b"v3333");
            assert_eq!(batch.len(), 3);
            assert!(!batch.is_empty());
            assert!(db.get(b"k1").unwrap().is_none());
            let p = db.write(batch);
            assert!(p.is_ok());
            let r: Result<Option<Vec<u8>>, Error> = db.get(b"k1");
            assert_eq!(r.unwrap().unwrap(), b"v1111");
        }
        {
            // test delete
            let mut batch = WriteBatch::default();
            batch.delete(b"k1");
            assert_eq!(batch.len(), 1);
            assert!(!batch.is_empty());
            let p = db.write(batch);
            assert!(p.is_ok());
            assert!(db.get(b"k1").unwrap().is_none());
        }
        {
            // test delete_range
            let mut batch = WriteBatch::default();
            batch.delete_range(b"k2", b"k4");
            assert_eq!(batch.len(), 1);
            assert!(!batch.is_empty());
            let p = db.write(batch);
            assert!(p.is_ok());
            assert!(db.get(b"k2").unwrap().is_none());
            assert!(db.get(b"k3").unwrap().is_none());
        }
        {
            // test size_in_bytes
            let mut batch = WriteBatch::default();
            let before = batch.size_in_bytes();
            batch.put(b"k1", b"v1234567890");
            let after = batch.size_in_bytes();
            assert!(before + 10 <= after);
        }
    }
}

#[test]
fn iterator_test() {
    let path = DBPath::new("_rust_rocksdb_iteratortest");
    {
        let data = [(b"k1", b"v1111"), (b"k2", b"v2222"), (b"k3", b"v3333")];
        let db = DB::open_default(&path).unwrap();

        for (key, value) in &data {
            assert!(db.put(key, value).is_ok());
        }

        let iter = db.iterator(IteratorMode::Start);

        for (idx, (db_key, db_value)) in iter.enumerate() {
            let (key, value) = data[idx];
            assert_eq!((&key[..], &value[..]), (db_key.as_ref(), db_value.as_ref()));
        }
    }
}

#[test]
fn iterator_test_past_end() {
    let path = DBPath::new("_rust_rocksdb_iteratortest_past_end");
    {
        let db = DB::open_default(&path).unwrap();
        db.put(b"k1", b"v1111").unwrap();
        let mut iter = db.iterator(IteratorMode::Start);
        assert!(iter.next().is_some());
        assert!(iter.next().is_none());
        assert!(iter.next().is_none());
    }
}

#[test]
fn iterator_test_tailing() {
    let path = DBPath::new("_rust_rocksdb_iteratortest_tailing");
    {
        let data = [(b"k1", b"v1"), (b"k2", b"v2"), (b"k3", b"v3")];
        let mut ro = ReadOptions::default();
        ro.set_tailing(true);
        let db = DB::open_default(&path).unwrap();

        let mut data_iter = data.iter();
        let (k, v) = data_iter.next().unwrap();
        let r = db.put(k, v);
        assert!(r.is_ok());

        let tail_iter = db.iterator_opt(IteratorMode::Start, ro);
        for (k, v) in data_iter {
            let r = db.put(k, v);
            assert!(r.is_ok());
        }

        let mut tot = 0;
        for (i, (k, v)) in tail_iter.enumerate() {
            assert_eq!(
                (k.to_vec(), v.to_vec()),
                (data[i].0.to_vec(), data[i].1.to_vec())
            );
            tot += 1;
        }
        assert_eq!(tot, data.len());
    }
}

#[test]
fn iterator_test_upper_bound() {
    let path = DBPath::new("_rust_rocksdb_iteratortest_upper_bound");
    {
        let db = DB::open_default(&path).unwrap();
        db.put(b"k1", b"v1").unwrap();
        db.put(b"k2", b"v2").unwrap();
        db.put(b"k3", b"v3").unwrap();
        db.put(b"k4", b"v4").unwrap();
        db.put(b"k5", b"v5").unwrap();

        let mut readopts = ReadOptions::default();
        readopts.set_iterate_upper_bound(b"k4".to_vec());

        let iter = db.iterator_opt(IteratorMode::Start, readopts);
        let expected: Vec<_> = vec![(b"k1", b"v1"), (b"k2", b"v2"), (b"k3", b"v3")]
            .into_iter()
            .map(|(k, v)| (k.to_vec().into_boxed_slice(), v.to_vec().into_boxed_slice()))
            .collect();
        assert_eq!(expected, iter.collect::<Vec<_>>());
    }
}

#[test]
fn iterator_test_lower_bound() {
    let path = DBPath::new("_rust_rocksdb_iteratortest_lower_bound");
    {
        let db = DB::open_default(&path).unwrap();
        db.put(b"k1", b"v1").unwrap();
        db.put(b"k2", b"v2").unwrap();
        db.put(b"k3", b"v3").unwrap();
        db.put(b"k4", b"v4").unwrap();
        db.put(b"k5", b"v5").unwrap();

        let mut readopts = ReadOptions::default();
        readopts.set_iterate_lower_bound(b"k4".to_vec());

        let iter = db.iterator_opt(IteratorMode::Start, readopts);
        let expected: Vec<_> = vec![(b"k4", b"v4"), (b"k5", b"v5")]
            .into_iter()
            .map(|(k, v)| (k.to_vec().into_boxed_slice(), v.to_vec().into_boxed_slice()))
            .collect();
        assert_eq!(expected, iter.collect::<Vec<_>>());
    }
}

#[test]
fn snapshot_test() {
    let path = DBPath::new("_rust_rocksdb_snapshottest");
    {
        let db = DB::open_default(&path).unwrap();

        assert!(db.put(b"k1", b"v1111").is_ok());

        let snap = db.snapshot();
        assert_eq!(snap.get(b"k1").unwrap().unwrap(), b"v1111");

        assert!(db.put(b"k2", b"v2222").is_ok());

        assert!(db.get(b"k2").unwrap().is_some());
        assert!(snap.get(b"k2").unwrap().is_none());
    }
}

#[derive(Clone)]
struct SnapshotWrapper {
    snapshot: Arc<Snapshot<'static>>,
}

impl SnapshotWrapper {
    fn new(db: &DB) -> Self {
        Self {
            snapshot: Arc::new(unsafe { mem::transmute(db.snapshot()) }),
        }
    }

    fn check<K>(&self, key: K, value: &[u8]) -> bool
    where
        K: AsRef<[u8]>,
    {
        self.snapshot.get(key).unwrap().unwrap() == value
    }
}

#[test]
fn sync_snapshot_test() {
    let path = DBPath::new("_rust_rocksdb_sync_snapshottest");
    let db = DB::open_default(&path).unwrap();

    assert!(db.put(b"k1", b"v1").is_ok());
    assert!(db.put(b"k2", b"v2").is_ok());

    let wrapper = SnapshotWrapper::new(&db);
    let wrapper_1 = wrapper.clone();
    let handler_1 = thread::spawn(move || wrapper_1.check("k1", b"v1"));
    let handler_2 = thread::spawn(move || wrapper.check("k2", b"v2"));

    assert!(handler_1.join().unwrap());
    assert!(handler_2.join().unwrap());
}

#[test]
fn set_option_test() {
    let path = DBPath::new("_rust_rocksdb_set_optionstest");
    {
        let db = DB::open_default(&path).unwrap();
        // set an option to valid values
        assert!(db
            .set_options(&[("disable_auto_compactions", "true")])
            .is_ok());
        assert!(db
            .set_options(&[("disable_auto_compactions", "false")])
            .is_ok());
        // invalid names/values should result in an error
        assert!(db
            .set_options(&[("disable_auto_compactions", "INVALID_VALUE")])
            .is_err());
        assert!(db
            .set_options(&[("INVALID_NAME", "INVALID_VALUE")])
            .is_err());
        // option names/values must not contain NULLs
        assert!(db
            .set_options(&[("disable_auto_compactions", "true\0")])
            .is_err());
        assert!(db
            .set_options(&[("disable_auto_compactions\0", "true")])
            .is_err());
        // empty options are not allowed
        assert!(db.set_options(&[]).is_err());
        // multiple options can be set in a single API call
        let multiple_options = [
            ("paranoid_file_checks", "true"),
            ("report_bg_io_stats", "true"),
        ];
        db.set_options(&multiple_options).unwrap();
    }
}

#[test]
fn set_option_cf_test() {
    let path = DBPath::new("_rust_rocksdb_set_options_cftest");
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        let db = DB::open_cf(&opts, &path, vec!["cf1"]).unwrap();
        let cf = db.cf_handle("cf1").unwrap();
        // set an option to valid values
        assert!(db
            .set_options_cf(cf, &[("disable_auto_compactions", "true")])
            .is_ok());
        assert!(db
            .set_options_cf(cf, &[("disable_auto_compactions", "false")])
            .is_ok());
        // invalid names/values should result in an error
        assert!(db
            .set_options_cf(cf, &[("disable_auto_compactions", "INVALID_VALUE")])
            .is_err());
        assert!(db
            .set_options_cf(cf, &[("INVALID_NAME", "INVALID_VALUE")])
            .is_err());
        // option names/values must not contain NULLs
        assert!(db
            .set_options_cf(cf, &[("disable_auto_compactions", "true\0")])
            .is_err());
        assert!(db
            .set_options_cf(cf, &[("disable_auto_compactions\0", "true")])
            .is_err());
        // empty options are not allowed
        assert!(db.set_options_cf(cf, &[]).is_err());
        // multiple options can be set in a single API call
        let multiple_options = [
            ("paranoid_file_checks", "true"),
            ("report_bg_io_stats", "true"),
        ];
        db.set_options(&multiple_options).unwrap();
    }
}

#[test]
fn test_sequence_number() {
    let path = DBPath::new("_rust_rocksdb_test_sequence_number");
    {
        let db = DB::open_default(&path).unwrap();
        assert_eq!(db.latest_sequence_number(), 0);
        let _ = db.put(b"key", b"value");
        assert_eq!(db.latest_sequence_number(), 1);
    }
}

struct OperationCounts {
    puts: usize,
    deletes: usize,
}

impl rocksdb::WriteBatchIterator for OperationCounts {
    fn put(&mut self, _key: Box<[u8]>, _value: Box<[u8]>) {
        self.puts += 1;
    }
    fn delete(&mut self, _key: Box<[u8]>) {
        self.deletes += 1;
    }
}

#[test]
fn test_get_updates_since_empty() {
    let path = DBPath::new("_rust_rocksdb_test_get_updates_since_empty");
    let db = DB::open_default(&path).unwrap();
    // get_updates_since() on an empty database
    let mut iter = db.get_updates_since(0).unwrap();
    assert!(iter.next().is_none());
}

#[test]
fn test_get_updates_since_multiple_batches() {
    let path = DBPath::new("_rust_rocksdb_test_get_updates_since_multiple_batches");
    let db = DB::open_default(&path).unwrap();
    // add some records and collect sequence numbers,
    // verify 3 batches of 1 put each were done
    db.put(b"key1", b"value1").unwrap();
    let seq1 = db.latest_sequence_number();
    db.put(b"key2", b"value2").unwrap();
    db.put(b"key3", b"value3").unwrap();
    db.put(b"key4", b"value4").unwrap();
    let mut iter = db.get_updates_since(seq1).unwrap();
    let mut counts = OperationCounts {
        puts: 0,
        deletes: 0,
    };
    let (seq, batch) = iter.next().unwrap();
    assert_eq!(seq, 2);
    batch.iterate(&mut counts);
    let (seq, batch) = iter.next().unwrap();
    assert_eq!(seq, 3);
    batch.iterate(&mut counts);
    let (seq, batch) = iter.next().unwrap();
    assert_eq!(seq, 4);
    batch.iterate(&mut counts);
    assert!(iter.next().is_none());
    assert_eq!(counts.puts, 3);
    assert_eq!(counts.deletes, 0);
}

#[test]
fn test_get_updates_since_one_batch() {
    let path = DBPath::new("_rust_rocksdb_test_get_updates_since_one_batch");
    let db = DB::open_default(&path).unwrap();
    db.put(b"key2", b"value2").unwrap();
    // some puts and deletes in a single batch,
    // verify 1 put and 1 delete were done
    let seq1 = db.latest_sequence_number();
    assert_eq!(seq1, 1);
    let mut batch = WriteBatch::default();
    batch.put(b"key1", b"value1");
    batch.delete(b"key2");
    db.write(batch).unwrap();
    assert_eq!(db.latest_sequence_number(), 3);
    let mut iter = db.get_updates_since(seq1).unwrap();
    let mut counts = OperationCounts {
        puts: 0,
        deletes: 0,
    };
    let (seq, batch) = iter.next().unwrap();
    assert_eq!(seq, 2);
    batch.iterate(&mut counts);
    assert!(iter.next().is_none());
    assert_eq!(counts.puts, 1);
    assert_eq!(counts.deletes, 1);
}

#[test]
fn test_get_updates_since_nothing() {
    let path = DBPath::new("_rust_rocksdb_test_get_updates_since_nothing");
    let db = DB::open_default(&path).unwrap();
    // get_updates_since() with no new changes
    db.put(b"key1", b"value1").unwrap();
    let seq1 = db.latest_sequence_number();
    let mut iter = db.get_updates_since(seq1).unwrap();
    assert!(iter.next().is_none());
}

#[test]
fn test_get_updates_since_out_of_range() {
    let path = DBPath::new("_rust_rocksdb_test_get_updates_since_out_of_range");
    let db = DB::open_default(&path).unwrap();
    db.put(b"key1", b"value1").unwrap();
    // get_updates_since() with an out of bounds sequence number
    let result = db.get_updates_since(1000);
    assert!(result.is_err());
}

#[test]
fn test_open_as_secondary() {
    let primary_path = DBPath::new("_rust_rocksdb_test_open_as_secondary_primary");

    let db = DB::open_default(&primary_path).unwrap();
    db.put(b"key1", b"value1").unwrap();

    let mut opts = Options::default();
    opts.set_max_open_files(-1);

    let secondary_path = DBPath::new("_rust_rocksdb_test_open_as_secondary_secondary");
    let secondary = DB::open_as_secondary(&opts, &primary_path, &secondary_path).unwrap();

    let result = secondary.get(b"key1").unwrap().unwrap();
    assert_eq!(get_byte_slice(&result), b"value1");

    db.put(b"key1", b"value2").unwrap();
    assert!(secondary.try_catch_up_with_primary().is_ok());

    let result = secondary.get(b"key1").unwrap().unwrap();
    assert_eq!(get_byte_slice(&result), b"value2");
}

#[test]
fn test_open_with_ttl() {
    let path = DBPath::new("_rust_rocksdb_test_open_with_ttl");

    let mut opts = Options::default();
    opts.create_if_missing(true);
    let db = DB::open_with_ttl(&opts, &path, Duration::from_secs(1)).unwrap();
    db.put(b"key1", b"value1").unwrap();

    thread::sleep(Duration::from_secs(2));
    // Trigger a manual compaction, this will check the TTL filter
    // in the database and drop all expired entries.
    db.compact_range(None::<&[u8]>, None::<&[u8]>);
    assert!(db.get(b"key1").unwrap().is_none());
}

#[test]
fn test_open_cf_with_ttl() {
    let path = DBPath::new("_rust_rocksdb_test_open_cf_with_ttl");

    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.create_missing_column_families(true);
    let db = DB::open_cf_with_ttl(&opts, &path, &["test_cf"], Duration::from_secs(1)).unwrap();
    let cf = db.cf_handle("test_cf").unwrap();
    db.put_cf(cf, b"key1", b"value1").unwrap();

    thread::sleep(Duration::from_secs(2));
    // Trigger a manual compaction, this will check the TTL filter
    // in the database and drop all expired entries.
    db.compact_range_cf(cf, None::<&[u8]>, None::<&[u8]>);

    assert!(db.get_cf(cf, b"key1").unwrap().is_none());
}

#[test]
fn test_open_as_single_threaded() {
    let primary_path = DBPath::new("_rust_rocksdb_test_open_as_single_threaded");

    let mut db = DBWithThreadMode::<SingleThreaded>::open_default(&primary_path).unwrap();
    let db_ref1 = &mut db;
    let opts = Options::default();
    db_ref1.create_cf("cf1", &opts).unwrap();
}

#[test]
fn test_open_with_multiple_refs_as_multi_threaded() {
    // This tests multiple references can be allowed while creating column families
    let primary_path = DBPath::new("_rust_rocksdb_test_open_as_multi_threaded");

    let db = DBWithThreadMode::<MultiThreaded>::open_default(&primary_path).unwrap();
    let db_ref1 = &db;
    let db_ref2 = &db;
    let opts = Options::default();
    db_ref1.create_cf("cf1", &opts).unwrap();
    db_ref2.create_cf("cf2", &opts).unwrap();
}

#[test]
fn test_open_with_multiple_refs_as_single_threaded() {
    // This tests multiple references CANNOT be allowed while creating column families
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail/open_with_multiple_refs_as_single_threaded.rs");
}

#[test]
fn compact_range_test() {
    let path = DBPath::new("_rust_rocksdb_compact_range_test");
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        // set compaction style
        {
            let mut uni_co_opts = UniversalCompactOptions::default();
            uni_co_opts.set_size_ratio(2);
            uni_co_opts.set_stop_style(UniversalCompactionStopStyle::Total);
            opts.set_compaction_style(DBCompactionStyle::Universal);
            opts.set_universal_compaction_options(&uni_co_opts);
        }

        // set compaction options
        let mut compact_opts = CompactOptions::default();
        compact_opts.set_exclusive_manual_compaction(true);
        compact_opts.set_target_level(1);
        compact_opts.set_change_level(true);
        compact_opts.set_bottommost_level_compaction(BottommostLevelCompaction::ForceOptimized);

        // put and compact column family cf1
        let cfs = vec!["cf1"];
        let db = DB::open_cf(&opts, &path, cfs).unwrap();
        let cf1 = db.cf_handle("cf1").unwrap();
        db.put_cf(cf1, b"k1", b"v1").unwrap();
        db.put_cf(cf1, b"k2", b"v2").unwrap();
        db.put_cf(cf1, b"k3", b"v3").unwrap();
        db.put_cf(cf1, b"k4", b"v4").unwrap();
        db.put_cf(cf1, b"k5", b"v5").unwrap();
        db.compact_range_cf(cf1, Some(b"k2"), Some(b"k4"));
        db.compact_range_cf_opt(cf1, Some(b"k1"), None::<&str>, &compact_opts);

        // put and compact default column family
        db.put(b"k1", b"v1").unwrap();
        db.put(b"k2", b"v2").unwrap();
        db.put(b"k3", b"v3").unwrap();
        db.put(b"k4", b"v4").unwrap();
        db.put(b"k5", b"v5").unwrap();
        db.compact_range(Some(b"k3"), None::<&str>);
        db.compact_range_opt(None::<&str>, Some(b"k5"), &compact_opts);
    }
}

#[test]
fn fifo_compaction_test() {
    let path = DBPath::new("_rust_rocksdb_fifo_compaction_test");
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        // set compaction style
        {
            let mut fifo_co_opts = FifoCompactOptions::default();
            fifo_co_opts.set_max_table_files_size(4 << 10); // 4KB
            opts.set_compaction_style(DBCompactionStyle::Fifo);
            opts.set_fifo_compaction_options(&fifo_co_opts);
        }

        // put and compact column family cf1
        let cfs = vec!["cf1"];
        let db = DB::open_cf(&opts, &path, cfs).unwrap();
        let cf1 = db.cf_handle("cf1").unwrap();
        db.put_cf(cf1, b"k1", b"v1").unwrap();
        db.put_cf(cf1, b"k2", b"v2").unwrap();
        db.put_cf(cf1, b"k3", b"v3").unwrap();
        db.put_cf(cf1, b"k4", b"v4").unwrap();
        db.put_cf(cf1, b"k5", b"v5").unwrap();
        db.compact_range_cf(cf1, Some(b"k2"), Some(b"k4"));

        // check stats
        let ctx = PerfContext::default();

        let block_cache_hit_count = ctx.metric(PerfMetric::BlockCacheHitCount);
        if block_cache_hit_count > 0 {
            let expect = format!("block_cache_hit_count = {}", block_cache_hit_count);
            assert!(ctx.report(true).contains(&expect));
        }
    }
}

#[test]
fn env_and_dbpaths_test() {
    let path = DBPath::new("_rust_rocksdb_dbpath_test");
    let path1 = DBPath::new("_rust_rocksdb_dbpath_test_1");
    let path2 = DBPath::new("_rust_rocksdb_dbpath_test_2");
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        {
            let mut env = Env::default().unwrap();
            env.lower_high_priority_thread_pool_cpu_priority();
            opts.set_env(&env);
        }

        {
            let mut paths = Vec::new();
            paths.push(rocksdb::DBPath::new(&path1, 20 << 20).unwrap());
            paths.push(rocksdb::DBPath::new(&path2, 30 << 20).unwrap());
            opts.set_db_paths(&paths);
        }

        let db = DB::open(&opts, &path).unwrap();
        db.put(b"k1", b"v1").unwrap();
        assert_eq!(db.get(b"k1").unwrap().unwrap(), b"v1");
    }
}

#[test]
fn prefix_extract_and_iterate_test() {
    let path = DBPath::new("_rust_rocksdb_prefix_extract_and_iterate");
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        opts.set_prefix_extractor(SliceTransform::create_fixed_prefix(2));

        let db = DB::open(&opts, &path).unwrap();
        db.put(b"p1_k1", b"v1").unwrap();
        db.put(b"p2_k2", b"v2").unwrap();
        db.put(b"p1_k3", b"v3").unwrap();
        db.put(b"p1_k4", b"v4").unwrap();
        db.put(b"p2_k5", b"v5").unwrap();

        let mut readopts = ReadOptions::default();
        readopts.set_prefix_same_as_start(true);
        readopts.set_iterate_lower_bound(b"p1".to_vec());
        readopts.set_pin_data(true);

        let iter = db.iterator_opt(IteratorMode::Start, readopts);
        let expected: Vec<_> = vec![(b"p1_k1", b"v1"), (b"p1_k3", b"v3"), (b"p1_k4", b"v4")]
            .into_iter()
            .map(|(k, v)| (k.to_vec().into_boxed_slice(), v.to_vec().into_boxed_slice()))
            .collect();
        assert_eq!(expected, iter.collect::<Vec<_>>());
    }
}

#[test]
fn get_with_cache_and_bulkload_test() {
    let path = DBPath::new("_rust_rocksdb_get_with_cache_and_bulkload_test");
    let log_path = DBPath::new("_rust_rocksdb_log_path_test");

    // create options
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.create_missing_column_families(true);
    opts.set_wal_bytes_per_sync(8 << 10); // 8KB
    opts.set_writable_file_max_buffer_size(512 << 10); // 512KB
    opts.set_enable_write_thread_adaptive_yield(true);
    opts.set_unordered_write(true);
    opts.set_max_subcompactions(2);
    opts.set_max_background_jobs(4);
    opts.set_use_adaptive_mutex(true);
    opts.set_db_log_dir(&log_path);
    opts.set_memtable_whole_key_filtering(true);
    opts.set_dump_malloc_stats(true);

    // trigger all sst files in L1/2 instead of L0
    opts.set_max_bytes_for_level_base(64 << 10); // 64KB

    {
        // set block based table and cache
        let cache = Cache::new_lru_cache(512 << 10).unwrap();
        assert_eq!(cache.get_usage(), 0);
        let mut block_based_opts = BlockBasedOptions::default();
        block_based_opts.set_block_cache(&cache);
        block_based_opts.set_cache_index_and_filter_blocks(true);
        opts.set_block_based_table_factory(&block_based_opts);

        // open db
        let db = DB::open(&opts, &path).unwrap();

        // write a lot
        let mut batch = WriteBatch::default();
        for i in 0..10_000 {
            batch.put(format!("{:0>4}", i).as_bytes(), b"v");
        }
        assert!(db.write(batch).is_ok());

        // flush memory table to sst and manual compaction
        assert!(db.flush().is_ok());
        db.compact_range(Some(format!("{:0>4}", 0).as_bytes()), None::<Vec<u8>>);

        // get -> trigger caching
        let _ = db.get(b"1");
        assert!(cache.get_usage() > 0);

        // get approximated memory usage
        let mem_usage = get_memory_usage_stats(Some(&[&db]), None).unwrap();
        assert!(mem_usage.mem_table_total > 0);

        // get approximated cache usage
        let mem_usage = get_memory_usage_stats(None, Some(&[&cache])).unwrap();
        assert!(mem_usage.cache_total > 0);
    }

    // bulk loading
    {
        // open db
        let db = DB::open(&opts, &path).unwrap();

        // try to get key
        let iter = db.iterator(IteratorMode::Start);
        for (expected, (k, _)) in iter.enumerate() {
            assert_eq!(k.as_ref(), format!("{:0>4}", expected).as_bytes());
        }

        // check live files (sst files meta)
        let livefiles = db.live_files().unwrap();
        assert_eq!(livefiles.len(), 1);
        livefiles.iter().for_each(|f| {
            assert_eq!(f.level, 2);
            assert!(!f.name.is_empty());
            assert_eq!(
                f.start_key.as_ref().unwrap().as_slice(),
                format!("{:0>4}", 0).as_bytes()
            );
            assert_eq!(
                f.end_key.as_ref().unwrap().as_slice(),
                format!("{:0>4}", 9999).as_bytes()
            );
            assert_eq!(f.num_entries, 10000);
            assert_eq!(f.num_deletions, 0);
        });

        // delete sst file in range (except L0)
        assert!(db
            .delete_file_in_range(
                format!("{:0>4}", 0).as_bytes(),
                format!("{:0>4}", 9999).as_bytes()
            )
            .is_ok());
        let livefiles = db.live_files().unwrap();
        assert_eq!(livefiles.len(), 0);

        // try to get a deleted key
        assert!(db.get(format!("{:0>4}", 123).as_bytes()).unwrap().is_none());
    }

    // raise error when db exists
    {
        opts.set_error_if_exists(true);
        assert!(DB::open(&opts, &path).is_err());
    }

    // disable all threads
    {
        // create new options
        let mut opts = Options::default();
        opts.set_max_background_jobs(0);
        opts.set_stats_dump_period_sec(0);
        opts.set_stats_persist_period_sec(0);

        // test Env::Default()->SetBackgroundThreads(0, Env::Priority::BOTTOM);
        let mut env = Env::default().unwrap();
        env.set_bottom_priority_background_threads(0);
        opts.set_env(&env);

        // open db
        let db = DB::open(&opts, &path).unwrap();

        // try to get key
        let iter = db.iterator(IteratorMode::Start);
        for (expected, (k, _)) in iter.enumerate() {
            assert_eq!(k.as_ref(), format!("{:0>4}", expected).as_bytes());
        }
    }
}

#[test]
fn test_open_for_read_only() {
    let path = DBPath::new("_rust_rocksdb_test_open_for_read_only");
    {
        let db = DB::open_default(&path).unwrap();
        db.put(b"k1", b"v1").unwrap();
    }
    {
        let opts = Options::default();
        let error_if_log_file_exist = false;
        let db = DB::open_for_read_only(&opts, &path, error_if_log_file_exist).unwrap();
        assert_eq!(db.get(b"k1").unwrap().unwrap(), b"v1");
        assert!(db.put(b"k2", b"v2").is_err());
    }
}

#[test]
fn test_open_cf_for_read_only() {
    let path = DBPath::new("_rust_rocksdb_test_open_cf_for_read_only");
    let cfs = vec!["cf1"];
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        let db = DB::open_cf(&opts, &path, cfs.clone()).unwrap();
        let cf1 = db.cf_handle("cf1").unwrap();
        db.put_cf(cf1, b"k1", b"v1").unwrap();
    }
    {
        let opts = Options::default();
        let error_if_log_file_exist = false;
        let db = DB::open_cf_for_read_only(&opts, &path, cfs, error_if_log_file_exist).unwrap();
        let cf1 = db.cf_handle("cf1").unwrap();
        assert_eq!(db.get_cf(cf1, b"k1").unwrap().unwrap(), b"v1");
        assert!(db.put_cf(cf1, b"k2", b"v2").is_err());
    }
}

#[test]
fn delete_range_test() {
    let path = DBPath::new("_rust_rocksdb_delete_range_test");
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cfs = vec!["cf1"];
        let db = DB::open_cf(&opts, &path, cfs).unwrap();

        let cf1 = db.cf_handle("cf1").unwrap();
        db.put_cf(cf1, b"k1", b"v1").unwrap();
        db.put_cf(cf1, b"k2", b"v2").unwrap();
        db.put_cf(cf1, b"k3", b"v3").unwrap();
        db.put_cf(cf1, b"k4", b"v4").unwrap();
        db.put_cf(cf1, b"k5", b"v5").unwrap();

        db.delete_range_cf(cf1, b"k2", b"k4").unwrap();
        assert_eq!(db.get_cf(cf1, b"k1").unwrap().unwrap(), b"v1");
        assert_eq!(db.get_cf(cf1, b"k4").unwrap().unwrap(), b"v4");
        assert_eq!(db.get_cf(cf1, b"k5").unwrap().unwrap(), b"v5");
        assert!(db.get_cf(cf1, b"k2").unwrap().is_none());
        assert!(db.get_cf(cf1, b"k3").unwrap().is_none());
    }
}

#[test]
fn multi_get() {
    let path = DBPath::new("_rust_rocksdb_multi_get");

    {
        let db = DB::open_default(&path).unwrap();
        db.put(b"k1", b"v1").unwrap();
        db.put(b"k2", b"v2").unwrap();

        let values = db
            .multi_get(&[b"k0", b"k1", b"k2"])
            .expect("multi_get failed");
        assert_eq!(3, values.len());
        assert!(values[0].is_empty());
        assert_eq!(values[1], b"v1");
        assert_eq!(values[2], b"v2");
    }
}

#[test]
fn multi_get_cf() {
    let path = DBPath::new("_rust_rocksdb_multi_get_cf");

    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        let db = DB::open_cf(&opts, &path, &["cf0", "cf1", "cf2"]).unwrap();

        let cf0 = db.cf_handle("cf0").unwrap();

        let cf1 = db.cf_handle("cf1").unwrap();
        db.put_cf(cf1, b"k1", b"v1").unwrap();

        let cf2 = db.cf_handle("cf2").unwrap();
        db.put_cf(cf2, b"k2", b"v2").unwrap();

        let values = db
            .multi_get_cf(vec![(cf0, b"k0"), (cf1, b"k1"), (cf2, b"k2")])
            .expect("multi_get failed");
        assert_eq!(3, values.len());
        assert!(values[0].is_empty());
        assert_eq!(values[1], b"v1");
        assert_eq!(values[2], b"v2");
    }
}

#[test]
fn key_may_exist() {
    let path = DBPath::new("_rust_rocksdb_multi_get");

    {
        let db = DB::open_default(&path).unwrap();
        assert_eq!(false, db.key_may_exist("nonexistent"));
        assert_eq!(
            false,
            db.key_may_exist_opt("nonexistent", &ReadOptions::default())
        );
    }
}

#[test]
fn key_may_exist_cf() {
    let path = DBPath::new("_rust_rocksdb_multi_get_cf");

    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        let db = DB::open_cf(&opts, &path, &["cf"]).unwrap();
        let cf = db.cf_handle("cf").unwrap();

        assert_eq!(false, db.key_may_exist_cf(cf, "nonexistent"));
        assert_eq!(
            false,
            db.key_may_exist_cf_opt(cf, "nonexistent", &ReadOptions::default())
        );
    }
}

#[test]
fn test_snapshot_outlive_db() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail/snapshot_outlive_db.rs");
}
