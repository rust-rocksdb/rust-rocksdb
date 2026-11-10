use rocksdb::{ReadOptions, DB};

fn main() {
    let db = DB::open_default("foo").unwrap();
    let _iter = {
        let snapshot = db.snapshot();
        let mut readopts = ReadOptions::default();
        readopts.set_snapshot(&snapshot);
        db.raw_iterator_opt(readopts)
    };
}
