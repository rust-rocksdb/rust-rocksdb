use rust_rocksdb::{checkpoint::Checkpoint, DB};

fn main() {
    let _checkpoint = {
        let db = DB::open_default("foo").unwrap();
        Checkpoint::new(&db)
    };
}
