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

use rocksdb::{
    BlockBasedOptions, Cache, DBCompressionType, DataBlockIndexType, Env, Options, ReadOptions, DB,
};
use util::DBPath;

#[test]
fn test_load_latest() {
    let n = DBPath::new("_rust_rocksdb_test_load_latest");
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        let _ = DB::open_cf(&opts, &n, vec!["cf0", "cf1"]).unwrap();
    }
    let (_, cfs) = Options::load_latest(
        &n,
        Env::new().unwrap(),
        true,
        Cache::new_lru_cache(1024 * 8),
    )
    .unwrap();
    assert!(cfs.iter().any(|cf| cf.name() == "default"));
    assert!(cfs.iter().any(|cf| cf.name() == "cf0"));
    assert!(cfs.iter().any(|cf| cf.name() == "cf1"));
}

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

#[test]
fn set_wal_compression_zstd() {
    let path = DBPath::new("_set_wal_compression_zstd");
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_wal_compression_type(DBCompressionType::None);
        opts.set_wal_compression_type(DBCompressionType::Zstd);
        let _db = DB::open(&opts, &path).unwrap();
    }
}

#[test]
#[should_panic(expected = "Lz4 is not supported for WAL compression")]
fn set_wal_compression_unsupported() {
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_wal_compression_type(DBCompressionType::Lz4);
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

#[test]
fn test_add_compact_on_deletion_collector_factory() {
    let n = DBPath::new("_rust_rocksdb_test_add_compact_on_deletion_collector_factory");

    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.add_compact_on_deletion_collector_factory(5, 10, 0.5);
    let _db = DB::open(&opts, &n).unwrap();

    let mut rocksdb_log = fs::File::open(format!("{}/LOG", (&n).as_ref().to_str().unwrap()))
        .expect("rocksdb creates a LOG file");
    let mut settings = String::new();
    rocksdb_log
        .read_to_string(&mut settings)
        .expect("can read the LOG file");
    assert!(settings.contains("CompactOnDeletionCollector (Sliding window size = 5 Deletion trigger = 10 Deletion ratio = 0.5)"));
}

#[test]
fn test_set_avoid_unnecessary_blocking_io() {
    let path = DBPath::new("_set_avoid_unnecessary_blocking_io");
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_avoid_unnecessary_blocking_io(true);
        let db = DB::open(&opts, &path).unwrap();
        let _ = db.put(b"k1", b"a");
        assert_eq!(&*db.get(b"k1").unwrap().unwrap(), b"a");
    }
}

#[test]
fn test_set_periodic_compaction_seconds() {
    let path = DBPath::new("_set_periodic_compaction_seconds");
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_periodic_compaction_seconds(5);
        let _db = DB::open(&opts, &path).unwrap();
    }
}

#[test]
fn test_set_ratelimiter() {
    let path = DBPath::new("_set_ratelimiter");
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_ratelimiter(1024000, 1000, 1);
        let db = DB::open(&opts, &path).unwrap();

        let _ = db.put(b"k1", b"a");
        assert_eq!(&*db.get(b"k1").unwrap().unwrap(), b"a");
    }

    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_auto_tuned_ratelimiter(1024000, 1000, 1);
        let db = DB::open(&opts, &path).unwrap();

        let _ = db.put(b"k2", b"a");
        assert_eq!(&*db.get(b"k2").unwrap().unwrap(), b"a");
    }
}
