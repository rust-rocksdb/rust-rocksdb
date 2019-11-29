// Copyright 2019 PingCAP, Inc.
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

use super::rocksdb::{ColumnFamilyOptions, DBOptions, WriteOptions, DB};
use super::test::Bencher;

fn run_bench_wal(b: &mut Bencher, name: &str, mut opts: DBOptions, wopts: WriteOptions) {
    let path = tempfile::Builder::new().prefix(name).tempdir().expect("");
    let path_str = path.path().to_str().unwrap();
    opts.create_if_missing(true);
    opts.set_max_background_jobs(6);
    opts.set_max_subcompactions(2);

    let mut cf_opts = ColumnFamilyOptions::new();
    cf_opts.set_write_buffer_size(16 * 1024);
    cf_opts.set_max_write_buffer_number(10);

    let db = DB::open_cf(opts, path_str, vec![("default", cf_opts)]).unwrap();

    let value = vec![1; 1024];

    let mut i = 0;
    b.iter(|| {
        let key = format!("key_{}", i);
        db.put_opt(key.as_bytes(), &value, &wopts).unwrap();
        i += 1;
    });

    drop(db);
}

fn run_bench_wal_recycle_log(b: &mut Bencher, name: &str, recycled: bool) {
    let mut opts = DBOptions::new();
    if recycled {
        opts.set_recycle_log_file_num(10);
    }

    let mut wopts = WriteOptions::new();
    wopts.set_sync(true);

    run_bench_wal(b, name, opts, wopts);
}

#[bench]
fn bench_wal_with_recycle_log(b: &mut Bencher) {
    run_bench_wal_recycle_log(b, "_rust_rocksdb_wal_with_recycle_log", true);
}

#[bench]
fn bench_wal_without_recycle_log(b: &mut Bencher) {
    run_bench_wal_recycle_log(b, "_rust_rocksdb_wal_without_recycle_log", false);
}

#[bench]
fn bench_wal_no_sync(b: &mut Bencher) {
    let opts = DBOptions::new();
    let mut wopts = WriteOptions::new();
    wopts.set_sync(false);

    run_bench_wal(b, "_rust_rocksdb_wal_no_sync", opts, wopts);
}

#[bench]
fn bench_wal_disalbe_wal(b: &mut Bencher) {
    let opts = DBOptions::new();
    let mut wopts = WriteOptions::new();
    wopts.disable_wal(true);

    run_bench_wal(b, "_rust_rocksdb_wal_disable_wal", opts, wopts);
}
