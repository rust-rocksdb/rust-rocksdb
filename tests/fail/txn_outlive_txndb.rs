use rocksdb::TxnDB;

fn main() {
    let _txn = {
        let db = TxnDB::open_default("foo").unwrap();
        db.txn()
    };
}