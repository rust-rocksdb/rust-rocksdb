use rocksdb::{TransactionDB, SingleThreaded};

fn main() {
    let db = TransactionDB::<SingleThreaded>::open_default("foo").unwrap();
    let _snapshot = {
        let txn = db.transaction();
        txn.snapshot()
    };
}