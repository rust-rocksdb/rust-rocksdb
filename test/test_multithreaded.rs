use rocksdb::{Options, DB, Writable};
use std::thread;
use std::sync::Arc;

const N: usize = 100_000;

#[test]
pub fn test_multithreaded() {
    let path = "_rust_rocksdb_multithreadtest";
    {
        let db = DB::open_default(path).unwrap();
        let db = Arc::new(db);

        db.put(b"key", b"value1").unwrap();

        let db1 = db.clone();
        let j1 = thread::spawn(move|| {
            for _ in 1..N {
                db1.put(b"key", b"value1").unwrap();
            }
        });

        let db2 = db.clone();
        let j2 = thread::spawn(move|| {
            for _ in 1..N {
                db2.put(b"key", b"value2").unwrap();
            }
        });

        let db3 = db.clone();
        let j3 = thread::spawn(move|| {
            for _ in 1..N {
                match db3.get(b"key") {
                    Ok(Some(v)) => {
                        if &v[..] != b"value1" && &v[..] != b"value2" {
                            assert!(false);
                        }
                    }
                    _ => {
                        assert!(false);
                    }
                }
            }
        });

        j1.join().unwrap();
        j2.join().unwrap();
        j3.join().unwrap();
    }
    assert!(DB::destroy(&Options::new(), path).is_ok());
}
