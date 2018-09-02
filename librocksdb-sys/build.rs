extern crate cc;
extern crate bindgen;

use std::env;
use std::fs;
use std::path::PathBuf;

fn link(name: &str, bundled: bool) {
    use std::env::var;
    let target = var("TARGET").unwrap();
    let target: Vec<_> = target.split('-').collect();
    if target.get(2) == Some(&"windows") {
        println!("cargo:rustc-link-lib=dylib={}", name);
        if bundled && target.get(3) == Some(&"gnu") {
            let dir = var("CARGO_MANIFEST_DIR").unwrap();
            println!("cargo:rustc-link-search=native={}/{}", dir, target[0]);
        }
    }
}

fn fail_on_empty_directory(name: &str) {
    if fs::read_dir(name).unwrap().count() == 0 {
        println!(
            "The `{}` directory is empty, did you forget to pull the submodules?",
            name
        );
        println!("Try `git submodule update --init --recursive`");
        panic!();
    }
}

fn bindgen_rocksdb() {
    let bindings = bindgen::Builder::default()
        .header("rocksdb/include/rocksdb/c.h")
        .blacklist_type("max_align_t") // https://github.com/rust-lang-nursery/rust-bindgen/issues/550
        .ctypes_prefix("libc")
        .generate()
        .expect("unable to generate rocksdb bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("unable to write rocksdb bindings");
}

fn build_rocksdb() {
    let mut config = cc::Build::new();
    config.include("rocksdb/include/");
    config.include("rocksdb/");
    config.include("rocksdb/third-party/gtest-1.7.0/fused-src/");
    config.include("snappy/");
    config.include(".");

    config.define("NDEBUG", Some("1"));
    config.define("SNAPPY", Some("1"));

    let mut lib_sources = include_str!("rocksdb_lib_sources.txt")
        .split(" ")
        .collect::<Vec<&'static str>>();

    // We have a pregenerated a version of build_version.cc in the local directory
    lib_sources = lib_sources
        .iter()
        .cloned()
        .filter(|file| *file != "util/build_version.cc")
        .collect::<Vec<&'static str>>();

    if cfg!(target_os = "macos") {
        config.define("OS_MACOSX", Some("1"));
        config.define("ROCKSDB_PLATFORM_POSIX", Some("1"));
        config.define("ROCKSDB_LIB_IO_POSIX", Some("1"));

    }
    if cfg!(target_os = "linux") {
        config.define("OS_LINUX", Some("1"));
        config.define("ROCKSDB_PLATFORM_POSIX", Some("1"));
        config.define("ROCKSDB_LIB_IO_POSIX", Some("1"));
        // COMMON_FLAGS="$COMMON_FLAGS -fno-builtin-memcmp"
    }
    if cfg!(target_os = "freebsd") {
        config.define("OS_FREEBSD", Some("1"));
        config.define("ROCKSDB_PLATFORM_POSIX", Some("1"));
        config.define("ROCKSDB_LIB_IO_POSIX", Some("1"));
    }

    if cfg!(windows) {
        link("rpcrt4", false);
        link("shlwapi", true);

        config.define("OS_WIN", Some("1"));

        // Remove POSIX-specific sources
        lib_sources = lib_sources
            .iter()
            .cloned()
            .filter(|file| match *file {
                "port/port_posix.cc" |
                "env/env_posix.cc" |
                "env/io_posix.cc" => false,
                _ => true,
            })
            .collect::<Vec<&'static str>>();

        // Add Windows-specific sources
        lib_sources.push("port/win/port_win.cc");
        lib_sources.push("port/win/env_win.cc");
        lib_sources.push("port/win/env_default.cc");
        lib_sources.push("port/win/win_logger.cc");
        lib_sources.push("port/win/io_win.cc");
        lib_sources.push("port/win/win_thread.cc");
    }

    if cfg!(target_env = "msvc") {
        config.flag("-EHsc");
    } else {
        config.flag("-std=c++11");
        // this was breaking the build on travis due to
        // > 4mb of warnings emitted.
        config.flag("-Wno-unused-parameter");
    }

    for file in lib_sources {
        let file = "rocksdb/".to_string() + file;
        config.file(&file);
    }

    config.file("build_version.cc");

    config.cpp(true);
    config.compile("librocksdb.a");
}

fn build_snappy() {
    let mut config = cc::Build::new();
    config.include("snappy/");
    config.include(".");

    config.define("NDEBUG", Some("1"));

    if cfg!(target_env = "msvc") {
        config.flag("-EHsc");
    } else {
        config.flag("-std=c++11");
    }

    config.file("snappy/snappy.cc");
    config.file("snappy/snappy-sinksource.cc");
    config.file("snappy/snappy-c.cc");
    config.cpp(true);
    config.compile("libsnappy.a");
}

fn try_to_find_and_link_lib(lib_name: &str) -> bool {
    if let Ok(lib_dir) = env::var(&format!("{}_LIB_DIR", lib_name)) {
        println!("cargo:rustc-link-search=native={}", lib_dir);
        let mode = match env::var_os(&format!("{}_STATIC", lib_name)) {
            Some(_) => "static",
            None => "dylib",
        };
        println!("cargo:rustc-link-lib={}={}", mode, lib_name.to_lowercase());
        return true;
    }
    false
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=rocksdb/");
    println!("cargo:rerun-if-changed=snappy/");

    fail_on_empty_directory("rocksdb");
    fail_on_empty_directory("snappy");
    bindgen_rocksdb();

    if !try_to_find_and_link_lib("ROCKSDB") {
        build_rocksdb();
    }
    if !try_to_find_and_link_lib("SNAPPY") {
        build_snappy();
    }
}
