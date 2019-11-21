extern crate bindgen;

fn bindgen_rocksdb() {
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .derive_debug(false)
        .blacklist_type("max_align_t") // https://github.com/rust-lang-nursery/rust-bindgen/issues/550
        .ctypes_prefix("libc")
        .size_t_is_usize(true)
        .generate()
        .expect("Unable to generate rocksdb bindings");

    let out_path = std::env::var("OUT_DIR").unwrap();
    let out_path = std::path::PathBuf::from(out_path);
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Unable to write rocksdb bindings");
}

#[cfg(feature = "vendored")]
mod vendor {
    extern crate cc;
    extern crate glob;

    use std::env;
    use std::fs;
    use std::path::Path;

    fn enforce_rerun<P: AsRef<Path>>(path: P) {
        println!("cargo:rerun-if-changed={}", path.as_ref().to_string_lossy());
    }

    fn check_submodule<P: AsRef<Path>>(path: P) {
        let path = path
            .as_ref()
            .canonicalize()
            .unwrap_or_else(|_| panic!("Failed to canonicalize {:?}", path.as_ref()));
        let dir =
            fs::read_dir(&path).unwrap_or_else(|_| panic!("Failed to open directory {:?}", path));
        if dir.count() == 0 {
            println!(
                "The `{:?}` directory is empty, did you forget to pull the submodules?",
                path
            );
            println!("Try `git submodule update --init --recursive`");
            panic!(
                "Missing submodule {}",
                path.file_name().unwrap().to_string_lossy()
            )
        }
    }

    fn windows_link(lib_name: &str) {
        println!("cargo:rustc-link-lib=dylib={}", lib_name);
    }

