use rocksdb::{TransactionDB, SingleThreaded};

fn main() {
    let _txn = {
        let db = TransactionDB::<SingleThreaded>::open_default("foo").unwrap();
        db.transaction()
    };
}