use std::ffi::CStr;

use libc::{self, c_char, c_void};

use crate::{
    compaction_filter::{self, CompactionFilter},
    ffi,
};

/// Each compaction will create a new CompactionFilter allowing the
/// application to know about different compactions.
///
/// RocksDB may call [`create`] concurrently from multiple background threads.
/// Use atomics or a mutex for mutable factory state.
/// Each returned filter is used by a single thread.
///
/// See [compaction_filter::CompactionFilter][CompactionFilter] and
/// [Options::set_compaction_filter_factory][set_compaction_filter_factory]
/// for more details
///
/// [`create`]: CompactionFilterFactory::create
/// [CompactionFilter]: ../compaction_filter/trait.CompactionFilter.html
/// [set_compaction_filter_factory]: ../struct.Options.html#method.set_compaction_filter_factory
pub trait CompactionFilterFactory: Send + Sync {
    type Filter: CompactionFilter;

    /// Returns a CompactionFilter for the compaction process.
    ///
    /// This may be called concurrently from multiple threads.
    fn create(&self, context: CompactionFilterContext) -> Self::Filter;

    /// Returns a name that identifies this compaction filter factory.
    fn name(&self) -> &CStr;
}

pub unsafe extern "C" fn destructor_callback<F>(raw_self: *mut c_void)
where
    F: CompactionFilterFactory,
{
    drop(unsafe { Box::from_raw(raw_self as *mut F) });
}

pub unsafe extern "C" fn name_callback<F>(raw_self: *mut c_void) -> *const c_char
where
    F: CompactionFilterFactory,
{
    let self_ = unsafe { &*(raw_self.cast_const() as *const F) };
    self_.name().as_ptr()
}

/// Context information of a compaction run
pub struct CompactionFilterContext {
    /// Does this compaction run include all data files
    pub is_full_compaction: bool,
    /// Is this compaction requested by the client (true),
    /// or is it occurring as an automatic compaction process
    pub is_manual_compaction: bool,
}

impl CompactionFilterContext {
    unsafe fn from_raw(ptr: *mut ffi::rocksdb_compactionfiltercontext_t) -> Self {
        let is_full_compaction =
            unsafe { ffi::rocksdb_compactionfiltercontext_is_full_compaction(ptr) } != 0;
        let is_manual_compaction =
            unsafe { ffi::rocksdb_compactionfiltercontext_is_manual_compaction(ptr) } != 0;

        Self {
            is_full_compaction,
            is_manual_compaction,
        }
    }
}

pub unsafe extern "C" fn create_compaction_filter_callback<F>(
    raw_self: *mut c_void,
    context: *mut ffi::rocksdb_compactionfiltercontext_t,
) -> *mut ffi::rocksdb_compactionfilter_t
where
    F: CompactionFilterFactory,
{
    // SAFETY: `raw_self` comes from `Box::into_raw(Box<F>)` and remains valid
    // until RocksDB destroys the native factory after all callbacks finish.
    // Callbacks may overlap, so construct a shared reference; `F: Sync` allows this.
    let self_ = unsafe { &*(raw_self.cast_const() as *const F) };
    let context = unsafe { CompactionFilterContext::from_raw(context) };
    let filter = Box::new(self_.create(context));

    let filter_ptr = Box::into_raw(filter);

    unsafe {
        ffi::rocksdb_compactionfilter_create(
            filter_ptr as *mut c_void,
            Some(compaction_filter::destructor_callback::<F::Filter>),
            Some(compaction_filter::filter_callback::<F::Filter>),
            Some(compaction_filter::name_callback::<F::Filter>),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compaction_filter::Decision;
    use crate::{Options, DB};
    use std::ffi::CString;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier};

    struct CountFilter(u16, CString);
    impl CompactionFilter for CountFilter {
        fn filter(&mut self, _level: u32, _key: &[u8], _value: &[u8]) -> crate::CompactionDecision {
            self.0 += 1;
            if self.0 > 2 {
                Decision::Remove
            } else {
                Decision::Keep
            }
        }

        fn name(&self) -> &CStr {
            &self.1
        }
    }

    struct TestFactory(CString);
    impl CompactionFilterFactory for TestFactory {
        type Filter = CountFilter;

        fn create(&self, _context: CompactionFilterContext) -> Self::Filter {
            CountFilter(0, CString::new("CountFilter").unwrap())
        }

        fn name(&self) -> &CStr {
            &self.0
        }
    }

    #[test]
    fn compaction_filter_factory_test() {
        let tempdir = tempfile::Builder::new()
            .prefix("_rust_rocksdb_filter_factory_test")
            .tempdir()
            .expect("Failed to create temporary path for the _rust_rocksdb_filter_factory_test.");
        let path = tempdir.path();
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_compaction_filter_factory(TestFactory(CString::new("TestFactory").unwrap()));
        {
            let db = DB::open(&opts, path).unwrap();
            let _r = db.put(b"k1", b"a");
            let _r = db.put(b"_rk", b"b");
            let _r = db.put(b"%k", b"c");
            db.compact_range(None::<&[u8]>, None::<&[u8]>);
            assert_eq!(db.get(b"%k1").unwrap(), None);
        }
        let result = DB::destroy(&opts, path);
        assert!(result.is_ok());
    }

    struct CountingFactory {
        created: Arc<AtomicUsize>,
        barrier: Barrier,
    }

    impl CompactionFilterFactory for CountingFactory {
        type Filter = CountFilter;

        fn create(&self, context: CompactionFilterContext) -> Self::Filter {
            assert!(context.is_manual_compaction);
            self.created.fetch_add(1, Ordering::Relaxed);
            // Keep both callbacks active at once.
            self.barrier.wait();
            CountFilter(0, CString::new("CountFilter").unwrap())
        }

        fn name(&self) -> &CStr {
            c"CountingFactory"
        }
    }

    #[test]
    fn compaction_filter_factory_callbacks_are_concurrent() {
        let tempdir = tempfile::tempdir().unwrap();
        let created = Arc::new(AtomicUsize::new(0));
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_disable_auto_compactions(true);
        opts.set_max_background_jobs(4);
        opts.set_compaction_filter_factory(CountingFactory {
            created: Arc::clone(&created),
            barrier: Barrier::new(2),
        });
        let a = DB::open(&opts, tempdir.path().join("a")).unwrap();
        let b = DB::open(&opts, tempdir.path().join("b")).unwrap();
        for db in [&a, &b] {
            for key in [b"k1", b"k2", b"k3"] {
                db.put(key, b"v").unwrap();
            }
            db.flush().unwrap();
        }

        std::thread::scope(|scope| {
            for db in [&a, &b] {
                scope.spawn(move || db.compact_range(None::<&[u8]>, None::<&[u8]>));
            }
        });

        assert_eq!(created.load(Ordering::Relaxed), 2);
        // Each filter keeps its first two keys, demonstrating independent state.
        for db in [&a, &b] {
            assert_eq!(db.get(b"k1").unwrap().as_deref(), Some(b"v".as_slice()));
            assert_eq!(db.get(b"k2").unwrap().as_deref(), Some(b"v".as_slice()));
            assert_eq!(db.get(b"k3").unwrap(), None);
        }
    }
}
