use rocksdb::{ReadOptions, DB};

fn main() {
    let db = DB::open_default("foo").unwrap();
    let mut readopts = ReadOptions::default();
    {
        let snapshot = db.snapshot();
        readopts.set_snapshot(&snapshot);
    }
    let _ = db.get_opt(b"key", &readopts);
}
