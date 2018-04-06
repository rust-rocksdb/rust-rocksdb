extern crate rocksdb;

use std::time::{SystemTime, UNIX_EPOCH};

use rocksdb::{DB, Options};

// Ensures that DB::Destroy is called for this database when DBName is dropped.
pub struct DBName {
    pub name: String,
}

impl DBName {
    // Suffixes the given `prefix` with a timestamp to ensure that subsequent test runs don't reuse
    // an old database in case of panics prior to Drop being called.
    pub fn new(prefix: &str) -> DBName {
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let name = format!(
            "{}.{}.{}",
            prefix,
            current_time.as_secs(),
            current_time.subsec_nanos()
        );

        DBName { name }
    }
}

impl Drop for DBName {
    fn drop(&mut self) {
        let opts = Options::default();
        DB::destroy(&opts, &self.name).unwrap();
    }
}

