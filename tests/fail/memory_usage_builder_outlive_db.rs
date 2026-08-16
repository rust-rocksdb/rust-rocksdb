use rocksdb::{perf::MemoryUsageBuilder, DB};

fn main() {
    let mut builder = MemoryUsageBuilder::new().unwrap();
    {
        let db = DB::open_default("foo").unwrap();
        builder.add_db(&db);
    }
    let _memory_usage = builder.build().unwrap();
}
