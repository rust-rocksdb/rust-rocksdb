// Copyright 2014 Tyler Neely
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
//

extern crate rocksdb;
use rocksdb::{ColumnFamilyOptions, DBOptions, MergeOperands, Writable, DB};

// fn snapshot_test() {
//    let path = "_rust_rocksdb_iteratortest";
//    {
//        let mut db = DB::open_default(path).unwrap();
//        let p = db.put(b"k1", b"v1111");
//        assert!(p.is_ok());
//        let p = db.put(b"k2", b"v2222");
//        assert!(p.is_ok());
//        let p = db.put(b"k3", b"v3333");
//        assert!(p.is_ok());
//        let mut snap = db.snapshot();
//        let mut view1 = snap.iterator();
//        println!("See the output of the first iter");
//        for (k,v) in view1.from_start() {
// println!("Hello {}: {}", std::str::from_utf8(k).unwrap(),
// std::str::from_utf8(v).unwrap());
//        };
//        for (k,v) in view1.from_start() {
// println!("Hello {}: {}", std::str::from_utf8(k).unwrap(),
// std::str::from_utf8(v).unwrap());
//        };
//        for (k,v) in view1.from_end() {
// println!("Hello {}: {}", std::str::from_utf8(k).unwrap(),
// std::str::from_utf8(v).unwrap());
//        };
//    }
//    let opts = Options::new();
//    assert!(DB::destroy(&opts, path).is_ok());
// }

#[cfg(not(feature = "valgrind"))]
fn main() {
    let path = "/tmp/rust-rocksdb";
    let db = DB::open_default(path).unwrap();
    assert!(db.put(b"my key", b"my value").is_ok());
    match db.get(b"my key") {
        Ok(Some(value)) => match value.to_utf8() {
            Some(v) => println!("retrieved utf8 value: {}", v),
            None => println!("did not read valid utf-8 out of the db"),
        },
        Ok(None) => panic!("value not present!"),
        Err(e) => println!("error retrieving value: {}", e),
    }

    assert!(db.delete(b"my key").is_ok());

    custom_merge();
}

fn concat_merge(_: &[u8], existing_val: Option<&[u8]>, operands: &mut MergeOperands) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::with_capacity(operands.size_hint().0);
    match existing_val {
        Some(v) => {
            for e in v {
                result.push(*e)
            }
        }
        None => (),
    }
    for op in operands {
        for e in op {
            result.push(*e);
        }
    }
    result
}

fn custom_merge() {
    let path = "_rust_rocksdb_mergetest";
    let mut opts = DBOptions::new();
    opts.create_if_missing(true);
    let mut cf_opts = ColumnFamilyOptions::new();
    cf_opts.add_merge_operator("test operator", concat_merge);
    {
        let db = DB::open_cf(opts, path, vec![("default", cf_opts)]).unwrap();
        db.put(b"k1", b"a").unwrap();
        db.merge(b"k1", b"b").unwrap();
        db.merge(b"k1", b"c").unwrap();
        db.merge(b"k1", b"d").unwrap();
        db.merge(b"k1", b"efg").unwrap();
        db.merge(b"k1", b"h").unwrap();
        match db.get(b"k1") {
            Ok(Some(value)) => match value.to_utf8() {
                Some(v) => println!("retrieved utf8 value: {}", v),
                None => println!("did not read valid utf-8 out of the db"),
            },
            Ok(None) => panic!("value not present!"),
            Err(e) => println!("error retrieving value: {}", e),
        }
    }
    let opts = DBOptions::new();
    let _ = DB::destroy(&opts, path).is_ok();
}

#[cfg(test)]
mod tests {
    use rocksdb::DBCompactionStyle;
    use rocksdb::DBRecoveryMode;
    use rocksdb::{BlockBasedOptions, ColumnFamilyOptions, DBCompressionType, DBOptions, DB};

    #[allow(dead_code)]
    fn tuned_for_somebody_elses_disk(
        path: &str,
        mut opts: DBOptions,
        blockopts: &mut BlockBasedOptions,
    ) -> DB {
        let per_level_compression: [DBCompressionType; 7] = [
            DBCompressionType::No,
            DBCompressionType::No,
            DBCompressionType::No,
            DBCompressionType::Lz4,
            DBCompressionType::Lz4,
            DBCompressionType::Lz4,
            DBCompressionType::Lz4,
        ];
        let mut cf_opts = ColumnFamilyOptions::new();
        opts.create_if_missing(true);
        opts.set_max_open_files(10000);
        opts.set_use_fsync(false);
        opts.set_bytes_per_sync(8388608);
        cf_opts.set_block_cache_size_mb(1024);
        opts.set_table_cache_num_shard_bits(6);
        cf_opts.set_max_write_buffer_number(32);
        cf_opts.set_write_buffer_size(536870912);
        cf_opts.set_target_file_size_base(1073741824);
        cf_opts.set_min_write_buffer_number_to_merge(4);
        cf_opts.set_level_zero_file_num_compaction_trigger(4);
        cf_opts.set_level_zero_stop_writes_trigger(2000);
        cf_opts.set_level_zero_slowdown_writes_trigger(0);
        cf_opts.set_compaction_style(DBCompactionStyle::Universal);
        opts.set_max_background_jobs(4);
        cf_opts.set_report_bg_io_stats(true);
        opts.set_wal_recovery_mode(DBRecoveryMode::PointInTime);
        opts.enable_statistics(true);
        opts.set_stats_dump_period_sec(60);
        cf_opts.compression_per_level(&per_level_compression);
        blockopts.set_block_size(524288);
        blockopts.set_no_block_cache(true);
        blockopts.set_cache_index_and_filter_blocks(true);
        blockopts.set_bloom_filter(10, false);
        cf_opts.set_block_based_table_factory(blockopts);
        cf_opts.set_disable_auto_compactions(true);
        cf_opts.set_max_compaction_bytes(1073741824 * 25);

        // let filter = new_bloom_filter(10);
        // opts.set_filter(filter);

        DB::open_cf(opts, path, vec![("default", cf_opts)]).unwrap()
    }

    // TODO(tyler) unstable
    // #[bench]
    // fn a_writes(b: &mut Bencher) {
    // dirty hack due to parallel tests causing contention.
    // sleep_ms(1000);
    // let path = "_rust_rocksdb_optimizetest";
    // let mut opts = Options::new();
    // let mut blockopts = BlockBasedOptions::new();
    // let mut db = tuned_for_somebody_elses_disk(path, &mut opts, &mut blockopts);
    // let mut i = 0 as u64;
    // b.iter(|| {
    // db.put(i.to_string().as_bytes(), b"v1111");
    // i += 1;
    // });
    // }
    //
    // #[bench]
    // fn b_reads(b: &mut Bencher) {
    // let path = "_rust_rocksdb_optimizetest";
    // let mut opts = Options::new();
    // let mut blockopts = BlockBasedOptions::new();
    // {
    // let db = tuned_for_somebody_elses_disk(path, &mut opts, &mut blockopts);
    // let mut i = 0 as u64;
    // b.iter(|| {
    // db.get(i.to_string().as_bytes()).on_error( |e| {
    // println!("error: {}", e);
    // e
    // });
    // i += 1;
    // });
    // }
    // DB::destroy(&opts, path).is_ok();
    // }
    //
}
