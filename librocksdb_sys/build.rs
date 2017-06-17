extern crate gcc;
extern crate pkg_config;

use std::{env, fs, str};
use std::path::PathBuf;
use std::process::Command;

macro_rules! t {
    ($e:expr) => (match $e {
        Ok(n) => n,
        Err(e) => panic!("\n{} failed with {}\n", stringify!($e), e),
    })
}

fn main() {
    const ROCKSDB_VERSION: &'static str = "5.4";

    if !cfg!(feature = "static-link") {
        match pkg_config::Config::new().atleast_version(ROCKSDB_VERSION).probe("rocksdb") {
            Ok(_) => (),
            Err(_) => {
                panic!("failed to find rocksdb >= {} by pkg-config", ROCKSDB_VERSION);
            }
        }
        gcc::Config::new()
        .cpp(true)
        .file("crocksdb/c.cc")
        .flag("-std=c++11")
        .flag("-fPIC")
        .flag("-O2")
        .compile("libcrocksdb.a");

        println!("cargo:rustc-link-lib=static=crocksdb");
        println!("cargo:rustc-link-lib=rocksdb");

        return;
    }

    if !cfg!(target_os = "linux") && !cfg!(target_os = "macos") {
        // only linux and apple support static link right now
        return;
    }

    let dst = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let build = dst.join("build");
    t!(fs::create_dir_all(&build));

    let fest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let p = PathBuf::from(fest_dir.clone()).join("build.sh");
    let crocksdb_path = PathBuf::from(fest_dir).join("crocksdb");
    env::set_var("CROCKSDB_PATH", crocksdb_path.to_str().unwrap());
    for lib in &["z", "snappy", "bz2", "lz4", "zstd", "rocksdb"] {
        let lib_name = format!("lib{}.a", lib);
        let src = build.join(&lib_name);
        let dst = dst.join(&lib_name);

        if dst.exists() && *lib != "rocksdb" {
            continue;
        }

        if *lib == "rocksdb" && src.exists() {
            fs::remove_dir_all(&src).unwrap();
            if dst.exists() {
                fs::remove_file(&dst).unwrap();
            }
        }

        if !src.exists() {
            let mut cmd = Command::new(p.as_path());
            cmd.current_dir(&build).args(&[format!("compile_{}", lib)]);
            if *lib == "rocksdb" {
                if cfg!(feature = "portable") {
                    cmd.env("PORTABLE", "1");
                }

                if cfg!(feature = "sse") {
                    cmd.env("USE_SSE", "1");
                }
            }
            run(&mut cmd);
        }

        if let Err(e) = fs::rename(src.as_path(), dst.as_path()) {
            panic!("failed to move {} to {}: {:?}",
                   src.display(),
                   dst.display(),
                   e);
        }
    }

    println!("cargo:rustc-link-lib=static=rocksdb");
    println!("cargo:rustc-link-lib=static=z");
    println!("cargo:rustc-link-lib=static=bz2");
    println!("cargo:rustc-link-lib=static=lz4");
    println!("cargo:rustc-link-lib=static=zstd");
    println!("cargo:rustc-link-lib=static=snappy");
    println!("cargo:rustc-link-search=native={}", dst.display());

    let mut cpp_linked = false;
    let std_lib_name = if cfg!(target_os = "linux") {
        "libstdc++.a"
    } else {
        "libc++.a"
    };
    let short_std_lib_name = &std_lib_name[3..std_lib_name.len() - 2];
    if let Ok(libs) = env::var("ROCKSDB_OTHER_STATIC") {
        for lib in libs.split(":") {
            if lib == short_std_lib_name {
                cpp_linked = true;
            }
            println!("cargo:rustc-link-lib=static={}", lib);
        }
        if let Ok(pathes) = env::var("ROCKSDB_OTHER_STATIC_PATH") {
            for p in pathes.split(":") {
                println!("cargo:rustc-link-search=native={}", p);
            }
        }
    }
    if !cpp_linked {
        let output =
            Command::new(p.as_path()).args(&["find_library", std_lib_name]).output().unwrap();
        if output.status.success() && !output.stdout.is_empty() {
            if let Ok(path_str) = str::from_utf8(&output.stdout) {
                let path = PathBuf::from(path_str);
                if path.is_absolute() {
                    println!("cargo:rustc-link-lib=static=stdc++");
                    println!("cargo:rustc-link-search=native={}",
                             path.parent().unwrap().display());
                    return;
                }
            }
        }
        println!("failed to detect {}: {:?}, fallback to dynamic",
                 std_lib_name,
                 output);
        println!("cargo:rustc-link-lib={}", &short_std_lib_name);
    }
}

fn run(cmd: &mut Command) {
    println!("running: {:?}", cmd);
    let status = match cmd.status() {
        Ok(s) => s,
        Err(e) => panic!("{:?} failed: {}", cmd, e),
    };
    if !status.success() {
        panic!("{:?} failed: {}", cmd, status);
    }
}
