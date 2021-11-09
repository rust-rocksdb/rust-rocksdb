use rocksdb::TxnDB;

fn main() {
    let db = TxnDB::open_default("foo").unwrap();
    let _snapshot = {
        let txn = db.txn();
        txn.snapshot()
    };
}