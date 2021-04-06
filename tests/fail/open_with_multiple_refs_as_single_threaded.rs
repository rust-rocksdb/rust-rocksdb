use rocksdb::{SingleThreaded, DBWithThreadMode, Options};

fn main() {
    let db = DBWithThreadMode::<SingleThreaded>::open_default("/path/to/dummy").unwrap();
    let db_ref1 = &db;
    let db_ref2 = &db;
    let opts = Options::default();
    db_ref1.create_cf("cf1", &opts).unwrap();
    db_ref2.create_cf("cf2", &opts).unwrap();
}
