use rocksdb::TransactionDB;

fn main() {
    let db = TransactionDB::open_default("foo").unwrap();
    let _snapshot = {
        let txn = db.transaction();
        txn.snapshot()
    };
}