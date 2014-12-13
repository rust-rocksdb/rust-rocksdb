rust-rocksdb
============

### running
Install RocksDB.  rust-rocksdb has been tested with version 3.8.1 on linux and OSX.
###### Cargo.toml
```rust
[dependencies]
rocksdb = "~0.0.1"
```
###### Code
```rust
extern crate rocksdb;
use rocksdb::RocksDB;

fn main() {
    let db = RocksDB::open_default("/path/for/rocksdb/storage").unwrap;
    db.put(b"my key", b"my value");
    db.get(b"my key").map( |value| {
        match value.to_utf8() {
            Some(v) =>
                println!("retrieved utf8 value {}", v),
            None =>
                println!("did not read valid utf-8 out of the db"),
        }
    })
        .on_absent( || { println!("value not found") })
        .on_error( |e| { println!("error retrieving value: {}", e) });

    db.delete(b"my key");
    db.close();
}
```

###### Rustic Merge Operator
```rust
extern crate rocksdb;
use rocksdb::{RocksDBOptions, RocksDB, MergeOperands};

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

fn main() {
    let path = "/path/to/rocksdb";
    let opts = RocksDBOptions::new();
    opts.create_if_missing(true);
    opts.add_merge_operator("test operator", concat_merge);
    let db = RocksDB::open(opts, path).unwrap();
    let p = db.put(b"k1", b"a");
    db.merge(b"k1", b"b");
    db.merge(b"k1", b"c");
    db.merge(b"k1", b"d");
    db.merge(b"k1", b"efg");
    let r = db.get(b"k1");
    assert!(r.unwrap().to_utf8().unwrap() == "abcdefg");
    db.close();
}
```


### status
  - [x] basic open/put/get/delete/close
  - [x] linux support
  - [x] rocksdb compiled via cargo
  - [x] OSX support
  - [x] rustic merge operator
  - [ ] batch
  - [ ] iterator
  - [ ] range
  - [ ] create/release snapshot
  - [ ] column family operations
  - [ ] compaction filter, style
  - [ ] LRU cache
  - [ ] destroy/repair
  - [ ] comparator
  - [ ] slicetransform
  - [ ] windows support

Feedback and pull requests welcome!
