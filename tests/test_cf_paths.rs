mod util;

use rocksdb::{ColumnFamilyDescriptor, DBPath as RocksDbPath, Options, DB};
use std::fs;
use std::path::Path;
use util::DBPath;

fn count_sst(dir: &Path) -> usize {
    fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |x| x == "sst"))
        .count()
}

// A column family with `set_cf_paths` set to a separate directory writes its SST files there,
// not into the main DB directory, and the data still reads back.
#[test]
fn cf_paths_places_sst_on_separate_dir() {
    let db_path = DBPath::new("_rust_rocksdb_cf_paths_test");
    let cf_dir = tempfile::Builder::new()
        .prefix("cf_tx")
        .tempdir()
        .expect("temp dir for the redirected column family");

    {
        let mut db_opts = Options::default();
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);

        // "default" column family: no override, stays in the DB directory.
        let default_opts = Options::default();

        // "col_tx": pin its SST files to a separate directory via a single large-target path.
        let mut tx_opts = Options::default();
        tx_opts.set_cf_paths(&[RocksDbPath::new(cf_dir.path(), u64::MAX).unwrap()]);

        let cfs = vec![
            ColumnFamilyDescriptor::new("default", default_opts),
            ColumnFamilyDescriptor::new("col_tx", tx_opts),
        ];
        let db = DB::open_cf_descriptors(&db_opts, &db_path, cfs).unwrap();
        let tx = db.cf_handle("col_tx").unwrap();

        for i in 0..4000u32 {
            db.put_cf(&tx, i.to_be_bytes(), vec![i as u8; 512]).unwrap();
        }
        db.flush_cf(&tx).unwrap();

        assert_eq!(
            db.get_cf(&tx, 42u32.to_be_bytes()).unwrap().unwrap(),
            vec![42u8; 512],
            "redirected column family reads back its data",
        );
    }

    assert!(
        count_sst(cf_dir.path()) > 0,
        "column family SST files should be in the cf_paths override dir",
    );
    assert_eq!(
        count_sst((&db_path).as_ref()),
        0,
        "no col_tx SST should be in the main DB dir (default CF got no writes)",
    );
}
