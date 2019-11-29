// Copyright 2018 PingCAP, Inc.
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

use rocksdb::{ColumnFamilyOptions, DBCompressionType, DBOptions, DB};

use super::tempdir_with_prefix;

#[test]
// Make sure all compression types are supported.
fn test_compression() {
    let path = tempdir_with_prefix("_rust_rocksdb_test_metadata");
    let compression_types = [
        DBCompressionType::Snappy,
        DBCompressionType::Zlib,
        DBCompressionType::Bz2,
        DBCompressionType::Lz4,
        DBCompressionType::Lz4hc,
        DBCompressionType::Zstd,
    ];
    for compression_type in compression_types.iter() {
        let mut opts = DBOptions::new();
        opts.create_if_missing(true);
        let mut cf_opts = ColumnFamilyOptions::new();
        cf_opts.compression(*compression_type);
        // DB open will fail if compression type is not supported.
        DB::open_cf(
            opts,
            path.path().to_str().unwrap(),
            vec![("default", cf_opts)],
        )
        .unwrap();
    }
}
