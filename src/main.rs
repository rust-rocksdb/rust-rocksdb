extern crate rocksdb;
extern crate test;
use rocksdb::open;
use test::Bencher;

fn main() {
    println!("test");
}

#[bench]
fn writes(b: &mut Bencher) {
    let db = open("testdb".to_string(), true).unwrap();
    let mut i = 0 as u64;
    b.iter(|| {
        db.put(i.to_string().as_bytes(), b"v1111");
        i += 1;
    });
    db.close();
}

#[bench]
fn reads(b: &mut Bencher) {
    let db = open("testdb".to_string(), true).unwrap();
    let mut i = 0 as u64;
    b.iter(|| {
        db.get(i.to_string().as_bytes());
        i += 1;
    });
    db.close();
}
