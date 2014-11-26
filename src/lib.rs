#![crate_id = "rocksdb"]
#![crate_type = "lib"]
#![allow(dead_code)]

extern crate "rocksdb-sys" as rocksdb_ffi;

pub use rocksdb::{
    open,
    Rocksdb,
    RocksdbResult,
};
pub mod rocksdb;
