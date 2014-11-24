#![crate_id = "rocksdb"]
#![crate_type = "lib"]
#![feature(globs)]

pub use rocksdb::{
    open,
    Rocksdb,
    RocksdbResult,
};
pub mod rocksdb;
mod ffi;
