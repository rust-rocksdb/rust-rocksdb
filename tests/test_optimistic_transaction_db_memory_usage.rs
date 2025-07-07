// Copyright 2021 Yiyuan Liu
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
//

mod util;

use rocksdb::{OptimisticTransactionDB, Options, SingleThreaded};
use util::DBPath;

#[test]
fn test_optimistic_transaction_db_memory_usage() {
    let path = DBPath::new("_rust_rocksdb_optimistic_transaction_db memory_usage_test");
    {
        let mut options = Options::default();
        options.create_if_missing(true);
        options.enable_statistics();

        // setup cache:
        let cache = rocksdb::Cache::new_lru_cache(1 << 20); // 1 MB cache
        let mut block_based_options = rocksdb::BlockBasedOptions::default();
        block_based_options.set_block_cache(&cache);
        options.set_block_based_table_factory(&block_based_options);

        let db: OptimisticTransactionDB<SingleThreaded> =
            OptimisticTransactionDB::open(&options, &path).unwrap();
        let mut builder = rocksdb::perf::MemoryUsageBuilder::new().unwrap();
        builder.add_db(&db);
        builder.add_cache(&cache);
        let memory_usage = builder.build().unwrap();

        for i in 1..=1000 {
            let key = format!("key{i}");
            let value = format!("value{i}");
            db.put(&key, &value).unwrap();
        }

        for i in 1..=1000 {
            let key = format!("key{i}");
            let result = db.get(&key).unwrap().unwrap();
            let result_str = String::from_utf8(result).unwrap();
            assert_eq!(result_str, format!("value{i}"));
        }

        assert_ne!(memory_usage.approximate_mem_table_total(), 0);
        assert_eq!(memory_usage.approximate_mem_table_readers_total(), 0); // Equals zero!
        assert_ne!(memory_usage.approximate_cache_total(), 0);
        assert_ne!(memory_usage.approximate_mem_table_unflushed(), 0);
    }
}
