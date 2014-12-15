extern crate rocksdb;
extern crate test;
use rocksdb::{RocksDBOptions, RocksDB, MergeOperands, new_bloom_filter};
use rocksdb::RocksDBCompactionStyle::RocksDBUniversalCompaction;
use test::Bencher;

#[allow(dead_code)]
fn main() {
    let path = "/tmp/rust-rocksdb";
    let db = RocksDB::open_default(path).unwrap();
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
    db.close();

    custom_merge();
}

#[allow(dead_code)]
fn concat_merge(new_key: &[u8], existing_val: Option<&[u8]>,
    mut operands: &mut MergeOperands) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::with_capacity(operands.size_hint().val0());
    match existing_val {
        Some(v) => result.push_all(v),
        None => (),
    }
    for op in operands {
        result.push_all(op);
    }
    result
}

#[allow(dead_code)]
fn custom_merge() {
    let path = "_rust_rocksdb_mergetest";
    let opts = RocksDBOptions::new();
    opts.create_if_missing(true);
    opts.add_merge_operator("test operator", concat_merge);
    let db = RocksDB::open(opts, path).unwrap();
    let p = db.put(b"k1", b"a");
    db.merge(b"k1", b"b");
    db.merge(b"k1", b"c");
    db.merge(b"k1", b"d");
    db.merge(b"k1", b"efg");
    let m = db.merge(b"k1", b"h");
    let r = db.get(b"k1");
    assert!(r.unwrap().to_utf8().unwrap() == "abcdefgh");
    db.close();
    RocksDB::destroy(opts, path).is_ok();
}

#[allow(dead_code)]
fn tuned_for_somebody_elses_disk() -> RocksDB {
    let path = "_rust_rocksdb_optimizetest";
    let opts = RocksDBOptions::new();
    opts.create_if_missing(true);
    opts.set_block_size(524288);
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
    opts.set_disable_auto_compactions(true);

    let filter = new_bloom_filter(10);
    opts.set_filter(filter);

    RocksDB::open(opts, path).unwrap()
}

#[allow(dead_code)]
#[bench]
fn writes(b: &mut Bencher) {
    let db = tuned_for_somebody_elses_disk();
    let mut i = 0 as u64;
    b.iter(|| {
        db.put(i.to_string().as_bytes(), b"v1111");
        i += 1;
    });
    db.close();
}

#[allow(dead_code)]
#[bench]
fn reads(b: &mut Bencher) {
    let db = tuned_for_somebody_elses_disk();
    let mut i = 0 as u64;
    b.iter(|| {
        db.get(i.to_string().as_bytes()).on_error( |e| {
            println!("error: {}", e);
            e
        });
        i += 1;
    });
    db.close();
}
