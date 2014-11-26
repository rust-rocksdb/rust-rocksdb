rust-rocksdb
============

* rust wrapper for rocksdb
* development began 11/16/14
* status (uncompleted tasks are not ranked by priority):
  - [x] basic open/put/get/close
  - [x] linux support
  - [x] rocksdb itself retrieved and compiled via cargo
  - [ ] OSX support
  - [ ] windows support
  - [ ] batch
  - [ ] iterator
  - [ ] create/release snapshot
  - [ ] range
  - [ ] compaction filter, style
  - [ ] rustic merge operator
  - [ ] comparator
  - [ ] slicetransform
  - [ ] LRU cache
  - [ ] windows support
  - [ ] logger
  - [ ] column family operations
  - [ ] destroy/repair

### running
- Cargo.toml
```
[dependencies.rocksdb]                                                                                                                                                                              
git = "https://github.com/spacejam/rust-rocksdb"
```
- Code
```
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

- Feedback and pull requests welcome!
