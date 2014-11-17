rust-rocksdb
============

* rust wrapper for rocksdb
* development began 11/16/14
* status: minimal functionality with poor style and an annoying interface

```
fn f() {
    // arguments are path for rocksdb files, create if missing
    let db = Rocksdb::open("testdb", true).unwrap();
    db.put(b"a key", b"a value");
    ...
    let r = db.get(b"this is key").unwrap();
    db.close();
}
```

### running
- get rocksdb
```
git clone https://github.com/facebook/rocksdb
cd rocksdb
make shared_lib
```
- run tests
```
LD_PRELOAD=/path/to/rocksdb/librocksdb.so cargo test
```
- enjoy (as much as you can with such a poor current library!  stay tuned!)
