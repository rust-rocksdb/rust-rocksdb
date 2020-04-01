// Copyright 2017 PingCAP, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// See the License for the specific language governing permissions and
// limitations under the License.

extern crate bindgen;
extern crate cc;
extern crate cmake;

use cc::Build;
use cmake::Config;
use std::path::PathBuf;
use std::{env, str};

// On these platforms jemalloc-sys will use a prefixed jemalloc which cannot be linked together
// with RocksDB.
// See https://github.com/gnzlbg/jemallocator/blob/bfc89192971e026e6423d9ee5aaa02bc56585c58/jemalloc-sys/build.rs#L45
const NO_JEMALLOC_TARGETS: &[&str] = &["android", "dragonfly", "musl", "darwin"];

// Generate the bindings to rocksdb C-API.
// Try to disable the generation of platform-related bindings.
fn bindgen_rocksdb(file_path: &PathBuf) {
    let bindings = bindgen::Builder::default()
        .header("crocksdb/crocksdb/c.h")
        .ctypes_prefix("libc")
        .generate()
        .expect("unable to generate rocksdb bindings");

    bindings
        .write_to_file(file_path)
        .expect("unable to write rocksdb bindings");
}

// Determine if need to update bindings. Supported platforms do not
// need to be updated by default unless the UPDATE_BIND is specified.
// Other platforms use bindgen to generate the bindings every time.
fn config_binding_path() {
    let file_path: PathBuf;

    let target = env::var("TARGET").unwrap_or_else(|_| "".to_owned());
    match target.as_str() {
        "x86_64-unknown-linux-gnu" | "aarch64-unknown-linux-gnu" => {
            file_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
                .join("bindings")
                .join(format!("{}-bindings.rs", target));
            if env::var("UPDATE_BIND")
                .map(|s| s.as_str() == "1")
                .unwrap_or(false)
            {
                bindgen_rocksdb(&file_path);
            }
        }
        _ => {
            file_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("rocksdb-bindings.rs");
            bindgen_rocksdb(&file_path);
        }
    };
    println!(
        "cargo:rustc-env=BINDING_PATH={}",
        file_path.to_str().unwrap()
    );
}

fn main() {
    println!("cargo:rerun-if-env-changed=UPDATE_BIND");

    let mut build = build_rocksdb();

    build.cpp(true).file("crocksdb/c.cc");
    if !cfg!(target_os = "windows") {
        build.flag("-std=c++11");
        build.flag("-fno-rtti");
    }
    link_cpp(&mut build);
    build.warnings(false).compile("libcrocksdb.a");
}

fn link_cpp(build: &mut Build) {
    let tool = build.get_compiler();
    let stdlib = if tool.is_like_gnu() {
        "libstdc++.a"
    } else if tool.is_like_clang() {
        "libc++.a"
    } else {
        // Don't link to c++ statically on windows.
        return;
    };
    let output = tool
        .to_command()
        .arg("--print-file-name")
        .arg(stdlib)
        .output()
        .unwrap();
    if !output.status.success() || output.stdout.is_empty() {
        // fallback to dynamically
        return;
    }
    let path = match str::from_utf8(&output.stdout) {
        Ok(path) => PathBuf::from(path),
        Err(_) => return,
    };
    if !path.is_absolute() {
        return;
    }
    // remove lib prefix and .a postfix.
    let libname = &stdlib[3..stdlib.len() - 2];
    // optional static linking
    if cfg!(feature = "static_libcpp") {
        println!("cargo:rustc-link-lib=static={}", &libname);
    } else {
        println!("cargo:rustc-link-lib=dylib={}", &libname);
    }
    println!(
        "cargo:rustc-link-search=native={}",
        path.parent().unwrap().display()
    );
    build.cpp_link_stdlib(None);
}

fn build_rocksdb() -> Build {
    let target = env::var("TARGET").expect("TARGET was not set");
    let mut cfg = Config::new("rocksdb");
    if cfg!(feature = "encryption") {
        cfg.register_dep("OPENSSL").define("WITH_OPENSSL", "ON");
        println!("cargo:rustc-link-lib=static=crypto");
    }
    if cfg!(feature = "jemalloc") && NO_JEMALLOC_TARGETS.iter().all(|i| !target.contains(i)) {
        cfg.register_dep("JEMALLOC").define("WITH_JEMALLOC", "ON");
        println!("cargo:rustc-link-lib=static=jemalloc");
    }
    if cfg!(feature = "portable") {
        cfg.define("PORTABLE", "ON");
    }
    if cfg!(feature = "sse") {
        cfg.define("FORCE_SSE42", "ON");
    }
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
    let dst = cfg
        .define("WITH_GFLAGS", "OFF")
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
        .define("WITH_TESTS", "OFF")
        .define("WITH_TOOLS", "OFF")
        .build_target("rocksdb")
        .very_verbose(true)
        .build();
    let build_dir = format!("{}/build", dst.display());
    let mut build = Build::new();
    if cfg!(target_os = "windows") {
        let profile = match &*env::var("PROFILE").unwrap_or_else(|_| "debug".to_owned()) {
            "bench" | "release" => "Release",
            _ => "Debug",
        };
        println!("cargo:rustc-link-search=native={}/{}", build_dir, profile);
        build.define("OS_WIN", None);
    } else {
        println!("cargo:rustc-link-search=native={}", build_dir);
        build.define("ROCKSDB_PLATFORM_POSIX", None);
    }
    if cfg!(target_os = "macos") {
        build.define("OS_MACOSX", None);
    } else if cfg!(target_os = "freebsd") {
        build.define("OS_FREEBSD", None);
    }

    config_binding_path();

    let cur_dir = env::current_dir().unwrap();
    build.include(cur_dir.join("rocksdb").join("include"));
    build.include(cur_dir.join("rocksdb"));
    build.include(cur_dir.join("libtitan_sys").join("titan").join("include"));
    build.include(cur_dir.join("libtitan_sys").join("titan"));

    // Adding rocksdb specific compile macros.
    // TODO: should make sure crocksdb compile options is the same as rocksdb and titan.
    build.define("ROCKSDB_SUPPORT_THREAD_LOCAL", None);
    if cfg!(feature = "encryption") {
        build.define("OPENSSL", None);
    }

    println!("cargo:rustc-link-lib=static=rocksdb");
    println!("cargo:rustc-link-lib=static=titan");
    println!("cargo:rustc-link-lib=static=z");
    println!("cargo:rustc-link-lib=static=bz2");
    println!("cargo:rustc-link-lib=static=lz4");
    println!("cargo:rustc-link-lib=static=zstd");
    println!("cargo:rustc-link-lib=static=snappy");
    build
}
