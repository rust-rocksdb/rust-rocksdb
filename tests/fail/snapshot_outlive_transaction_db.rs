use rocksdb::{TransactionDB, SingleThreaded};

fn main() {
    let _snapshot = {
        let db = TransactionDB::<SingleThreaded>::open_default("foo").unwrap();
        db.snapshot()
    };
}