extern crate rocksdb;
extern crate test;
use rocksdb::open;
use test::Bencher;

#[allow(dead_code)]
fn main() {
    let db = open("testdb".to_string(), true).unwrap();
    assert!(db.put(b"hey", b"v1111").is_ok());
    db.get(b"hey").map(|raw| { std::str::from_utf8(raw.as_slice()).map(|v| {println!("value: {}", v); })});
    db.close();
}

#[allow(dead_code)]
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

#[allow(dead_code)]
#[bench]
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