    fn build_rocksdb() {
        let target = env::var("TARGET").unwrap();
        let mut build = cc::Build::new();

        build
            .include("./rocksdb/include/")
            .include("./rocksdb/")
            .include("./rocksdb/third-party/gtest-1.7.0/fused-src/");

        #[cfg(feature = "bzip2")]
        {
            build.define("BZIP2", Some("1"));
            build.include("./bzip2/");
        }

        #[cfg(feature = "lz4")]
        {
            build.define("LZ4", Some("1"));
            build.include("./lz4/lib/");
        }

        #[cfg(feature = "snappy")]
        {
            build.define("SNAPPY", Some("1"));
            build.include("./snappy/");
        }

        #[cfg(feature = "zlib")]
        {
            build.define("ZLIB", Some("1"));
            build.include("./zlib/");
        }

        #[cfg(feature = "zstd")]
        {
            build.define("ZSTD", Some("1"));
            build
                .include("./zstd/lib/")
                .include("./zstd/lib/dictBuilder/");
        }

        build.include(".");
        build.define("NDEBUG", Some("1"));

        let mut lib_sources = include_str!("rocksdb_lib_sources.txt")
            .trim()
            .split("\n")
            .map(str::trim)
            .collect::<Vec<&'static str>>();

        // We have a pregenerated a version of build_version.cc in the local directory
        lib_sources = lib_sources
            .iter()
            .cloned()
            .filter(|file| *file != "util/build_version.cc")
            .collect::<Vec<&'static str>>();

        if target.contains("x86_64") {
            // This is needed to enable hardware CRC32C. Technically, SSE 4.2 is
            // only available since Intel Nehalem (about 2010) and AMD Bulldozer
            // (about 2011).
            build
                .define("HAVE_PCLMUL", Some("1"))
                .define("HAVE_SSE42", Some("1"))
                .flag_if_supported("-msse2")
                .flag_if_supported("-msse4.1")
                .flag_if_supported("-msse4.2")
                .flag_if_supported("-mpclmul");
        }

        if target.contains("darwin") {
            build
                .define("OS_MACOSX", Some("1"))
                .define("ROCKSDB_PLATFORM_POSIX", Some("1"))
                .define("ROCKSDB_LIB_IO_POSIX", Some("1"));
        } else if target.contains("android") {
            build
                .define("OS_ANDROID", Some("1"))
                .define("ROCKSDB_PLATFORM_POSIX", Some("1"))
                .define("ROCKSDB_LIB_IO_POSIX", Some("1"));
        } else if target.contains("linux") {
            build
                .define("OS_LINUX", Some("1"))
                .define("ROCKSDB_PLATFORM_POSIX", Some("1"))
                .define("ROCKSDB_LIB_IO_POSIX", Some("1"));
        } else if target.contains("freebsd") {
            build
                .define("OS_FREEBSD", Some("1"))
                .define("ROCKSDB_PLATFORM_POSIX", Some("1"))
                .define("ROCKSDB_LIB_IO_POSIX", Some("1"));
        } else if target.contains("windows") {
            windows_link("rpcrt4");
            windows_link("shlwapi");
            build
                .define("OS_WIN", Some("1"))
                .define("ROCKSDB_WINDOWS_UTF8_FILENAMES", Some("1"));
            if &target == "x86_64-pc-windows-gnu" {
                // Tell MinGW to create localtime_r wrapper of localtime_s function.
                build.define("_POSIX_C_SOURCE", None);
                // Tell MinGW to use at least Windows Vista headers instead of the ones of Windows XP.
                // (This is minimum supported version of rocksdb)
                build.define("_WIN32_WINNT", Some("0x0600"));
            }

            // Remove POSIX-specific sources
            lib_sources = lib_sources
                .iter()
                .cloned()
                .filter(|file| match *file {
                    "port/port_posix.cc" | "env/env_posix.cc" | "env/io_posix.cc" => false,
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

        if target.contains("msvc") {
            build.flag("-EHsc");
        } else {
            build.flag("-std=c++11");
            // this was breaking the build on travis due to
            // > 4mb of warnings emitted.
            build.flag("-Wno-unused-parameter");
        }

        for file in lib_sources {
            let file = "rocksdb/".to_string() + file;
            build.file(&file);
        }

        build.file("build_version.cc");

        build.cpp(true);
        build.compile("librocksdb.a");
    }

    #[cfg(feature = "bzip2")]
    fn build_bzip2() {
        let mut build = cc::Build::new();

        build.extra_warnings(false);
        build.opt_level(3);

        build
            .define("_FILE_OFFSET_BITS", Some("64"))
            .define("BZ_NO_STDIO", None);

        build
            .file("./bzip2/blocksort.c")
            .file("./bzip2/bzlib.c")
            .file("./bzip2/compress.c")
            .file("./bzip2/crctable.c")
            .file("./bzip2/decompress.c")
            .file("./bzip2/huffman.c")
            .file("./bzip2/randtable.c");

        build.compile("libbz2.a");
    }

    #[cfg(feature = "lz4")]
    fn build_lz4() {
        let target = env::var("TARGET").unwrap();
        let mut build = cc::Build::new();

        build.opt_level(3);

        if target.contains("i686-pc-windows-gnu") {
            build.flag("-fno-tree-vectorize");
        }

        build
            .file("./lz4/lib/lz4.c")
            .file("./lz4/lib/lz4frame.c")
            .file("./lz4/lib/lz4hc.c")
            .file("./lz4/lib/xxhash.c");

        build.compile("liblz4.a");
    }

    #[cfg(feature = "snappy")]
    fn build_snappy() {
        let target = env::var("TARGET").expect("No TARGET in environment");
        let mut build = cc::Build::new();

        build.include("./snappy/").include("./");

        build.define("NDEBUG", Some("1"));

        if target.contains("msvc") {
            build.flag("-EHsc");
        } else {
            build.flag("-std=c++11");
        }

        build
            .file("./snappy/snappy.cc")
            .file("./snappy/snappy-sinksource.cc")
            .file("snappy/snappy-c.cc");

        build.cpp(true);

        build.compile("libsnappy.a");
    }

    #[cfg(feature = "zlib")]
    fn build_zlib() {
        let mut build = cc::Build::new();

        build.opt_level(3);
        build.flag_if_supported("-Wno-implicit-function-declaration");

        let globs = &["./zlib/*.c"];
        globs
            .iter()
            .map(|pattern| glob::glob(pattern).unwrap())
            .flatten()
            .for_each(|path| build.file(path.unwrap()));

        build.compile("libz.a");
    }

    #[cfg(feature = "zstd")]
    fn build_zstd() {
        let mut build = cc::Build::new();

        build
            .include("./zstd/lib/")
            .include("./zstd/lib/common/")
            .include("./zstd/lib/legacy/");

        build.define("ZSTD_LIB_DEPRECATED", Some("0"));

        build.opt_level(3);

        let globs = &[
            "./zstd/lib/common/*.c",
            "./zstd/lib/compress/*.c",
            "./zstd/lib/decompress/*.c",
            "./zstd/lib/dictBuilder/*.c",
            "./zstd/lib/legacy/*.c",
        ];
        globs
            .iter()
            .map(|pattern| glob::glob(pattern).unwrap())
            .flatten()
            .for_each(|path| build.file(path.unwrap()));

        build.compile("libzstd.a");
    }

    pub fn vendor_dependencies() {
        let vendored_paths = &[
            "./rocksdb/",
            "./snappy/",
            "./lz4/",
            "./zstd/",
            "./zlib/",
            "./bzip2/",
        ];
        vendored_paths.iter().for_each(|p| {
            enforce_rerun(p);
            check_submodule(p);
        });

        super::bindgen_rocksdb();
    }
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
    #[cfg(feature = "vendored")]
    vendor::vendor_dependencies();

    // bindgen_rocksdb();

    // if !try_to_find_and_link_lib("ROCKSDB") {
    //     build_rocksdb();
    // }
    // if cfg!(feature = "snappy") && !try_to_find_and_link_lib("SNAPPY") {
    //     build_snappy();
    // }
    // if cfg!(feature = "lz4") && !try_to_find_and_link_lib("LZ4") {
    //     build_lz4();
    // }
    // if cfg!(feature = "zstd") && !try_to_find_and_link_lib("ZSTD") {
    //     build_zstd();
    // }
    // if cfg!(feature = "zlib") && !try_to_find_and_link_lib("ZLIB") {
    //     build_zlib();
    // }
    // if cfg!(feature = "bzip2") && !try_to_find_and_link_lib("BZIP2") {
    //     build_bzip2();
    // }
}
