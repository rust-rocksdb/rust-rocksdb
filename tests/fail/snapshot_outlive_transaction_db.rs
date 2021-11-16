use rocksdb::TransactionDB;

fn main() {
    let _snapshot = {
        let db = TransactionDB::open_default("foo").unwrap();
        db.snapshot()
    };
}