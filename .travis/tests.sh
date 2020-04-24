#!/bin/bash

cargo test --manifest-path=librocksdb-sys/Cargo.toml
cargo test -- --skip test_iterator_outlive_db
