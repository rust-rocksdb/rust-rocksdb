extern crate rocksdb;
extern crate test;
use rocksdb::RocksDB;
use test::Bencher;

#[allow(dead_code)]
fn main() {
  match RocksDB::open_default("/tmp/rust-rocksdb") {
    Ok(db) => {
      assert!(db.put(b"my key", b"my value").is_ok());

      db.get(b"my key").map( |value| {
        match value.to_utf8() {
          Some(v) =>
            println!("retrieved utf8 value: {}", v),
          None =>
            println!("did not read valid utf-8 out of the db"),
        }
      })
        .on_absent( || { println!("value not found") })
        .on_error( |e| { println!("error retrieving value: {}", e) });

      assert!(db.delete(b"my key").is_ok());

      db.close();
    },
    Err(e) => panic!(e),
  }
}

#[allow(dead_code)]
#[bench]
fn writes(b: &mut Bencher) {
  let db = RocksDB::open_default("testdb").unwrap();
  let mut i = 0 as u64;
  b.iter(|| {
    db.put(i.to_string().as_bytes(), b"v1111");
    i += 1;
  });
  db.close();
}

#[allow(dead_code)]
#[bench]
fn reads(b: &mut Bencher) {
  let db = RocksDB::open_default("testdb").unwrap();
  let mut i = 0 as u64;
  b.iter(|| {
    db.get(i.to_string().as_bytes()).on_error(
      |e| {
        println!("error: {}", e);
        e
      });
    i += 1;
  });
  db.close();
}
