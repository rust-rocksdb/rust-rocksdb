fn verify_lib_dir<P: AsRef<std::path::Path>>(lib_dir: P) {
    let lib_dir = lib_dir
        .as_ref()
        .canonicalize()
        .unwrap_or_else(|_| panic!("Failed to canonicalize {:?}", lib_dir.as_ref()));
    let dir_lst = std::fs::read_dir(&lib_dir)
        .unwrap_or_else(|_| panic!("Failed to open library dir {:?}", lib_dir));
    if dir_lst.count() == 0 {
        panic!(
            "Library dir {:?} is empty and is probably incorrect. Verify it is the correct path.",
            lib_dir
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

// Use `alt_name` when the project and its library have different names, such as bzip2 which has
// `libbz2` as opposed to `libbzip2`.
fn link_lib(lib_name: &str, alt_name: Option<&str>) {
    #[cfg(feature = "static")]
    const LINK_TYPE: &str = "static";
    #[cfg(not(feature = "static"))]
    const LINK_TYPE: &str = "dylib";

    let lib_dir = get_lib_dir(lib_name);
    verify_lib_dir(&lib_dir);
    println!("cargo:rustc-link-search=native={}", lib_dir);
    println!(
        "cargo:rustc-link-lib={}={}",
        LINK_TYPE,
        alt_name.unwrap_or(lib_name)
    );
}

pub fn link_dependencies() {
    #[cfg(feature = "bzip2")]
    link_lib("bzip2", Some("bz2"));
    #[cfg(feature = "lz4")]
    link_lib("lz4", None);
    #[cfg(feature = "snappy")]
    link_lib("snappy", None);
    #[cfg(feature = "zlib")]
    link_lib("zlib", Some("z"));
    #[cfg(feature = "zstd")]
    link_lib("zstd", None);

    link_lib("rocksdb", None);
}
