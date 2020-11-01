use rocksdb::prelude::*;
use rocksdb::checkpoint::Checkpoint;

fn main() {
    let _checkpoint = {
        let db = DB::open_default("foo").unwrap();
        Checkpoint::new(&db)
    };
}
