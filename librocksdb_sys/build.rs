use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

macro_rules! t {
    ($e:expr) => (match $e {
        Ok(n) => n,
        Err(e) => panic!("\n{} failed with {}\n", stringify!($e), e),
    })
}

fn main() {
    let want_static = env::var("ROCKSDB_SYS_STATIC").map(|s| s == "1").unwrap_or(false);
    if !want_static {
        return;
    }

    let target = env::var("TARGET").unwrap();
    if !target.contains("linux") && !target.contains("darwin") {
        // only linux and apple support static link right now
        return;
    }

    let dst = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let build = dst.join("build");
    t!(fs::create_dir_all(&build));

    let fest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let p = PathBuf::from(fest_dir).join("build.sh");
    for lib in &["z", "snappy", "bz2", "lz4", "rocksdb"] {
        let lib_name = format!("lib{}.a", lib);
        let src = build.join(&lib_name);
        let dst = dst.join(&lib_name);
        
        if dst.exists() {
            continue;
        }

        if !src.exists() {
            let mut cmd = Command::new(p.as_path());
            cmd.current_dir(&build).args(&[format!("compile_{}", lib)]);
            if *lib == "rocksdb" {
                if let Some(s) = env::var("ROCKSDB_SYS_PORTABLE").ok() {
                    cmd.env("PORTABLE", s);
                }
            }
            run(&mut cmd);
        }

        if let Err(e) = fs::rename(src.as_path(), dst.as_path()) {
            panic!("failed to move {} to {}: {:?}", src.display(), dst.display(), e);
        }
    }

    println!("cargo:rustc-link-lib=static=rocksdb");
    println!("cargo:rustc-link-lib=static=z");
    println!("cargo:rustc-link-lib=static=bz2");
    println!("cargo:rustc-link-lib=static=lz4");
    println!("cargo:rustc-link-lib=static=snappy");
    println!("cargo:rustc-link-search=native={}", dst.display());

    let mut cpp_linked = false;
    if let Ok(libs) = env::var("ROCKSDB_OTHER_STATIC") {
        for lib in libs.split(":") {
            if lib == "stdc++" {
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
    // TODO: find a reliable way to link stdc++ statically.
    if !cpp_linked {
        println!("cargo:rustc-link-lib=stdc++");
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
