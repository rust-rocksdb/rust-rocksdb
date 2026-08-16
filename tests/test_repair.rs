mod util;

use std::cmp::Ordering;
use std::fs;
use std::path::Path;

use rocksdb::{ColumnFamilyDescriptor, IteratorMode, Options, DB, DEFAULT_COLUMN_FAMILY_NAME};
use util::DBPath;

fn reverse_cf_options() -> Options {
    let mut opts = Options::default();
    opts.set_comparator(
        "test_repair.reverse",
        Box::new(|left: &[u8], right: &[u8]| match left.cmp(right) {
            Ordering::Less => Ordering::Greater,
            Ordering::Equal => Ordering::Equal,
            Ordering::Greater => Ordering::Less,
        }),
    );
    opts
}

fn repair_descriptors() -> Vec<ColumnFamilyDescriptor> {
    vec![
        ColumnFamilyDescriptor::new(DEFAULT_COLUMN_FAMILY_NAME, Options::default()),
        ColumnFamilyDescriptor::new("reverse", reverse_cf_options()),
    ]
}

fn remove_manifest_files(path: &Path) {
    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name.to_string_lossy().starts_with("MANIFEST-") {
            fs::remove_file(entry.path()).unwrap();
        }
    }
}

#[test]
fn repair_cf_descriptors_preserves_per_cf_options() {
    let path = DBPath::new("_rust_rocksdb_repair_cf_descriptors");

    let mut db_opts = Options::default();
    db_opts.create_if_missing(true);
    db_opts.create_missing_column_families(true);

    {
        let db = DB::open_cf_descriptors(&db_opts, &path, repair_descriptors()).unwrap();
        let reverse_cf = db.cf_handle("reverse").unwrap();

        db.put(b"default-flushed", b"v0").unwrap();
        db.flush().unwrap();
        db.put(b"default-wal", b"v1").unwrap();

        db.put_cf(&reverse_cf, b"a", b"va").unwrap();
        db.flush_cf(&reverse_cf).unwrap();
        db.put_cf(&reverse_cf, b"b", b"vb").unwrap();
    }

    remove_manifest_files((&path).as_ref());

    DB::repair_cf_descriptors(&db_opts, &path, repair_descriptors()).unwrap();

    {
        let db = DB::open_cf_descriptors(&db_opts, &path, repair_descriptors()).unwrap();
        let reverse_cf = db.cf_handle("reverse").unwrap();

        assert_eq!(db.get(b"default-flushed").unwrap().unwrap(), b"v0");
        assert_eq!(db.get(b"default-wal").unwrap().unwrap(), b"v1");
        assert_eq!(db.get_cf(&reverse_cf, b"a").unwrap().unwrap(), b"va");
        assert_eq!(db.get_cf(&reverse_cf, b"b").unwrap().unwrap(), b"vb");

        let repaired_keys = db
            .iterator_cf(&reverse_cf, IteratorMode::Start)
            .map(|item| item.unwrap().0)
            .collect::<Vec<_>>();
        assert_eq!(
            repaired_keys,
            vec![
                b"b".to_vec().into_boxed_slice(),
                b"a".to_vec().into_boxed_slice()
            ]
        );
    }
}

#[test]
fn repair_cf_descriptors_requires_default_descriptor() {
    let path = DBPath::new("_rust_rocksdb_repair_cf_requires_default");

    let mut db_opts = Options::default();
    db_opts.create_if_missing(true);
    db_opts.create_missing_column_families(true);

    {
        let db = DB::open_cf_descriptors(&db_opts, &path, repair_descriptors()).unwrap();
        let reverse_cf = db.cf_handle("reverse").unwrap();
        db.put_cf(&reverse_cf, b"a", b"va").unwrap();
        db.flush_cf(&reverse_cf).unwrap();
    }

    remove_manifest_files((&path).as_ref());

    let err = DB::repair_cf_descriptors(
        &db_opts,
        &path,
        [ColumnFamilyDescriptor::new("reverse", reverse_cf_options())],
    )
    .unwrap_err();

    assert!(
        err.to_string()
            .contains("Must contain entry for default column family"),
        "unexpected error: {err}"
    );
}
