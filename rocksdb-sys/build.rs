extern crate cmake;

use std::env;

use cmake::Config;

fn main() {
    let mut cfg = Config::new("rocksdb");
    cfg.define("CMAKE_VERBOSE_MAKEFILE", "ON")
        .register_dep("SNAPPY")
        .define("WITH_SNAPPY", "ON")
        .build_target("rocksdb");

    let snappy = env::var_os("DEP_SNAPPY_INCLUDE").expect("DEP_SNAPPY_INCLUDE is set in snappy.");

    if cfg!(target_env = "msvc") {
        cfg.env("SNAPPY_INCLUDE", snappy);

        println!("cargo:rustc-link-lib=dylib={}", "rpcrt4");
        println!("cargo:rustc-link-lib=dylib={}", "shlwapi");

        let features = env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
        if features.contains("crt-static") {
            cfg.define("WITH_MD_LIBRARY", "OFF");
        }
    } else {
        cfg.define("SNAPPY_INCLUDE_DIR", snappy)
            .define("SNAPPY_LIBRARIES", "/dev/null");
    }

    // NOTE: the cfg! macro doesn't work when cross-compiling, it would return values for the host
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    if target_arch == "arm" || target_arch == "aarch64" {
        cfg.define("PORTABLE", "ON");
    }

    let out = cfg.build();

    let mut build = out.join("build");

    if cfg!(target_os = "windows") {
        let profile = match &*env::var("PROFILE").unwrap_or("debug".to_owned()) {
            "bench" | "release" => "Release",
            _ => "Debug",
        };
        build = build.join(profile);
    }

    println!("cargo:rustc-link-search=native={}", build.display());
    println!("cargo:rustc-link-lib=static=rocksdb");
    println!("cargo:rustc-link-lib=static=snappy");

    // https://github.com/alexcrichton/cc-rs/blob/ca70fd32c10f8cea805700e944f3a8d1f97d96d4/src/lib.rs#L891
    if cfg!(any(target_os = "macos", target_os = "freebsd", target_os = "openbsd")) {
        println!("cargo:rustc-link-lib=c++");
	} else if cfg!(not(any(target_env = "msvc", target_os = "android"))) {
        println!("cargo:rustc-link-lib=stdc++");
    }
}
