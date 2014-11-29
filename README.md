rust-rocksdb
============

### running
Install RocksDB.  rust-rocksdb has been tested with version 3.8.1 on linux and windows.
###### Cargo.toml
```rust
[dependencies.rocksdb]
git = "https://github.com/spacejam/rust-rocksdb"
```
###### Code
```rust
extern crate rocksdb;
use rocksdb::RocksDB;

fn main() {
  match RocksDB::open_default("/path/for/rocksdb/storage") {
    Ok(db) => {
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
    },
    Err(e) => panic!(e),
  }
}
```

### status
  - [x] basic open/put/get/delete/close
  - [x] linux support
  - [x] rocksdb compiled via cargo
  - [x] OSX support
  - [ ] rustic merge operator
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
