use rocksdb::{IteratorMode, DB};

fn main() {
    let _iter = {
        let db = DB::open_default("foo").unwrap();
        db.iterator(IteratorMode::Start)
    };
}
