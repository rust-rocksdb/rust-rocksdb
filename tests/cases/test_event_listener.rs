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

use std::sync::atomic::*;
use std::sync::Arc;

use rocksdb::*;

use super::tempdir_with_prefix;
use super::test_ingest_external_file::gen_sst;

#[derive(Default, Clone)]
struct EventCounter {
    flush: Arc<AtomicUsize>,
    compaction: Arc<AtomicUsize>,
    ingestion: Arc<AtomicUsize>,
    drop_count: Arc<AtomicUsize>,
    input_records: Arc<AtomicUsize>,
    output_records: Arc<AtomicUsize>,
    input_bytes: Arc<AtomicUsize>,
    output_bytes: Arc<AtomicUsize>,
    manual_compaction: Arc<AtomicUsize>,
}

impl Drop for EventCounter {
    fn drop(&mut self) {
        self.drop_count.fetch_add(1, Ordering::SeqCst);
    }
}

impl EventListener for EventCounter {
    fn on_flush_completed(&self, info: &FlushJobInfo) {
        assert!(!info.cf_name().is_empty());
        assert!(info.file_path().exists());
        assert_ne!(info.table_properties().data_size(), 0);
        self.flush.fetch_add(1, Ordering::SeqCst);
    }

    fn on_compaction_completed(&self, info: &CompactionJobInfo) {
        info.status().unwrap();
        assert!(!info.cf_name().is_empty());
        let input_file_count = info.input_file_count();
        assert_ne!(input_file_count, 0);
        for i in 0..input_file_count {
            let path = info.input_file_at(i);
            assert!(path.exists());
        }

        let output_file_count = info.output_file_count();
        assert_ne!(output_file_count, 0);
        for i in 0..output_file_count {
            let path = info.output_file_at(i);
            assert!(path.exists());
        }

        let props = info.table_properties();
        assert_eq!(props.len(), output_file_count + input_file_count);

        assert_ne!(info.elapsed_micros(), 0);
        assert_eq!(info.num_corrupt_keys(), 0);
        assert!(info.output_level() >= 0);

        self.compaction.fetch_add(1, Ordering::SeqCst);
        self.input_records
            .fetch_add(info.input_records() as usize, Ordering::SeqCst);
        self.output_records
            .fetch_add(info.output_records() as usize, Ordering::SeqCst);
        self.input_bytes
            .fetch_add(info.total_input_bytes() as usize, Ordering::SeqCst);
        self.output_bytes
            .fetch_add(info.total_output_bytes() as usize, Ordering::SeqCst);

        if info.compaction_reason() == CompactionReason::ManualCompaction {
            self.manual_compaction.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn on_external_file_ingested(&self, info: &IngestionInfo) {
        assert!(!info.cf_name().is_empty());
        assert!(info.internal_file_path().exists());
        assert_ne!(info.table_properties().data_size(), 0);
        self.ingestion.fetch_add(1, Ordering::SeqCst);
    }
}

#[derive(Default, Clone)]
struct StallEventCounter {
    flush: Arc<AtomicUsize>,
    stall_conditions_changed_num: Arc<AtomicUsize>,
    triggered_writes_slowdown: Arc<AtomicUsize>,
    triggered_writes_stop: Arc<AtomicUsize>,
    stall_change_from_normal_to_other: Arc<AtomicUsize>,
}

impl EventListener for StallEventCounter {
    fn on_flush_completed(&self, info: &FlushJobInfo) {
        assert!(!info.cf_name().is_empty());
        self.flush.fetch_add(1, Ordering::SeqCst);
        self.triggered_writes_slowdown
            .fetch_add(info.triggered_writes_slowdown() as usize, Ordering::SeqCst);
        self.triggered_writes_stop
            .fetch_add(info.triggered_writes_stop() as usize, Ordering::SeqCst);
    }

    fn on_stall_conditions_changed(&self, info: &WriteStallInfo) {
        assert!(info.cf_name() == "test_cf");
        self.stall_conditions_changed_num
            .fetch_add(1, Ordering::SeqCst);
        if info.prev() == WriteStallCondition::Normal && info.cur() != WriteStallCondition::Normal {
            self.stall_change_from_normal_to_other
                .fetch_add(1, Ordering::SeqCst);
        }
    }
}

#[derive(Default, Clone)]
struct BackgroundErrorCounter {
    background_error: Arc<AtomicUsize>,
}

impl EventListener for BackgroundErrorCounter {
    fn on_background_error(&self, _: DBBackgroundErrorReason, _: Result<(), String>) {
        self.background_error.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn test_event_listener_stall_conditions_changed() {
    let path = tempdir_with_prefix("_rust_rocksdb_event_listener_stall_conditions");
    let path_str = path.path().to_str().unwrap();

    let mut opts = DBOptions::new();
    let counter = StallEventCounter::default();
    opts.add_event_listener(counter.clone());
    opts.create_if_missing(true);
    let mut cf_opts = ColumnFamilyOptions::new();
    cf_opts.set_level_zero_slowdown_writes_trigger(1);
    cf_opts.set_level_zero_stop_writes_trigger(1);
    cf_opts.set_level_zero_file_num_compaction_trigger(1);
    let mut db = DB::open_cf(
        opts,
        path_str,
        vec![("default", ColumnFamilyOptions::new())],
    )
    .unwrap();
    db.create_cf(("test_cf", cf_opts)).unwrap();

    let test_cf = db.cf_handle("test_cf").unwrap();
    for i in 1..5 {
        db.put_cf(
            test_cf,
            format!("{:04}", i).as_bytes(),
            format!("{:04}", i).as_bytes(),
        )
        .unwrap();
        db.flush_cf(test_cf, true).unwrap();
    }
    let flush_cnt = counter.flush.load(Ordering::SeqCst);
    assert_ne!(flush_cnt, 0);
    let stall_conditions_changed_num = counter.stall_conditions_changed_num.load(Ordering::SeqCst);
    let triggered_writes_slowdown = counter.triggered_writes_slowdown.load(Ordering::SeqCst);
    let triggered_writes_stop = counter.triggered_writes_stop.load(Ordering::SeqCst);
    let stall_change_from_normal_to_other = counter
        .stall_change_from_normal_to_other
        .load(Ordering::SeqCst);
    assert_ne!(stall_conditions_changed_num, 0);
    assert_ne!(triggered_writes_slowdown, 0);
    assert_ne!(triggered_writes_stop, 0);
    assert_ne!(stall_change_from_normal_to_other, 0);
}

#[test]
fn test_event_listener_basic() {
    let path = tempdir_with_prefix("_rust_rocksdb_event_listener_flush");
    let path_str = path.path().to_str().unwrap();

    let mut opts = DBOptions::new();
    let counter = EventCounter::default();
    opts.add_event_listener(counter.clone());
    opts.create_if_missing(true);
    let db = DB::open(opts, path_str).unwrap();
    for i in 1..8000 {
        db.put(
            format!("{:04}", i).as_bytes(),
            format!("{:04}", i).as_bytes(),
        )
        .unwrap();
    }
    db.flush(true).unwrap();
    assert_ne!(counter.flush.load(Ordering::SeqCst), 0);

    for i in 1..8000 {
        db.put(
            format!("{:04}", i).as_bytes(),
            format!("{:04}", i).as_bytes(),
        )
        .unwrap();
    }
    db.flush(true).unwrap();
    let flush_cnt = counter.flush.load(Ordering::SeqCst);
    assert_ne!(flush_cnt, 0);
    assert_eq!(counter.compaction.load(Ordering::SeqCst), 0);
    db.compact_range(None, None);
    assert_eq!(counter.flush.load(Ordering::SeqCst), flush_cnt);
    assert_ne!(counter.compaction.load(Ordering::SeqCst), 0);
    drop(db);
    assert_eq!(counter.drop_count.load(Ordering::SeqCst), 1);
    assert!(
        counter.input_records.load(Ordering::SeqCst)
            > counter.output_records.load(Ordering::SeqCst)
    );
    assert!(
        counter.input_bytes.load(Ordering::SeqCst) > counter.output_bytes.load(Ordering::SeqCst)
    );
    assert_eq!(counter.manual_compaction.load(Ordering::SeqCst), 1);
}

#[test]
fn test_event_listener_ingestion() {
    let path = tempdir_with_prefix("_rust_rocksdb_event_listener_ingestion");
    let path_str = path.path().to_str().unwrap();

    let mut opts = DBOptions::new();
    let counter = EventCounter::default();
    opts.add_event_listener(counter.clone());
    opts.create_if_missing(true);
    let db = DB::open(opts, path_str).unwrap();

    let gen_path = tempdir_with_prefix("_rust_rocksdb_ingest_sst_gen");
    let test_sstfile = gen_path.path().join("test_sst_file");
    let test_sstfile_str = test_sstfile.to_str().unwrap();

    let default_options = db.get_options();
    gen_sst(
        default_options,
        Some(db.cf_handle("default").unwrap()),
        test_sstfile_str,
        &[(b"k1", b"v1"), (b"k2", b"v2")],
    );

    let ingest_opt = IngestExternalFileOptions::new();
    db.ingest_external_file(&ingest_opt, &[test_sstfile_str])
        .unwrap();
    assert!(test_sstfile.exists());
    assert_eq!(db.get(b"k1").unwrap().unwrap(), b"v1");
    assert_eq!(db.get(b"k2").unwrap().unwrap(), b"v2");
    assert_ne!(counter.ingestion.load(Ordering::SeqCst), 0);
}

#[test]
fn test_event_listener_background_error() {
    // TODO(yiwu): should create a test Env object which inject some IO error, to
    // actually trigger background error.
    let path = tempdir_with_prefix("_rust_rocksdb_event_listener_ingestion");
    let path_str = path.path().to_str().unwrap();

    let mut opts = DBOptions::new();
    let counter = BackgroundErrorCounter::default();
    opts.add_event_listener(counter.clone());
    opts.create_if_missing(true);
    let db = DB::open(opts, path_str).unwrap();

    for i in 1..10 {
        db.put(format!("{:04}", i).as_bytes(), b"value").unwrap();
        db.flush(false).unwrap();
    }
    assert_eq!(counter.background_error.load(Ordering::SeqCst), 0);
}
