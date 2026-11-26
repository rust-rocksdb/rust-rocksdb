use rocksdb::{IteratorMode, DB};

fn main() {
    let db = DB::open_default("foo").unwrap();
    let _iter = {
        let snapshot = db.snapshot();
        snapshot.iterator(IteratorMode::Start)
    };
}
