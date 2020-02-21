fn bindgen_rocksdb() {
    let bindings = bindgen::Builder::default()
        .header("rocksdb/include/rocksdb/c.h")
        .derive_debug(false)
        .blacklist_type("max_align_t") // https://github.com/rust-lang-nursery/rust-bindgen/issues/550
        .ctypes_prefix("libc")
        .size_t_is_usize(true)
        .generate()
        .expect("Unable to generate RocksDB bindings");
    let out_path = std::env::var("OUT_DIR").unwrap();
    let out_path = std::path::PathBuf::from(out_path);
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write RocksDB bindings!");
}

#[cfg(feature = "vendor")]
mod vendor;

#[cfg(not(feature = "vendor"))]
mod system;

fn main() {
    #[cfg(feature = "vendor")]
    vendor::vendor_dependencies();

    #[cfg(not(feature = "vendor"))]
    system::link_dependencies();

    bindgen_rocksdb();
}
