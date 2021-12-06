// Copyright 2020 Tyler Neely
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod util;

use std::{fs, io::Read as _};

use rocksdb::{BlockBasedOptions, DBCompressionType, DataBlockIndexType, Options, ReadOptions, DB};
use util::DBPath;

#[test]
fn test_set_num_levels() {
    let n = DBPath::new("_rust_rocksdb_test_set_num_levels");
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_num_levels(2);
        let _db = DB::open(&opts, &n).unwrap();
    }
}

#[test]
fn test_increase_parallelism() {
    let n = DBPath::new("_rust_rocksdb_test_increase_parallelism");
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.increase_parallelism(4);
        let _db = DB::open(&opts, &n).unwrap();
    }
}

#[test]
fn test_set_level_compaction_dynamic_level_bytes() {
    let n = DBPath::new("_rust_rocksdb_test_set_level_compaction_dynamic_level_bytes");
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_level_compaction_dynamic_level_bytes(true);
        let _db = DB::open(&opts, &n).unwrap();
    }
}

#[test]
fn test_block_based_options() {
    let path = "_rust_rocksdb_test_block_based_options";
    let n = DBPath::new(path);
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);

        let mut block_opts = BlockBasedOptions::default();
        block_opts.set_cache_index_and_filter_blocks(true);
        block_opts.set_pin_l0_filter_and_index_blocks_in_cache(true);
        block_opts.set_format_version(4);
        block_opts.set_index_block_restart_interval(16);

        opts.set_block_based_table_factory(&block_opts);
        let _db = DB::open(&opts, &n).unwrap();

        // read the setting from the LOG file
        let mut rocksdb_log = fs::File::open(format!("{}/LOG", (&n).as_ref().to_str().unwrap()))
            .expect("rocksdb creates a LOG file");
        let mut settings = String::new();
        rocksdb_log.read_to_string(&mut settings).unwrap();

        // check the settings are set in the LOG file
        assert!(settings.contains("cache_index_and_filter_blocks: 1"));
        assert!(settings.contains("pin_l0_filter_and_index_blocks_in_cache: 1"));
        assert!(settings.contains("format_version: 4"));
        assert!(settings.contains("index_block_restart_interval: 16"));
    }
}

#[test]
fn test_read_options() {
    let mut read_opts = ReadOptions::default();
    read_opts.set_verify_checksums(false);
}

#[test]
fn test_set_data_block_index_type() {
    let path = "_rust_rocksdb_test_set_data_block_index_type";
    let n = DBPath::new(path);

    // Default is `BinarySearch`
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);

        let block_opts = BlockBasedOptions::default();
        opts.set_block_based_table_factory(&block_opts);
        let _db = DB::open(&opts, &n).expect("open a db works");

        let mut rocksdb_log = fs::File::open(format!("{}/LOG", (&n).as_ref().to_str().unwrap()))
            .expect("rocksdb creates a LOG file");
        let mut settings = String::new();
        rocksdb_log
            .read_to_string(&mut settings)
            .expect("can read the LOG file");
        assert!(settings.contains("data_block_index_type: 0"));
        assert!(settings.contains("data_block_hash_table_util_ratio: 0.750000"));
    }

    // Setting the index type and hash table utilization ratio works
    {
        let mut opts = Options::default();
        opts.create_if_missing(false);

        let mut block_opts = BlockBasedOptions::default();
        block_opts.set_data_block_index_type(DataBlockIndexType::BinaryAndHash);
        block_opts.set_data_block_hash_ratio(0.35);
        opts.set_block_based_table_factory(&block_opts);
        let _db = DB::open(&opts, &n).expect("open a db works");

        let mut rocksdb_log = fs::File::open(format!("{}/LOG", (&n).as_ref().to_str().unwrap()))
            .expect("rocksdb creates a LOG file");
        let mut settings = String::new();
        rocksdb_log
            .read_to_string(&mut settings)
            .expect("can read the LOG file");
        assert!(settings.contains("data_block_index_type: 1"));
        assert!(settings.contains("data_block_hash_table_util_ratio: 0.350000"));
    }
}

#[test]
#[cfg(feature = "zstd")]
fn set_compression_options_zstd_max_train_bytes() {
    let path = DBPath::new("_rust_set_compression_options_zstd_max_train_bytes");
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_compression_options(4, 5, 6, 7);
        opts.set_zstd_max_train_bytes(100);
        let _db = DB::open(&opts, &path).unwrap();
    }
}

fn test_compression_type(ty: DBCompressionType) {
    let path = DBPath::new("_test_compression_type");

    let mut opts = Options::default();
    opts.set_compression_type(ty);
    opts.create_if_missing(true);
    let db = DB::open(&opts, &path);

    let should_open = match ty {
        DBCompressionType::None => true,
        DBCompressionType::Snappy => cfg!(feature = "snappy"),
        DBCompressionType::Zlib => cfg!(feature = "zlib"),
        DBCompressionType::Bz2 => cfg!(feature = "bzip2"),
        DBCompressionType::Lz4 | DBCompressionType::Lz4hc => cfg!(feature = "lz4"),
        DBCompressionType::Zstd => cfg!(feature = "zstd"),
    };

    if should_open {
        let _db = db.unwrap();
    } else {
        let _err = db.unwrap_err();
    }
}

#[test]
fn test_none_compression() {
    test_compression_type(DBCompressionType::None);
}

#[test]
fn test_snappy_compression() {
    test_compression_type(DBCompressionType::Snappy);
}

#[test]
fn test_zlib_compression() {
    test_compression_type(DBCompressionType::Zlib);
}

#[test]
fn test_bz2_compression() {
    test_compression_type(DBCompressionType::Bz2);
}

#[test]
fn test_lz4_compression() {
    test_compression_type(DBCompressionType::Lz4);
    test_compression_type(DBCompressionType::Lz4hc);
}

#[test]
fn test_zstd_compression() {
    test_compression_type(DBCompressionType::Zstd);
}
