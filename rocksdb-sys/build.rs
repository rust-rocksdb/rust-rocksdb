extern crate cmake;

use std::env;

use cmake::Config;

fn main() {
	let mut cfg = Config::new("rocksdb");
	cfg.define("CMAKE_VERBOSE_MAKEFILE", "ON")
		.register_dep("SNAPPY")
		.define("WITH_SNAPPY", "ON")
		.build_target("rocksdb");

	if cfg!(target_env = "msvc") {
		cfg.env("SNAPPY_INCLUDE", env::var_os("DEP_SNAPPY_INCLUDE").expect("DEP_SNAPPY_INCLUDE is set in snappy."));

		println!("cargo:rustc-link-lib=dylib={}", "rpcrt4");
		println!("cargo:rustc-link-lib=dylib={}", "shlwapi");
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
	} else if cfg!(not(target_env = "msvc")) {
		println!("cargo:rustc-link-lib=stdc++");
	}
}
