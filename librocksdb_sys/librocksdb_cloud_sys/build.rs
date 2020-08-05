// Copyright 2020 TiKV Project Authors. Licensed under Apache-2.0.
extern crate cc;
extern crate cmake;

use std::env;

fn main() {
    let cur_dir = env::current_dir().unwrap();
    let mut cfg = cmake::Config::new(".");
    let dst = cfg
        .define("ROCKSDB_DIR", cur_dir.join("..").join("rocksdb"))
        .env("USE_AWS", "1")
        .build_target("rocksdb_cloud")
        .very_verbose(true)
        .build();
    println!("cargo:rustc-link-search=native={}/build", dst.display());
    println!("cargo:rustc-link-lib=static=rocksdb_cloud");
}
