use rocksdb::TransactionDB;

fn main() {
    let _txn = {
        let db = TransactionDB::open_default("foo").unwrap();
        db.transaction()
    };
}