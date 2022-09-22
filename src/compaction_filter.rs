use std::ffi::CString;
use std::{ptr, slice, usize};

use crate::table_properties::TableProperties;
use crocksdb_ffi::CompactionFilterDecision as RawCompactionFilterDecision;
pub use crocksdb_ffi::CompactionFilterValueType;
pub use crocksdb_ffi::DBCompactionFilter;
use crocksdb_ffi::{
    self, DBCompactionFilterContext, DBCompactionFilterFactory, DBTableFileCreationReason,
};
use libc::{c_char, c_int, c_uchar, c_void, malloc, memcpy, size_t};

/// Decision used in `CompactionFilter::filter`.
pub enum CompactionFilterDecision {
    /// The record will be kept instead of filtered.
    Keep,
    /// The record will be filtered, and a tombstone will be left.
    Remove,
    /// The record will be kept but the value will be replaced.
    ChangeValue(Vec<u8>),
    /// All records between [current, `until`) will be filtered without any tombstones left.
    RemoveAndSkipUntil(Vec<u8>),
}

/// `CompactionFilter` allows an application to modify/delete a key-value at
/// the time of compaction.
/// For more details, Please checkout rocksdb's documentation.
pub trait CompactionFilter {
    /// The compaction process invokes this
    /// method for kv that is being compacted. A return value
    /// of false indicates that the kv should be preserved in the
    /// output of this compaction run and a return value of true
    /// indicates that this key-value should be removed from the
    /// output of the compaction. The application can inspect
    /// the existing value of the key and make decision based on it.
    fn filter(
        &mut self,
        _level: usize,
        _key: &[u8],
        _value: &[u8],
        _new_value: &mut Vec<u8>,
        _value_changed: &mut bool,
    ) -> bool {
        false
    }

    /// This method will overwrite `filter` if a `CompactionFilter` implements both of them.
    fn featured_filter(
        &mut self,
        level: usize,
        key: &[u8],
        _seqno: u64,
        value: &[u8],
        value_type: CompactionFilterValueType,
    ) -> CompactionFilterDecision {
        match value_type {
            CompactionFilterValueType::Value => {
                let (mut new_value, mut value_changed) = (Vec::new(), false);
                if self.filter(level, key, value, &mut new_value, &mut value_changed) {
                    return CompactionFilterDecision::Remove;
                }
                if value_changed {
                    CompactionFilterDecision::ChangeValue(new_value)
                } else {
                    CompactionFilterDecision::Keep
                }
            }
            // Currently `MergeOperand` and `BlobIndex` will always be kept.
            _ => CompactionFilterDecision::Keep,
        }
    }
}

#[repr(C)]
struct CompactionFilterProxy<C: CompactionFilter> {
    name: CString,
    filter: C,
}

extern "C" fn name<C: CompactionFilter>(filter: *mut c_void) -> *const c_char {
    unsafe { (*(filter as *mut CompactionFilterProxy<C>)).name.as_ptr() }
}

extern "C" fn destructor<C: CompactionFilter>(filter: *mut c_void) {
    unsafe {
        Box::from_raw(filter as *mut CompactionFilterProxy<C>);
    }
}

