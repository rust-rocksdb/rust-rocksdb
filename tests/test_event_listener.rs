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


use std::sync::Arc;
use std::sync::atomic::*;

use rocksdb::*;
use tempdir::TempDir;

use test_ingest_external_file::gen_sst;

#[derive(Default, Clone)]
struct EventCounter {
    flush: Arc<AtomicUsize>,
    compaction: Arc<AtomicUsize>,
    ingestion: Arc<AtomicUsize>,
    drop_count: Arc<AtomicUsize>,
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

        self.compaction.fetch_add(1, Ordering::SeqCst);
    }

    fn on_external_file_ingested(&self, info: &IngestionInfo) {
        assert!(!info.cf_name().is_empty());
        assert!(info.internal_file_path().exists());
        assert_ne!(info.table_properties().data_size(), 0);
        self.ingestion.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn test_event_listener_basic() {
    let path = TempDir::new("_rust_rocksdb_event_listener_flush").expect("");
    let path_str = path.path().to_str().unwrap();

    let mut opts = Options::new();
    let counter = EventCounter::default();
    opts.add_event_listener(counter.clone());
    opts.create_if_missing(true);
    let db = DB::open(opts, path_str).unwrap();
    for i in 1..8000 {
        db.put(format!("{:04}", i).as_bytes(),
                    format!("{:04}", i).as_bytes())
            .unwrap();
    }
    db.flush(true).unwrap();
    assert_ne!(counter.flush.load(Ordering::SeqCst), 0);

    for i in 1..8000 {
        db.put(format!("{:04}", i).as_bytes(),
                    format!("{:04}", i).as_bytes())
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
}

#[test]
fn test_event_listener_ingestion() {
    let path = TempDir::new("_rust_rocksdb_event_listener_ingestion").expect("");
    let path_str = path.path().to_str().unwrap();

    let mut opts = Options::new();
    let counter = EventCounter::default();
    opts.add_event_listener(counter.clone());
    opts.create_if_missing(true);
    let db = DB::open(opts, path_str).unwrap();

    let gen_path = TempDir::new("_rust_rocksdb_ingest_sst_gen").expect("");
    let test_sstfile = gen_path.path().join("test_sst_file");
    let test_sstfile_str = test_sstfile.to_str().unwrap();

    let default_options = db.get_options();
    gen_sst(default_options,
            Some(db.cf_handle("default").unwrap()),
            test_sstfile_str,
            &[(b"k1", b"v1"), (b"k2", b"v2")]);

    let ingest_opt = IngestExternalFileOptions::new();
    db.ingest_external_file(&ingest_opt, &[test_sstfile_str])
        .unwrap();
    assert!(test_sstfile.exists());
    assert_eq!(db.get(b"k1").unwrap().unwrap(), b"v1");
    assert_eq!(db.get(b"k2").unwrap().unwrap(), b"v2");
    assert_ne!(counter.ingestion.load(Ordering::SeqCst), 0);
}
