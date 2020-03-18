use std::ffi::CString;
use std::{mem, ptr, slice};

pub use crocksdb_ffi::DBCompactionFilter;
use crocksdb_ffi::{self, DBCompactionFilterContext, DBCompactionFilterFactory};
use libc::{c_char, c_int, c_void, malloc, memcpy, size_t};

/// `CompactionFilter` allows an application to modify/delete a key-value at
/// the time of compaction.
/// For more details, Please checkout rocksdb's documentation.
pub trait CompactionFilter {
    /// The compaction process invokes this
    /// method for kv that is being compacted. A return value
    /// of false indicates that the kv should be preserved in the
    /// output of this compaction run and a return value of true
    /// indicates that this key-value should be removed from the
    /// output of the compaction.  The application can inspect
    /// the existing value of the key and make decision based on it.
    fn filter(
        &mut self,
        level: usize,
        key: &[u8],
        value: &[u8],
        new_value: &mut Vec<u8>,
        value_changed: &mut bool,
    ) -> bool;
}

#[repr(C)]
struct CompactionFilterProxy {
    name: CString,
    filter: Box<dyn CompactionFilter>,
}

extern "C" fn name(filter: *mut c_void) -> *const c_char {
    unsafe { (*(filter as *mut CompactionFilterProxy)).name.as_ptr() }
}

extern "C" fn destructor(filter: *mut c_void) {
    unsafe {
        Box::from_raw(filter as *mut CompactionFilterProxy);
    }
}

extern "C" fn filter(
    filter: *mut c_void,
    level: c_int,
    key: *const u8,
    key_len: size_t,
    value: *const u8,
    value_len: size_t,
    new_value: *mut *mut u8,
    new_value_len: *mut size_t,
    value_changed: *mut bool,
) -> bool {
    unsafe {
        *new_value = ptr::null_mut();
        *new_value_len = 0;
        *value_changed = false;

        let filter = &mut *(filter as *mut CompactionFilterProxy);
        let key = slice::from_raw_parts(key, key_len);
        let value = slice::from_raw_parts(value, value_len);
        let mut new_value_v = Vec::default();
        let filtered = filter.filter.filter(
            level as usize,
            key,
            value,
            &mut new_value_v,
            mem::transmute(&mut *value_changed),
        );
        if *value_changed {
            *new_value_len = new_value_v.len();
            // The vector is allocated in Rust, so dup it before pass into C.
            *new_value = malloc(*new_value_len) as *mut u8;
            memcpy(
                new_value as *mut c_void,
                &new_value_v[0] as *const u8 as *const c_void,
                *new_value_len,
            );
        }
        filtered
    }
}

pub struct CompactionFilterHandle {
    pub inner: *mut DBCompactionFilter,
}

impl Drop for CompactionFilterHandle {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_compactionfilter_destroy(self.inner);
        }
    }
}

pub unsafe fn new_compaction_filter(
    c_name: CString,
    f: Box<dyn CompactionFilter>,
) -> CompactionFilterHandle {
    let filter = new_compaction_filter_raw(c_name, f);
    CompactionFilterHandle { inner: filter }
}

/// Just like `new_compaction_filter`, but returns a raw pointer instead of a RAII struct.
/// Generally used in `CompactionFilterFactory::create_compaction_filter`.
pub unsafe fn new_compaction_filter_raw(
    c_name: CString,
    f: Box<dyn CompactionFilter>,
) -> *mut DBCompactionFilter {
    let proxy = Box::into_raw(Box::new(CompactionFilterProxy {
        name: c_name,
        filter: f,
    }));
    crocksdb_ffi::crocksdb_compactionfilter_create(proxy as *mut c_void, destructor, filter, name)
}

pub struct CompactionFilterContext(DBCompactionFilterContext);
impl CompactionFilterContext {
    pub fn is_full_compaction(&self) -> bool {
        let ctx = &self.0 as *const DBCompactionFilterContext;
        unsafe { crocksdb_ffi::crocksdb_compactionfiltercontext_is_full_compaction(ctx) }
    }

    pub fn is_manual_compaction(&self) -> bool {
        let ctx = &self.0 as *const DBCompactionFilterContext;
        unsafe { crocksdb_ffi::crocksdb_compactionfiltercontext_is_manual_compaction(ctx) }
    }
}

pub trait CompactionFilterFactory {
    fn create_compaction_filter(
        &self,
        context: &CompactionFilterContext,
    ) -> *mut DBCompactionFilter;
}

#[repr(C)]
struct CompactionFilterFactoryProxy {
    name: CString,
    factory: Box<dyn CompactionFilterFactory>,
}

mod factory {
    use super::{CompactionFilterContext, CompactionFilterFactoryProxy};
    use crocksdb_ffi::{DBCompactionFilter, DBCompactionFilterContext};
    use libc::{c_char, c_void};

    pub(super) extern "C" fn name(factory: *mut c_void) -> *const c_char {
        unsafe {
            let proxy = &*(factory as *mut CompactionFilterFactoryProxy);
            proxy.name.as_ptr()
        }
    }

