extern crate rocksdb;
mod util;

use rocksdb::{OptimistictransactionDB,CreateIter};
use util::DBPath;

#[test]
pub fn test_optimistictransaction() {
    let n = DBPath::new("optimistictransaction");
    {
        let db = OptimistictransactionDB::open_default(&n).unwrap();

        let trans = db.transaction_default();
        
        trans.put(b"k1", b"v1").unwrap();
        trans.put(b"k2", b"v2").unwrap();
        trans.put(b"k3", b"v3").unwrap();
        trans.put(b"k4", b"v4").unwrap();

        let trans_result = trans.commit();

        assert_eq!(trans_result.is_ok(), true);

        let trans2 = db.transaction_default();

        let mut iter = trans2.raw_iterator();

        iter.seek_to_first();

        assert_eq!(iter.valid(), true);
        assert_eq!(iter.key(), Some(b"k1".to_vec()));
        assert_eq!(iter.value(), Some(b"v1".to_vec()));

        iter.next();

        assert_eq!(iter.valid(), true);
        assert_eq!(iter.key(), Some(b"k2".to_vec()));
        assert_eq!(iter.value(), Some(b"v2".to_vec()));

        iter.next(); // k3
        iter.next(); // k4
        iter.next(); // invalid!

        assert_eq!(iter.valid(), false);
        assert_eq!(iter.key(), None);
        assert_eq!(iter.value(), None);

        let trans3 = db.transaction_default();

        trans2.put(b"k2", b"v5").unwrap();
        trans3.put(b"k2", b"v6").unwrap();

        let trans3_result = trans3.commit();

        assert_eq!(trans3_result.is_ok(),true);

        let trans2_result = trans2.commit();

        assert_eq!(trans2_result.is_err(),true);
    }
}