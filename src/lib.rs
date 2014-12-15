#![crate_id = "rocksdb"]
#![crate_type = "lib"]
#![allow(dead_code)]

pub use ffi as rocksdb_ffi;
pub use ffi::{
    new_bloom_filter,
    new_cache,
    RocksDBUniversalCompactionStyle,
    RocksDBCompactionStyle,
    RocksDBCompressionType,
};
pub use rocksdb::{
    RocksDB,
    MergeOperands,
    RocksDBResult,
    RocksDBOptions,
    RocksDBVector,
};
pub mod rocksdb;
pub mod ffi;
