extern crate cc;
extern crate cmake;

use std::env;

fn main() {
    // RocksDB cmake script expect libz.a being under ${DEP_Z_ROOT}/lib, but libz-sys crate put it
    // under ${DEP_Z_ROOT}/build. Append the path to CMAKE_PREFIX_PATH to get around it.
    env::set_var("CMAKE_PREFIX_PATH", {
        let zlib_path = format!("{}/build", env::var("DEP_Z_ROOT").unwrap());
        if let Ok(prefix_path) = env::var("CMAKE_PREFIX_PATH") {
            format!("{};{}", prefix_path, zlib_path)
        } else {
            zlib_path
        }
    });
    let cur_dir = std::env::current_dir().unwrap();
    let mut cfg = cmake::Config::new("titan");
    if cfg!(feature = "portable") {
        cfg.define("PORTABLE", "ON");
    }
    if cfg!(feature = "sse") {
        cfg.define("FORCE_SSE42", "ON");
    }
    let dst = cfg
        .define("ROCKSDB_DIR", cur_dir.join("..").join("rocksdb"))
        .define("WITH_TITAN_TESTS", "OFF")
        .define("WITH_TITAN_TOOLS", "OFF")
        .register_dep("Z")
        .define("WITH_ZLIB", "ON")
        .register_dep("BZIP2")
        .define("WITH_BZ2", "ON")
        .register_dep("LZ4")
        .define("WITH_LZ4", "ON")
        .register_dep("ZSTD")
        .define("WITH_ZSTD", "ON")
        .register_dep("SNAPPY")
        .define("WITH_SNAPPY", "ON")
        .build_target("titan")
        .very_verbose(true)
        .build();
    println!("cargo:rustc-link-search=native={}/build", dst.display());
    println!("cargo:rustc-link-lib=static=titan");
}