    pub(super) extern "C" fn destructor(factory: *mut c_void) {
        unsafe {
            Box::from_raw(factory as *mut CompactionFilterFactoryProxy);
        }
    }

    pub(super) extern "C" fn create_compaction_filter(
        factory: *mut c_void,
        context: *const DBCompactionFilterContext,
    ) -> *mut DBCompactionFilter {
        unsafe {
            let factory = &mut *(factory as *mut CompactionFilterFactoryProxy);
            let context: &CompactionFilterContext = &*(context as *const CompactionFilterContext);
            factory.factory.create_compaction_filter(context)
        }
    }
}

pub struct CompactionFilterFactoryHandle {
    pub inner: *mut DBCompactionFilterFactory,
}

impl Drop for CompactionFilterFactoryHandle {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_compactionfilterfactory_destroy(self.inner);
        }
    }
}

pub unsafe fn new_compaction_filter_factory(
    c_name: CString,
    f: Box<dyn CompactionFilterFactory>,
) -> Result<CompactionFilterFactoryHandle, String> {
    let proxy = Box::into_raw(Box::new(CompactionFilterFactoryProxy {
        name: c_name,
        factory: f,
    }));

    let factory = crocksdb_ffi::crocksdb_compactionfilterfactory_create(
        proxy as *mut c_void,
        self::factory::destructor,
        self::factory::create_compaction_filter,
        self::factory::name,
    );

    Ok(CompactionFilterFactoryHandle { inner: factory })
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;
    use std::sync::mpsc::{self, SyncSender};
    use std::time::Duration;

    use super::{
        CompactionFilter, CompactionFilterContext, CompactionFilterFactory, DBCompactionFilter,
    };
    use crate::{ColumnFamilyOptions, DBOptions, DB};

    struct Factory(SyncSender<()>);
    impl Drop for Factory {
        fn drop(&mut self) {
            self.0.send(()).unwrap();
        }
    }
    impl CompactionFilterFactory for Factory {
        fn create_compaction_filter(&self, _: &CompactionFilterContext) -> *mut DBCompactionFilter {
            return std::ptr::null_mut();
        }
    }

    struct Filter(SyncSender<()>);
    impl Drop for Filter {
        fn drop(&mut self) {
            self.0.send(()).unwrap();
        }
    }
    impl CompactionFilter for Filter {
        fn filter(&mut self, _: usize, _: &[u8], _: &[u8], _: &mut Vec<u8>, _: &mut bool) -> bool {
            false
        }
    }

    #[test]
    fn test_factory_destructor() {
        let (tx, rx) = mpsc::sync_channel(1);
        let mut cf_opts = ColumnFamilyOptions::default();
        let name = CString::new("compaction filter factory").unwrap();
        let factory = Box::new(Factory(tx)) as Box<dyn CompactionFilterFactory>;
        cf_opts
            .set_compaction_filter_factory(name, factory)
            .unwrap();
        drop(cf_opts);
        assert!(rx.recv_timeout(Duration::from_secs(1)).is_ok());

        let dir = tempfile::Builder::new()
            .prefix("compaction_filter")
            .tempdir()
            .unwrap();
        let path = dir.path().to_str().unwrap();
        let (tx, rx) = mpsc::sync_channel(1);

        let mut db_opts = DBOptions::default();
        db_opts.create_if_missing(true);
        let mut cfds = Vec::new();
        cfds.push(("default", {
            let mut cf_opts = ColumnFamilyOptions::default();
            let name = CString::new("compaction filter factory").unwrap();
            let factory = Box::new(Factory(tx)) as Box<dyn CompactionFilterFactory>;
            cf_opts
                .set_compaction_filter_factory(name, factory)
                .unwrap();
            cf_opts
        }));
        let db = DB::open_cf(db_opts, path, cfds);
        drop(db);
        assert!(rx.recv_timeout(Duration::from_secs(1)).is_ok());
    }

    #[test]
    fn test_filter_destructor() {
        let (tx, rx) = mpsc::sync_channel(1);
        let mut cf_opts = ColumnFamilyOptions::default();
        let name = CString::new("compaction filter factory").unwrap();
        let filter = Box::new(Filter(tx)) as Box<dyn CompactionFilter>;
        cf_opts.set_compaction_filter(name, filter).unwrap();
        drop(cf_opts);
        assert!(rx.recv_timeout(Duration::from_secs(1)).is_ok());

        let dir = tempfile::Builder::new()
            .prefix("compaction_filter")
            .tempdir()
            .unwrap();
        let path = dir.path().to_str().unwrap();
        let (tx, rx) = mpsc::sync_channel(1);

        let mut db_opts = DBOptions::default();
        db_opts.create_if_missing(true);
        let mut cfds = Vec::new();
        cfds.push(("default", {
            let mut cf_opts = ColumnFamilyOptions::default();
            let name = CString::new("compaction filter factory").unwrap();
            let filter = Box::new(Filter(tx)) as Box<dyn CompactionFilter>;
            cf_opts.set_compaction_filter(name, filter).unwrap();
            cf_opts
        }));
        let db = DB::open_cf(db_opts, path, cfds);
        drop(db);
        assert!(rx.recv_timeout(Duration::from_secs(1)).is_ok());
    }
}
