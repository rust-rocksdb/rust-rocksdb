extern crate cc;
extern crate cmake;

fn main() {
    let cur_dir = std::env::current_dir().unwrap();
    let dst = cmake::Config::new("titan")
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
        .build();
    println!("cargo:rustc-link-search=native={}/build", dst.display());
    println!("cargo:rustc-link-lib=static=titan");
}
