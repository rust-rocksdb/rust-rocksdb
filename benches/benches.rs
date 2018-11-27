// Copyright 2016 PingCAP, Inc.
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
#![feature(test)]
extern crate rocksdb;
extern crate tempdir;
extern crate test;
use rocksdb::{set_perf_level, DBOptions, PerfContext, PerfLevel, Writable, DB};
use tempdir::TempDir;
use test::Bencher;

#[bench]
fn bench_perf_context(b: &mut Bencher) {
    set_perf_level(PerfLevel::EnableTimeExceptForMutex);
    let mut ctx = PerfContext::get();
    ctx.reset();
    b.iter(|| {
        assert_eq!(ctx.write_wal_time(), 0);
        assert_eq!(ctx.write_memtable_time(), 0);
        assert_eq!(ctx.write_delay_time(), 0);
    })
}

#[bench]
fn bench_kv_write_with_perf(b: &mut Bencher) {
    let temp_dir = TempDir::new("bench_kv_write_with_perf").unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let db = DB::open(opts, temp_dir.path().to_str().unwrap()).unwrap();
    set_perf_level(PerfLevel::EnableTimeExceptForMutex);

    b.iter(|| {
        for i in 0..10000 {
            let k = &[i as u8];
            db.put(k, k).unwrap();
            if i % 5 == 0 {
                db.delete(k).unwrap();
            }
        }
    })
}

#[bench]
fn bench_kv_write_without_perf(b: &mut Bencher) {
    let temp_dir = TempDir::new("bench_kv_write_with_perf").unwrap();
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let db = DB::open(opts, temp_dir.path().to_str().unwrap()).unwrap();
    set_perf_level(PerfLevel::Uninitialized);

    b.iter(|| {
        for i in 0..10000 {
            let k = &[i as u8];
            db.put(k, k).unwrap();
            if i % 5 == 0 {
                db.delete(k).unwrap();
            }
        }
    })
}
