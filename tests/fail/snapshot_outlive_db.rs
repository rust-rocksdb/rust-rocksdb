use rocksdb::prelude::*;

fn main() {
    let _snapshot = {
        let db = DB::open_default("foo").unwrap();
        db.snapshot()
    };
}
