#!/bin/bash

# NOTE: 
# This script is only used when you want to generate bindings yourself.
# The generated bindings will overwrite librocksdb_sys/bindings/*

export UPDATE_BIND=1
cargo build  --target x86_64-unknown-linux-gnu
rustfmt librocksdb_sys/bindings/*
