/*
   Copyright 2014 Tyler Neely

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/
#![feature(test)]

extern crate rocksdb;
extern crate test;
use rocksdb::{Options, RocksDB, MergeOperands, new_bloom_filter, Writable, DBIterator, SubDBIterator };
use rocksdb::RocksDBCompactionStyle::RocksDBUniversalCompaction;

fn snapshot_test() {
    let path = "_rust_rocksdb_iteratortest";
    {
        let mut db = RocksDB::open_default(path).unwrap();
        let p = db.put(b"k1", b"v1111");
        assert!(p.is_ok());
        let p = db.put(b"k2", b"v2222");
        assert!(p.is_ok());
        let p = db.put(b"k3", b"v3333");
        assert!(p.is_ok());
        let mut snap = db.snapshot();
        let mut view1 = snap.iterator();
        println!("See the output of the first iter");
        for (k,v) in view1.from_start() {
            println!("Hello {}: {}", std::str::from_utf8(k).unwrap(), std::str::from_utf8(v).unwrap());
        };
        for (k,v) in view1.from_start() {
            println!("Hello {}: {}", std::str::from_utf8(k).unwrap(), std::str::from_utf8(v).unwrap());
        };
        for (k,v) in view1.from_end() {
            println!("Hello {}: {}", std::str::from_utf8(k).unwrap(), std::str::from_utf8(v).unwrap());
        };
    }
    let opts = Options::new();
    assert!(RocksDB::destroy(&opts, path).is_ok());
}

fn iterator_test() {
    let path = "_rust_rocksdb_iteratortest";
    {
        let mut db = RocksDB::open_default(path).unwrap();
        let p = db.put(b"k1", b"v1111");
        assert!(p.is_ok());
        let p = db.put(b"k2", b"v2222");
        assert!(p.is_ok());
        let p = db.put(b"k3", b"v3333");
        assert!(p.is_ok());
        {
            let mut view1 = db.iterator();
            println!("See the output of the first iter");
            for (k,v) in view1.from_start() {
                println!("Hello {}: {}", std::str::from_utf8(k).unwrap(), std::str::from_utf8(v).unwrap());
            };
            for (k,v) in view1.from_start() {
                println!("Hello {}: {}", std::str::from_utf8(k).unwrap(), std::str::from_utf8(v).unwrap());
            };
            for (k,v) in view1.from_end() {
                println!("Hello {}: {}", std::str::from_utf8(k).unwrap(), std::str::from_utf8(v).unwrap());
            };
        }
        let mut view2 = db.iterator();
        let p = db.put(b"k4", b"v4444");
        assert!(p.is_ok());
        let mut view3 = db.iterator();
        println!("See the output of the second iter");
        for (k,v) in view2.from_start() {
            println!("Hello {}: {}", std::str::from_utf8(k).unwrap(), std::str::from_utf8(v).unwrap());
        }
        println!("See the output of the third iter");
        for (k,v) in view3.from_start() {
            println!("Hello {}: {}", std::str::from_utf8(k).unwrap(), std::str::from_utf8(v).unwrap());
        }
        println!("now the 3rd iter from k2 fwd");
        for (k,v) in view3.from(b"k2", rocksdb::Direction::forward) {
            println!("Hello {}: {}", std::str::from_utf8(k).unwrap(), std::str::from_utf8(v).unwrap());
        }
        println!("now the 3rd iter from k2 and back");
        for (k,v) in view3.from(b"k2", rocksdb::Direction::reverse) {
            println!("Hello {}: {}", std::str::from_utf8(k).unwrap(), std::str::from_utf8(v).unwrap());
        }
    }
    let opts = Options::new();
    assert!(RocksDB::destroy(&opts, path).is_ok());
}

#[cfg(not(feature = "valgrind"))]
fn main() {
    snapshot_test();
    iterator_test();
    let path = "/tmp/rust-rocksdb";
    let mut db = RocksDB::open_default(path).unwrap();
    assert!(db.put(b"my key", b"my value").is_ok());
    db.get(b"my key").map( |value| {
            match value.to_utf8() {
            Some(v) =>
            println!("retrieved utf8 value: {}", v),
            None =>
            println!("did not read valid utf-8 out of the db"),
            }
            })
    .on_absent( || { println!("value not found") })
        .on_error( |e| { println!("error retrieving value: {}", e) });

    assert!(db.delete(b"my key").is_ok());

    custom_merge();
}

fn concat_merge(new_key: &[u8], existing_val: Option<&[u8]>,
    mut operands: &mut MergeOperands) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::with_capacity(operands.size_hint().0);
    match existing_val {
        Some(v) => result.extend(v),
        None => (),
    }
    for op in operands {
        result.extend(op);
    }
    result
}

fn custom_merge() {
    let path = "_rust_rocksdb_mergetest";
    let mut opts = Options::new();
    opts.create_if_missing(true);
    opts.add_merge_operator("test operator", concat_merge);
    {
        let mut db = RocksDB::open(&opts, path).unwrap();
        db.put(b"k1", b"a");
        db.merge(b"k1", b"b");
        db.merge(b"k1", b"c");
        db.merge(b"k1", b"d");
        db.merge(b"k1", b"efg");
        db.merge(b"k1", b"h");
        db.get(b"k1").map( |value| {
                match value.to_utf8() {
                Some(v) =>
                println!("retrieved utf8 value: {}", v),
                None =>
                println!("did not read valid utf-8 out of the db"),
                }
                })
        .on_absent( || { println!("value not found") })
            .on_error( |e| { println!("error retrieving value: {}", e) });

    }
    RocksDB::destroy(&opts, path).is_ok();
}

#[cfg(feature = "valgrind")]
fn main() {
    let path = "_rust_rocksdb_valgrind";
    let mut opts = Options::new();
    opts.create_if_missing(true);
    opts.add_merge_operator("test operator", concat_merge);
    let db = RocksDB::open(&opts, path).unwrap();
    loop {
        db.put(b"k1", b"a");
        db.merge(b"k1", b"b");
        db.merge(b"k1", b"c");
        db.merge(b"k1", b"d");
        db.merge(b"k1", b"efg");
        db.merge(b"k1", b"h");
        db.get(b"k1").map( |value| {
            match value.to_utf8() {
                Some(v) => (),
                None => panic!("value corrupted"),
            }
        })
            .on_absent( || { panic!("value not found") })
            .on_error( |e| { panic!("error retrieving value: {}", e) });
        db.delete(b"k1");
    }
}


#[cfg(test)]
mod tests  {
    use test::Bencher;
    use std::thread::sleep_ms;

    use rocksdb::{BlockBasedOptions, Options, RocksDB, MergeOperands, new_bloom_filter, Writable };
    use rocksdb::RocksDBCompactionStyle::RocksDBUniversalCompaction;

    fn tuned_for_somebody_elses_disk(path: &str, opts: & mut Options, blockopts: &mut BlockBasedOptions) -> RocksDB {
        opts.create_if_missing(true);
        opts.set_max_open_files(10000);
        opts.set_use_fsync(false);
        opts.set_bytes_per_sync(8388608);
        opts.set_disable_data_sync(false);
        opts.set_block_cache_size_mb(1024);
        opts.set_table_cache_num_shard_bits(6);
        opts.set_max_write_buffer_number(32);
        opts.set_write_buffer_size(536870912);
        opts.set_target_file_size_base(1073741824);
        opts.set_min_write_buffer_number_to_merge(4);
        opts.set_level_zero_stop_writes_trigger(2000);
        opts.set_level_zero_slowdown_writes_trigger(0);
        opts.set_compaction_style(RocksDBUniversalCompaction);
        opts.set_max_background_compactions(4);
        opts.set_max_background_flushes(4);
        opts.set_filter_deletes(false);
        blockopts.set_block_size(524288);
        opts.set_block_based_table_factory(blockopts);
        opts.set_disable_auto_compactions(true);

        let filter = new_bloom_filter(10);
        //opts.set_filter(filter);

        RocksDB::open(&opts, path).unwrap()
    }

    #[bench]
    fn a_writes(b: &mut Bencher) {
        // dirty hack due to parallel tests causing contention.
        sleep_ms(1000);
        let path = "_rust_rocksdb_optimizetest";
        let mut opts = Options::new();
        let mut blockopts = BlockBasedOptions::new();
        let mut db = tuned_for_somebody_elses_disk(path, &mut opts, &mut blockopts);
        let mut i = 0 as u64;
        b.iter(|| {
            db.put(i.to_string().as_bytes(), b"v1111");
            i += 1;
        });
    }

    #[bench]
    fn b_reads(b: &mut Bencher) {
        let path = "_rust_rocksdb_optimizetest";
        let mut opts = Options::new();
        let mut blockopts = BlockBasedOptions::new();
        {
            let db = tuned_for_somebody_elses_disk(path, &mut opts, &mut blockopts);
            let mut i = 0 as u64;
            b.iter(|| {
                    db.get(i.to_string().as_bytes()).on_error( |e| {
                        println!("error: {}", e);
                        e
                        });
                    i += 1;
                    });
        }
        RocksDB::destroy(&opts, path).is_ok();
    }
}
