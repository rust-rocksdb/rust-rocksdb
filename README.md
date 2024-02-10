rust-rocksdb
============

[![crates.io](https://img.shields.io/crates/v/rust-rocksdb.svg)](https://crates.io/crates/rust-rocksdb)
[![documentation](https://docs.rs/rust-rocksdb/badge.svg)](https://docs.rs/rust-rocksdb)
[![license](https://img.shields.io/crates/l/rust-rocksdb.svg)](https://github.com/zaidoon1/rust-rocksdb/blob/master/LICENSE)
![rust 1.75.0 required](https://img.shields.io/badge/rust-1.75.0-blue.svg?label=MSRV)

## Why The Fork

The original rust-rocksdb repo is amazing and I appreciate all the work that has
been done, however, for my use case, I need to stay up to date with the latest
rocksdb releases as well as the latest rust releases so in order to to keep
everything up to date, I decided to fork the original repo so I can have total
control and be able to create regular releases.

## Requirements

- Clang and LLVM

## Contributing

Feedback and pull requests welcome!  If a particular feature of RocksDB is
important to you, please let me know by opening an issue, and I'll
prioritize it.

## Usage

This binding is statically linked with a specific version of RocksDB. If you
want to build it yourself, make sure you've also cloned the RocksDB and
compression submodules:

    git submodule update --init --recursive

## Compression Support

By default, support for the [Snappy](https://github.com/google/snappy),
[LZ4](https://github.com/lz4/lz4), [Zstd](https://github.com/facebook/zstd),
[Zlib](https://zlib.net), and [Bzip2](http://www.bzip.org) compression
is enabled through crate features.  If support for all of these compression
algorithms is not needed, default features can be disabled and specific
compression algorithms can be enabled. For example, to enable only LZ4
compression support, make these changes to your Cargo.toml:

```
[dependencies.rocksdb]
default-features = false
features = ["lz4"]
```

## Multithreaded ColumnFamily alternation

The underlying RocksDB does allow column families to be created and dropped
from multiple threads concurrently. But this crate doesn't allow it by default
for compatibility. If you need to modify column families concurrently, enable
crate feature called `multi-threaded-cf`, which makes this binding's
data structures to use RwLock by default. Alternatively, you can directly create
`DBWithThreadMode<MultiThreaded>` without enabling the crate feature.
