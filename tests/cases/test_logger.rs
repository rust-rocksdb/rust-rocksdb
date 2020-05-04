use std::sync::{atomic::*, Arc};
use std::time::Duration;
use std::{str, thread};

use super::tempdir_with_prefix;

use rocksdb::{DBInfoLogLevel as InfoLogLevel, DBOptions, Logger, DB};

#[derive(Default, Clone)]
struct TestDrop {
    called: Arc<AtomicUsize>,
}

impl Drop for TestDrop {
    fn drop(&mut self) {
        self.called.fetch_add(1, Ordering::SeqCst);
    }
}

#[derive(Default, Clone)]
struct TestLogger {
    print: Arc<AtomicUsize>,
    drop: Option<TestDrop>,
}

impl Logger for TestLogger {
    fn logv(&self, _log_level: InfoLogLevel, log: &str) {
        assert!(log.len() > 0);
        self.print.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn test_logger() {
    let drop_called = Arc::new(AtomicUsize::new(0));
    let path = tempdir_with_prefix("_rust_rocksdb_test_create_info_rust_log_opt");
    let mut opts = DBOptions::new();
    let logger = TestLogger {
        drop: Some(TestDrop {
            called: drop_called.clone(),
        }),
        print: Default::default(),
    };
    opts.set_info_log(logger.clone());
    opts.create_if_missing(true);
    opts.set_info_log_level(InfoLogLevel::Debug);
    let db = DB::open(opts.clone(), path.path().to_str().unwrap()).unwrap();
    thread::sleep(Duration::from_secs(2));
    assert_ne!(logger.print.load(Ordering::SeqCst), 0);
    drop(db);
    drop(opts);
    assert_eq!(0, drop_called.load(Ordering::SeqCst));
    drop(logger);
    assert_eq!(1, drop_called.load(Ordering::SeqCst));
}
