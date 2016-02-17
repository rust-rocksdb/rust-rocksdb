extern crate pkg_config;

use std::fs;
use std::path::Path;
use std::env;
use std::process::{Command, exit};
use std::io::{BufRead, BufReader, Write, stderr};


fn main() {
    configure_librocksdb();
}

#[cfg(feature = "static")]
fn as_static() -> bool {
	true
}

#[cfg(not(feature = "static"))]
fn as_static() -> bool {
	env::var_os("ROCKSDB_STATIC").is_some()
}

fn configure_librocksdb() {
    // try pkg_config first
    if pkg_config::find_library("rocksdb").is_ok() {
        return;
    }

	let target = env::var("TARGET").unwrap();
	let cpp = if target.contains("darwin") { "c++" } else { "stdc++" };
	println!("cargo:rustc-flags=-l {}", cpp);

    if as_static() {
		println!("cargo:rustc-link-lib=static=rocksdb");
		if let Some(path) = first_path_with_file("librocksdb.a") {
			println!("cargo:rustc-link-search=native={}", path);
			find_and_link_dependency("snappy");
			find_and_link_dependency("z");
			find_and_link_dependency("bz2");
			find_and_link_dependency("lz4");
			return;
		}
    } else {
		println!("cargo:rustc-link-lib=dylib=rocksdb");
		let soname = soname("rocksdb");
		if let Some(_) = first_path_with_file(&soname) {
			return;
		}
    };

	// system library not found, compile our own version
	let mut stderr = stderr();
	let out_dir = env::var("OUT_DIR").unwrap();
	let num_jobs = env::var("NUM_JOBS");

	let mut cmd = Command::new("make");

	cmd.current_dir(Path::new("rocksdb"))
	//	.arg("EXTRA_CFLAGS=-fPIC")
	//	.arg("EXTRA_CXXFLAGS=-fPIC") 
		.arg(format!("INSTALL_PATH={}", out_dir));

	if let Ok(jobs) = num_jobs {
		cmd.arg(format!("-j{}", jobs));
	}

	if as_static() {
		cmd.arg("install-static");
	} else {
		cmd.arg("install-shared");
	}

	match cmd.output() {
		Ok(out) => if !out.status.success() {
			let _ = writeln!(&mut stderr, "build failed:\n{}", String::from_utf8(out.stderr).unwrap());
			exit(1);
		},
		Err(e) => { let _ = writeln!(&mut stderr, "command execution failed: {:?}", e); exit(1) }
	}

	let config = match fs::File::open("rocksdb/make_config.mk") {
		Ok(c) => c,
		Err(e) => { let _ = writeln!(&mut stderr, "Failed to open rocksdb/make_config.mk: {}", e); exit(1) }
	};
	let config = BufReader::new(config);

	let mut lz4 = false;
	let mut snappy = false;
	let mut zlib = false;
	let mut bzip2 = false;

	for line in config.lines() {
		let line = line.unwrap();
		let words: Vec<_> = line.split_whitespace().collect();

		if !words[0].starts_with("PLATFORM_LDFLAGS=") {
			continue
		}

		lz4 = words.iter().any(|w| *w == "-llz4");
		snappy = words.iter().any( |w| *w == "-lsnappy");
		zlib = words.iter().any(|w| *w == "-lz");
		bzip2 = words.iter().any(|w| *w == "-lbz2");
		break;
	}
	println!("cargo:rustc-link-search=native={}/lib", out_dir);
	if lz4 { println!("cargo:rustc-link-lib=lz4"); }
	if snappy { println!("cargo:rustc-link-lib=snappy"); }
	if zlib { println!("cargo:rustc-link-lib=z"); }
	if bzip2 { println!("cargo:rustc-link-lib=bz2"); }
}

fn find_and_link_dependency(dep: &str) {
	if first_path_with_file(&format!("lib{}.a", dep)).is_some() {
		println!("cargo:rustc-link-lib=static={}", dep);
	} else if first_path_with_file(&soname(dep)).is_some() {
		println!("cargo:rustc-link-lib=dylib={}", dep);
	}
}
fn first_path_with_file(file: &str) -> Option<String> {
    // we want to look in LD_LIBRARY_PATH and then some default folders
    if let Some(ld_path) = env::var_os("LD_LIBRARY_PATH") {
        for p in env::split_paths(&ld_path) {
            if is_file_in(file, &p) {
                return p.to_str().map(|s| String::from(s))
            }
        }
    }
    for p in vec!["/usr/lib","/usr/local/lib"] {
        if is_file_in(file, &Path::new(p)) {
            return Some(String::from(p))
        }
    }
    return None
}

fn is_file_in(file: &str, folder: &Path) -> bool {
    let full = folder.join(file);
    match fs::metadata(full) {
        Ok(ref found) if found.is_file() => true,
        _ => false
    }

}

fn soname(name: &str) -> String {
	let target = env::var("TARGET").unwrap();
	if target.contains("darwin") { format!("lib{}.dylib", name) } else { format!("lib{}.so", name) }
}

