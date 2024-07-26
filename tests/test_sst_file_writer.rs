// Copyright 2020 Lucjan Suski
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

use rocksdb::{Error, Options, ReadOptions, SstFileWriter, DB};
use util::{DBPath, U64Comparator, U64Timestamp};

#[test]
fn sst_file_writer_works() {
    let db_path = DBPath::new("_rust_rocksdb_sstfilewritertest");
    let dir = tempfile::Builder::new()
        .prefix("_rust_rocksdb_sstfilewritertest")
        .tempdir()
        .expect("Failed to create temporary path for file writer.");
    let writer_path = dir.path().join("filewriter");
    {
        let opts = Options::default();
        let mut writer = SstFileWriter::create(&opts);
        writer.open(&writer_path).unwrap();
        writer.put(b"k1", b"v1").unwrap();

        writer.put(b"k2", b"v2").unwrap();

        writer.delete(b"k3").unwrap();
        writer.finish().unwrap();
        assert!(writer.file_size() > 0);
    }
    {
        let db = DB::open_default(&db_path).unwrap();
        db.put(b"k3", b"v3").unwrap();
        db.ingest_external_file(vec![&writer_path]).unwrap();
        let r: Result<Option<Vec<u8>>, Error> = db.get(b"k1");
        assert_eq!(r.unwrap().unwrap(), b"v1");
        let r: Result<Option<Vec<u8>>, Error> = db.get(b"k2");
        assert_eq!(r.unwrap().unwrap(), b"v2");
        assert!(db.get(b"k3").unwrap().is_none());
    }
}

#[test]
fn sst_file_writer_with_ts_works() {
    let db_path = DBPath::new("_rust_rocksdb_sstfilewritertest_with_ts");
    let dir = tempfile::Builder::new()
        .prefix("_rust_rocksdb_sstfilewritertest_with_ts")
        .tempdir()
        .expect("Failed to create temporary path for file writer.");
    let writer_path = dir.path().join("filewriter");

    let ts = U64Timestamp::new(1);
    let ts2 = U64Timestamp::new(2);
    let ts3 = U64Timestamp::new(3);
    {
        let mut opts = Options::default();
        opts.set_comparator_with_ts(
            U64Comparator::NAME,
            U64Timestamp::SIZE,
            Box::new(U64Comparator::compare),
            Box::new(U64Comparator::compare_ts),
            Box::new(U64Comparator::compare_without_ts),
        );

        let mut writer = SstFileWriter::create(&opts);
        writer.open(&writer_path).unwrap();
        writer.put_with_ts(b"k1", ts, b"v1").unwrap();
        writer.put_with_ts(b"k2", ts2, b"v2").unwrap();
        writer.put_with_ts(b"k3", ts2, b"v3").unwrap();
        writer.finish().unwrap();

        assert!(writer.file_size() > 0);
    }

    {
        let _ = DB::destroy(&Options::default(), &db_path);

        let mut db_opts = Options::default();
        db_opts.create_missing_column_families(true);
        db_opts.create_if_missing(true);
        db_opts.set_comparator_with_ts(
            U64Comparator::NAME,
            U64Timestamp::SIZE,
            Box::new(U64Comparator::compare),
            Box::new(U64Comparator::compare_ts),
            Box::new(U64Comparator::compare_without_ts),
        );

        let db = DB::open(&db_opts, &db_path).unwrap();
        db.ingest_external_file(vec![&writer_path]).unwrap();
        db.delete_with_ts(b"k3", ts3).unwrap();

        let mut opts = ReadOptions::default();
        opts.set_timestamp(ts);

        let r: Result<Option<Vec<u8>>, Error> = db.get_opt(b"k1", &opts);
        assert_eq!(r.unwrap().unwrap(), b"v1");

        // at ts1 k2 should be invisible
        assert!(db.get_opt(b"k2", &opts).unwrap().is_none());

        // at ts2 k2 and k3 should be visible
        opts.set_timestamp(ts2);
        let r: Result<Option<Vec<u8>>, Error> = db.get_opt(b"k2", &opts);
        assert_eq!(r.unwrap().unwrap(), b"v2");
        let r = db.get_opt(b"k3", &opts);
        assert_eq!(r.unwrap().unwrap(), b"v3");

        // at ts3 the k3 should be deleted
        opts.set_timestamp(ts3);
        assert!(db.get_opt(b"k3", &opts).unwrap().is_none());
    }
}
