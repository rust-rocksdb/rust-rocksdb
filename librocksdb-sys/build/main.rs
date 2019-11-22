extern crate bindgen;

fn bindgen_builder_rocksdb() -> bindgen::Builder {
    bindgen::Builder::default()
        .header("./rocksdb/include/rocksdb/c.h")
        .derive_debug(false)
        .blacklist_type("max_align_t") // https://github.com/rust-lang-nursery/rust-bindgen/issues/550
        .ctypes_prefix("libc")
        .size_t_is_usize(true)
        .generate()
        .expect("Unable to generate rocksdb bindings");
}

fn bindgen_write_bindings(builder: bindgen::Builder) {
    let bindings = builder
        .generate()
        .expect("Unable to generate RocksDB bindings");
    let out_path = std::env::var("OUT_DIR").unwrap();
    let out_path = std::path::PathBuf::from(out_path);
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write RocksDB bindings!");
}

#[cfg(feature = "vendored")]
mod vendor;

#[cfg(not(feature = "vendored"))]
mod system;

// fn try_to_find_and_link_lib(lib_name: &str) -> bool {
//     if let Ok(lib_dir) = env::var(&format!("{}_LIB_DIR", lib_name)) {
//         println!("cargo:rustc-link-search=native={}", lib_dir);
//         let mode = match env::var_os(&format!("{}_STATIC", lib_name)) {
//             Some(_) => "static",
//             None => "dylib",
//         };
//         println!("cargo:rustc-link-lib={}={}", mode, lib_name.to_lowercase());
//         return true;
//     }
//     false
// }

fn main() {
    #[cfg(feature = "vendored")]
    vendor::vendor_dependencies();

    #[cfg(not(feature = "vendored"))]
    system::link_dependencies();
}
