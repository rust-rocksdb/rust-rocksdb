rust-rocksdb
============
[![Build Status](https://travis-ci.org/spacejam/rust-rocksdb.svg?branch=master)](https://travis-ci.org/spacejam/rust-rocksdb)
[![crates.io](http://meritbadge.herokuapp.com/rocksdb)](https://crates.io/crates/rocksdb)
[![documentation](https://docs.rs/rocksdb/badge.svg)](https://docs.rs/rocksdb)

Feedback and pull requests welcome!  If a particular feature of RocksDB is important to you, please let me know by opening an issue, and I'll prioritize it.

This binding is statically linked with a specific version of RocksDB. If you want to build it yourself, make sure you've also cloned the RocksDB and Snappy submodules:

    git submodule update --init --recursive
