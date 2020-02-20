fn verify_lib_dir<P: AsRef<std::path::Path>>(lib_dir: P) {
    let lib_dir = lib_dir
        .as_ref()
        .canonicalize()
        .unwrap_or_else(|_| panic!("Failed to canonicalize {:?}", lib_dir.as_ref()));
    let dir_lst = std::fs::read_dir(&lib_dir)
        .unwrap_or_else(|_| panic!("Failed to open library dir {:?}", lib_dir));
    if dir_lst.count() == 0 {
        panic!(
            "Library dir {:?} is empty and is probably incorrect. Verify it is the correct path."
        )
    }
}

fn get_lib_dir(lib_name: &str) -> String {
    let lib_var_name = format!("{}_LIB_DIR", lib_name.to_uppercase());
    match std::env::var(&lib_var_name) {
        Ok(lib_dir) => lib_dir,
        Err(_) => panic!(
            "You must set {} and it must be valid UTF-8 in order to link this library!",
            lib_var_name
        ),
    }
}

fn dyn_link_lib(lib_name: &str) {
    let lib_dir = get_lib_dir(lib_name);
    verify_lib_dir(&lib_dir);
    println!("cargo:rustc-link-search=native={}", lib_dir);
    println!("cargo:rustc-link-lib=dylib={}", lib_name);
}

pub fn link_dependencies() {
    #[cfg(feature = "bzip2")]
    dyn_link_lib("bzip2");
    #[cfg(feature = "lz4")]
    dyn_link_lib("lz4");
    #[cfg(feature = "snappy")]
    dyn_link_lib("snappy");
    #[cfg(feature = "zlib")]
    dyn_link_lib("zlib");
    #[cfg(feature = "zstd")]
    dyn_link_lib("zstd");

    dyn_link_lib("rocksdb");
}
