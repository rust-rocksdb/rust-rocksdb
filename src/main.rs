extern crate rocksdb;
extern crate test;
use rocksdb::open;
use test::Bencher;

fn main() {
    let db = open("testdb".to_string(), true).unwrap();
    db.put(b"hey", b"v1111");
    db.get(b"hey").map(|v| { println!("value: {}", v.as_slice()); });
    db.close();
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

#[bbench]
fn reads(b: &mut Bencher) {
    let db = open("testdb".to_string(), true).unwrap();
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
