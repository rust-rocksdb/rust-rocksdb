# rust-rocksdb

This library has been tested against RocksDB 6.4 on Linux and macOS.

## Status
  - [x] basic open/put/get/delete/close
  - [x] rustic merge operator
  - [x] write batch (thanks @dgrnbrg!)
  - [x] save point
  - [x] compaction filter, style
  - [x] LRU cache
  - [x] destroy/repair
  - [x] iterator
  - [x] comparator
  - [x] snapshot
  - [x] column family operations
  - [x] prefix seek
  - [x] slicetransform
  - [x] rate limiter
  - [x] statistics
  - [x] recovery
  - [x] backup
  - [x] pause/continue background work
  - [x] delete range
  - [x] ingest external sst files
  - [ ] windows support

Feedback and pull requests welcome! If a particular feature of RocksDB is important to you, please let us know by opening an issue, and we will prioritize it.

## Build

```
$ git submodule update --init --recursive # if you just cloned the repository
$ cargo build
```

Bindings are pre-generated for x86_64 Linux. For other platforms, bindings are generated at compile time.

If the content in librocksdb_sys/crocksdb/crocksdb/c.h is updated, you may need to regenerate bindings:

```
$ ./scripts/generate-bindings.sh
```

## Running

###### Cargo.toml

```rust
[dependencies.rocksdb]
git = "https://github.com/tikv/rust-rocksdb.git"
```

###### Code

```rust
extern crate rocksdb;
use rocksdb::{DB, Writable};

fn main() {
    let mut db = DB::open_default("/path/for/rocksdb/storage").unwrap();
    db.put(b"my key", b"my value");
    match db.get(b"my key") {
        Ok(Some(value)) => println!("retrieved value {}", value.to_utf8().unwrap()),
        Ok(None) => println!("value not found"),
        Err(e) => println!("operational problem encountered: {}", e),
    }

    db.delete(b"my key");
}
```

###### Doing an atomic commit of several writes

```rust
extern crate rocksdb;
use rocksdb::{DB, WriteBatch, Writable};

fn main() {
    // NB: db is automatically freed at end of lifetime
    let mut db = DB::open_default("/path/for/rocksdb/storage").unwrap();
    {
        let mut batch = WriteBatch::new(); // WriteBatch and db both have trait Writable
        batch.put(b"my key", b"my value");
        batch.put(b"key2", b"value2");
        batch.put(b"key3", b"value3");
        db.write(batch); // Atomically commits the batch
    }
}
```

###### Getting an Iterator

```rust
extern crate rocksdb;
use rocksdb::{DB, Direction, IteratorMode};

fn main() {
    // NB: db is automatically freed at end of lifetime
    let mut db = DB::open_default("/path/for/rocksdb/storage").unwrap();
    let mut iter = db.iterator(IteratorMode::Start); // Always iterates forward
    for (key, value) in iter {
        println!("Saw {} {}", key, value); //actually, need to convert [u8] keys into Strings
    }
    iter = db.iterator(IteratorMode::End);  // Always iterates backward
    for (key, value) in iter {
        println!("Saw {} {}", key, value);
    }
    iter = db.iterator(IteratorMode::From(b"my key", Direction::forward)); // From a key in Direction::{forward,reverse}
    for (key, value) in iter {
        println!("Saw {} {}", key, value);
    }

    // You can seek with an existing Iterator instance, too
    iter.set_mode(IteratorMode::From(b"another key", Direction::reverse));
    for (key, value) in iter {
        println!("Saw {} {}", key, value);
    }
}
```

###### Getting an Iterator from a Snapshot

```rust
extern crate rocksdb;
use rocksdb::{DB, Direction};

fn main() {
    // NB: db is automatically freed at end of lifetime
    let mut db = DB::open_default("/path/for/rocksdb/storage").unwrap();
    let snapshot = db.snapshot(); // Creates a longer-term snapshot of the DB, but freed when goes out of scope
    let mut iter = snapshot.iterator(IteratorMode::Start); // Make as many iterators as you'd like from one snapshot
}
```

###### Rustic Merge Operator

```rust
extern crate rocksdb;
use rocksdb::{Options, DB, MergeOperands, Writable};

fn concat_merge(new_key: &[u8], existing_val: Option<&[u8]>,
    operands: &mut MergeOperands) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::with_capacity(operands.size_hint().0);
    existing_val.map(|v| {
        for e in v {
            result.push(*e)
        }
    });
    for op in operands {
        for e in op {
            result.push(*e)
        }
    }
    result
}

fn main() {
    let path = "/path/to/rocksdb";
    let mut opts = Options::new();
    opts.create_if_missing(true);
    opts.add_merge_operator("test operator", concat_merge);
    let mut db = DB::open(&opts, path).unwrap();
    let p = db.put(b"k1", b"a");
    db.merge(b"k1", b"b");
    db.merge(b"k1", b"c");
    db.merge(b"k1", b"d");
    db.merge(b"k1", b"efg");
    let r = db.get(b"k1");
    assert!(r.unwrap().unwrap().to_utf8().unwrap() == "abcdefg");
}
```

###### Apply Some Tunings

Please read [the official tuning guide](https://github.com/facebook/rocksdb/wiki/RocksDB-Tuning-Guide), and most importantly, measure performance under realistic workloads with realistic hardware.

```rust
use rocksdb::{Options, DB};
use rocksdb::DBCompactionStyle::DBUniversalCompaction;

fn badly_tuned_for_somebody_elses_disk() -> DB {
    let path = "_rust_rocksdb_optimizetest";
    let mut opts = Options::new();
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
    opts.set_compaction_style(DBUniversalCompaction);
    opts.set_max_background_compactions(4);
    opts.set_max_background_flushes(4);
    opts.set_disable_auto_compactions(true);

    DB::open(&opts, path).unwrap()
}
```
