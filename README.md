rust-rocksdb
============

### running
- Cargo.toml
```rust
[dependencies.rocksdb]                                                                                                                                                                              
git = "https://github.com/spacejam/rust-rocksdb"
```
- Code
```rust
extern crate rocksdb;                                                                                                                                                                               
                                                                                                                                                                                                    
fn main() {                                                                                                                                                                                         
    let db = rocksdb::open("/path/to/db".to_string(), true).unwrap();                                                                                                                               
    assert!(db.put(b"hey", b"v1111").is_ok());                                                                                                                                                      
    db.get(b"hey").map( |raw| {                                                                                                                                                                     
        std::str::from_utf8(raw.as_slice()).map( |v| {                                                                                                                                              
            println!("value: {}", v);                                                                                                                                                               
        })                                                                                                                                                                                          
    });                                                                                                                                                                                             
    db.close()                                                                                                                                                                                      
}
```

### status
  - [x] basic open/put/get/close
  - [x] linux support
  - [x] rocksdb compiled via cargo
  - [x] OSX support
  - [ ] column family operations
  - [ ] LRU cache
  - [ ] destroy/repair
  - [ ] batch
  - [ ] iterator
  - [ ] create/release snapshot
  - [ ] range
  - [ ] rustic merge operator
  - [ ] compaction filter, style
  - [ ] comparator
  - [ ] slicetransform
  - [ ] logger
  - [ ] windows support

- Feedback and pull requests welcome!
