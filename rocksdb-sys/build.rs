extern crate "pkg-config" as pkg_config;

use std::os;
use std::io::{mod, fs, Command};
use std::io::process::InheritFd;

//TODO windows support

fn main() {
  // Next, fall back and try to use pkg-config if its available.
  match pkg_config::find_library("librocksdb") {
    Ok(()) => return,
    Err(..) => {}
  }

  let src = os::getcwd().unwrap();
  let dst = Path::new(os::getenv("OUT_DIR").unwrap());

  let _ = fs::mkdir(&dst.join("build"), io::USER_DIR);

  println!("cwd: {}", src.join("rocksdb").as_str());
  run(Command::new(make())
        .arg("shared_lib")
        .arg(format!("-j{}", os::getenv("NUM_JOBS").unwrap()))
        .cwd(&src.join("rocksdb")));

  // Don't run `make install` because apparently it's a little buggy on mingw
  // for windows.
  fs::mkdir_recursive(&dst.join("lib/pkgconfig"), io::USER_DIR).unwrap();

  let target = os::getenv("TARGET").unwrap();
  if target.contains("apple") {
    fs::rename(&src.join("rocksdb/librocksdb.dylib"), &dst.join("lib/librocksdb.dylib")).unwrap();
  } else {
    fs::rename(&src.join("rocksdb/librocksdb.so"), &dst.join("lib/librocksdb.so")).unwrap();
  }

  println!("cargo:rustc-flags=-L {}/lib -l rocksdb:dylib", dst.display());
  println!("cargo:root={}", dst.display());
  println!("cargo:include={}/include", src.join("rocksdb").display());
}

fn run(cmd: &mut Command) {
  println!("running: {}", cmd);
  assert!(cmd.stdout(InheritFd(1))
         .stderr(InheritFd(2))
         .status()
         .unwrap()
         .success());

}

fn make() -> &'static str {
  if cfg!(target_os = "freebsd") {"gmake"} else {"make"}
}
