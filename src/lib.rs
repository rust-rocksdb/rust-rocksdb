#![crate_id = "rocksdb"]
#![crate_type = "lib"]
#![allow(dead_code)]

pub use ffi as rocksdb_ffi;
pub use rocksdb::{
    RocksDB,
    RocksDBResult,
};
pub mod rocksdb;
pub mod ffi;
