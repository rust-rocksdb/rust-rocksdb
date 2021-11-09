use rocksdb::TxnDB;

fn main() {
    let _snapshot = {
        let db = TxnDB::open_default("foo").unwrap();
        db.snapshot()
    };
}