extern "C" fn filter<C: CompactionFilter>(
    filter: *mut c_void,
    level: c_int,
    key: *const u8,
    key_len: size_t,
    seqno: u64,
    value_type: CompactionFilterValueType,
    value: *const u8,
    value_len: size_t,
    new_value: *mut *mut u8,
    new_value_len: *mut size_t,
    skip_until: *mut *mut u8,
    skip_until_length: *mut size_t,
) -> RawCompactionFilterDecision {
    unsafe {
        *new_value = ptr::null_mut();
        *new_value_len = 0;
        *skip_until = ptr::null_mut();
        *skip_until_length = 0;

        let filter = &mut (*(filter as *mut CompactionFilterProxy<C>)).filter;
        let key = slice::from_raw_parts(key, key_len);
        let value = slice::from_raw_parts(value, value_len);
        match filter.featured_filter(level as usize, key, seqno, value, value_type) {
            CompactionFilterDecision::Keep => RawCompactionFilterDecision::Keep,
            CompactionFilterDecision::Remove => RawCompactionFilterDecision::Remove,
            CompactionFilterDecision::ChangeValue(new_v) => {
                *new_value_len = new_v.len();
                *new_value = malloc(*new_value_len) as *mut u8;
                memcpy(*new_value as _, new_v.as_ptr() as _, *new_value_len);
                RawCompactionFilterDecision::ChangeValue
            }
            CompactionFilterDecision::RemoveAndSkipUntil(until) => {
                *skip_until_length = until.len();
                *skip_until = malloc(*skip_until_length) as *mut u8;
                memcpy(*skip_until as _, until.as_ptr() as _, *skip_until_length);
                RawCompactionFilterDecision::RemoveAndSkipUntil
            }
        }
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

pub unsafe fn new_compaction_filter<C: CompactionFilter>(
    c_name: CString,
    f: C,
) -> CompactionFilterHandle {
    let filter = new_compaction_filter_raw(c_name, f);
    CompactionFilterHandle { inner: filter }
}

/// Just like `new_compaction_filter`, but returns a raw pointer instead of a RAII struct.
/// Generally used in `CompactionFilterFactory::create_compaction_filter`.
pub unsafe fn new_compaction_filter_raw<C: CompactionFilter>(
    c_name: CString,
    f: C,
) -> *mut DBCompactionFilter {
    let proxy = Box::into_raw(Box::new(CompactionFilterProxy {
        name: c_name,
        filter: f,
    }));
    crocksdb_ffi::crocksdb_compactionfilter_create(
        proxy as *mut c_void,
        destructor::<C>,
        filter::<C>,
        name::<C>,
    )
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

    pub fn is_bottommost_level(&self) -> bool {
        let ctx = &self.0 as *const DBCompactionFilterContext;
        unsafe { crocksdb_ffi::crocksdb_compactionfiltercontext_is_bottommost_level(ctx) }
    }

    pub fn file_numbers(&self) -> &[u64] {
        let ctx = &self.0 as *const DBCompactionFilterContext;
        let (mut buffer, mut len): (*const u64, usize) = (ptr::null_mut(), 0);
        unsafe {
            crocksdb_ffi::crocksdb_compactionfiltercontext_file_numbers(
                ctx,
                &mut buffer as *mut *const u64,
                &mut len as *mut usize,
            );
            slice::from_raw_parts(buffer, len)
        }
    }

    pub fn table_properties(&self, offset: usize) -> &TableProperties {
        let ctx = &self.0 as *const DBCompactionFilterContext;
        unsafe {
            let raw = crocksdb_ffi::crocksdb_compactionfiltercontext_table_properties(ctx, offset);
            TableProperties::from_ptr(raw)
        }
    }

    pub fn start_key(&self) -> &[u8] {
        let ctx = &self.0 as *const DBCompactionFilterContext;
        unsafe {
            let mut start_key_len: usize = 0;
            let start_key_ptr =
                crocksdb_ffi::crocksdb_compactionfiltercontext_start_key(ctx, &mut start_key_len)
                    as *const u8;
            slice::from_raw_parts(start_key_ptr, start_key_len)
        }
    }

    pub fn end_key(&self) -> &[u8] {
        let ctx = &self.0 as *const DBCompactionFilterContext;
        unsafe {
            let mut end_key_len: usize = 0;
            let end_key_ptr =
                crocksdb_ffi::crocksdb_compactionfiltercontext_end_key(ctx, &mut end_key_len)
                    as *const u8;
            slice::from_raw_parts(end_key_ptr, end_key_len)
        }
    }
}

pub trait CompactionFilterFactory {
    fn create_compaction_filter(
        &self,
        context: &CompactionFilterContext,
    ) -> *mut DBCompactionFilter;

    /// Returns whether a thread creating table files for the specified `reason`
    /// should have invoke `create_compaction_filter` and pass KVs through the returned
    /// filter.
    fn should_filter_table_file_creation(&self, reason: DBTableFileCreationReason) -> c_uchar {
        // For compatibility, `CompactionFilter`s by default apply during compaction.
        matches!(reason, DBTableFileCreationReason::Compaction) as c_uchar
    }
}

#[repr(C)]
struct CompactionFilterFactoryProxy<C: CompactionFilterFactory> {
    name: CString,
    factory: C,
}

mod factory {
    use super::{CompactionFilterContext, CompactionFilterFactory, CompactionFilterFactoryProxy};
    use crocksdb_ffi::{DBCompactionFilter, DBCompactionFilterContext};
    use libc::{c_char, c_uchar, c_void};
    use librocksdb_sys::DBTableFileCreationReason;

    pub(super) extern "C" fn name<C: CompactionFilterFactory>(
        factory: *mut c_void,
    ) -> *const c_char {
        unsafe {
            let proxy = &*(factory as *mut CompactionFilterFactoryProxy<C>);
            proxy.name.as_ptr()
        }
    }

    pub(super) extern "C" fn destructor<C: CompactionFilterFactory>(factory: *mut c_void) {
        unsafe {
            Box::from_raw(factory as *mut CompactionFilterFactoryProxy<C>);
        }
    }

    pub(super) extern "C" fn create_compaction_filter<C: CompactionFilterFactory>(
        factory: *mut c_void,
        context: *const DBCompactionFilterContext,
    ) -> *mut DBCompactionFilter {
        unsafe {
            let factory = &mut *(factory as *mut CompactionFilterFactoryProxy<C>);
            let context: &CompactionFilterContext = &*(context as *const CompactionFilterContext);
            factory.factory.create_compaction_filter(context)
        }
    }

    pub(super) extern "C" fn should_filter_table_file_creation<C: CompactionFilterFactory>(
        factory: *const c_void,
        reason: DBTableFileCreationReason,
    ) -> c_uchar {
        unsafe {
            let factory = &*(factory as *const CompactionFilterFactoryProxy<C>);
            let reason: DBTableFileCreationReason = reason as DBTableFileCreationReason;
            factory.factory.should_filter_table_file_creation(reason)
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

pub unsafe fn new_compaction_filter_factory<C: CompactionFilterFactory>(
    c_name: CString,
    f: C,
) -> Result<CompactionFilterFactoryHandle, String> {
    let proxy = Box::into_raw(Box::new(CompactionFilterFactoryProxy {
        name: c_name,
        factory: f,
    }));

    let factory = crocksdb_ffi::crocksdb_compactionfilterfactory_create(
        proxy as *mut c_void,
        self::factory::destructor::<C>,
        self::factory::create_compaction_filter::<C>,
        self::factory::should_filter_table_file_creation::<C>,
        self::factory::name::<C>,
    );

    Ok(CompactionFilterFactoryHandle { inner: factory })
}

#[cfg(test)]
mod tests {
    use libc::c_uchar;
    use std::ffi::CString;
    use std::str;
    use std::sync::mpsc::{self, SyncSender};
    use std::time::Duration;

    use librocksdb_sys::DBTableFileCreationReason;

    use crate::{
        new_compaction_filter_raw, ColumnFamilyOptions, CompactionFilter, CompactionFilterContext,
        CompactionFilterFactory, DBCompactionFilter, DBOptions, Writable, DB,
    };

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

    struct KeyRangeFilter;

    impl CompactionFilter for KeyRangeFilter {
        fn filter(&mut self, _: usize, _: &[u8], _: &[u8], _: &mut Vec<u8>, _: &mut bool) -> bool {
            false
        }
    }

    struct KeyRangeFactory(SyncSender<Vec<u8>>);

    impl CompactionFilterFactory for KeyRangeFactory {
        fn create_compaction_filter(
            &self,
            context: &CompactionFilterContext,
        ) -> *mut DBCompactionFilter {
            let start_key = context.start_key();
            let end_key = context.end_key();
            &self.0.send(start_key.to_owned()).unwrap();
            &self.0.send(end_key.to_owned()).unwrap();

            unsafe {
                new_compaction_filter_raw::<KeyRangeFilter>(
                    CString::new("key_range_filter").unwrap(),
                    KeyRangeFilter,
                )
            }
        }
    }

    struct FlushFactory {}

    struct FlushFilter {}

    impl CompactionFilter for FlushFilter {
        fn filter(&mut self, _: usize, _: &[u8], _: &[u8], _: &mut Vec<u8>, _: &mut bool) -> bool {
            true
        }
    }

    impl CompactionFilterFactory for FlushFactory {
        fn should_filter_table_file_creation(&self, reason: DBTableFileCreationReason) -> c_uchar {
            matches!(reason, DBTableFileCreationReason::Flush) as c_uchar
        }

        fn create_compaction_filter(
            &self,
            _context: &CompactionFilterContext,
        ) -> *mut DBCompactionFilter {
            let name = CString::new("flush_compaction_filter").unwrap();
            unsafe { new_compaction_filter_raw::<FlushFilter>(name, FlushFilter {}) }
        }
    }

    #[test]
    fn test_factory_destructor() {
        let (tx, rx) = mpsc::sync_channel(1);
        let mut cf_opts = ColumnFamilyOptions::default();
        let name = CString::new("compaction filter factory").unwrap();
        let factory = Factory(tx);
        cf_opts
            .set_compaction_filter_factory::<CString, Factory>(name, factory)
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
            let factory = Factory(tx);
            cf_opts
                .set_compaction_filter_factory::<CString, Factory>(name, factory)
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
        let filter = Filter(tx);
        cf_opts
            .set_compaction_filter::<CString, Filter>(name, filter)
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
            let filter = Filter(tx);
            cf_opts
                .set_compaction_filter::<CString, Filter>(name, filter)
                .unwrap();
            cf_opts
        }));
        let db = DB::open_cf(db_opts, path, cfds);
        drop(db);
        assert!(rx.recv_timeout(Duration::from_secs(1)).is_ok());
    }

    #[test]
    fn test_compaction_filter_factory_context_keys() {
        let mut cf_opts = ColumnFamilyOptions::default();
        let name = CString::new("compaction filter factory").unwrap();
        let (tx, rx) = mpsc::sync_channel(2);
        let factory = KeyRangeFactory(tx);
        cf_opts
            .set_compaction_filter_factory::<CString, KeyRangeFactory>(name, factory)
            .unwrap();
        let mut opts = DBOptions::new();
        opts.create_if_missing(true);
        let path = tempfile::Builder::new()
            .prefix("test_factory_context_keys")
            .tempdir()
            .unwrap();
        let mut db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
        db.create_cf(("test", cf_opts)).unwrap();
        let cfh = db.cf_handle("test").unwrap();
        for i in 0..10 {
            db.put_cf(
                cfh,
                format!("key{}", i).as_bytes(),
                format!("value{}", i).as_bytes(),
            )
            .unwrap();
        }
        db.compact_range_cf(cfh, None, None);
        let sk = rx.recv().unwrap();
        let ek = rx.recv().unwrap();
        let sk = str::from_utf8(&sk).unwrap();
        let ek = str::from_utf8(&ek).unwrap();
        assert_eq!("key0", sk);
        assert_eq!("key9", ek);
    }

    #[test]
    fn test_flush_filter() {
        // cf with filter
        let name = CString::new("test_flush_filter_factory").unwrap();
        let factory = FlushFactory {};
        let mut cf_opts_wf = ColumnFamilyOptions::default();
        cf_opts_wf
            .set_compaction_filter_factory::<CString, FlushFactory>(name, factory)
            .unwrap();
        cf_opts_wf.set_disable_auto_compactions(true);

        // cf without filter
        let mut cf_opts_of = ColumnFamilyOptions::default();
        cf_opts_of.set_disable_auto_compactions(true);

        // db
        let mut opts = DBOptions::new();
        opts.create_if_missing(true);
        let path = tempfile::Builder::new()
            .prefix("test_factory_context_keys")
            .tempdir()
            .unwrap();
        let mut db = DB::open(opts, path.path().to_str().unwrap()).unwrap();
        db.create_cf(("wf", cf_opts_wf)).unwrap();
        db.create_cf(("of", cf_opts_of)).unwrap();
        let cfh_wf = db.cf_handle("wf").unwrap();
        let cfh_of = db.cf_handle("of").unwrap();

        // put data
        db.put_cf(cfh_wf, b"k", b"v").unwrap();
        db.put_cf(cfh_of, b"k", b"v").unwrap();
        db.flush_cf(cfh_wf, true).unwrap();
        db.flush_cf(cfh_of, true).unwrap();

        // assert
        assert!(db.get_cf(cfh_wf, b"k").unwrap().is_none());
        assert!(db.get_cf(cfh_of, b"k").unwrap().is_some());
    }
}
