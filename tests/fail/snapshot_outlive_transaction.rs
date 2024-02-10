use rust_rocksdb::{SingleThreaded, TransactionDB};

fn main() {
    let db = TransactionDB::<SingleThreaded>::open_default("foo").unwrap();
    let _snapshot = {
        let txn = db.transaction();
        txn.snapshot()
    };
}
