#![crate_id = "rocksdb"]
#![crate_type = "lib"]

pub use rocksdb::{
    open,
    Rocksdb,
};
pub mod rocksdb;
