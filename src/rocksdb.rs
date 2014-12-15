extern crate libc;
use self::libc::{c_char, c_int, c_void, size_t};
use std::io::{IoError};
use std::c_vec::CVec;
use std::c_str::CString;
use std::str::from_utf8;
use std::string::raw::from_buf_len;
use std::ptr;
use std::mem;
use std::slice;

use rocksdb_ffi;

pub struct RocksDBOptions {
    inner: rocksdb_ffi::RocksDBOptions,
    block_options: rocksdb_ffi::RocksDBBlockBasedTableOptions,
}

impl RocksDBOptions {
    pub fn new() -> RocksDBOptions {
        unsafe {
            let opts = rocksdb_ffi::rocksdb_options_create();
            let rocksdb_ffi::RocksDBOptions(opt_ptr) = opts;
            if opt_ptr.is_null() {
                panic!("Could not create rocksdb options".to_string());
            }
            let block_opts = rocksdb_ffi::rocksdb_block_based_options_create();

            RocksDBOptions{
                inner: opts,
                block_options: block_opts,
            }
        }
    }

    pub fn increase_parallelism(&self, parallelism: i32) {
        unsafe {
            rocksdb_ffi::rocksdb_options_increase_parallelism(
                self.inner, parallelism);
        }
    }

    pub fn optimize_level_style_compaction(&self,
        memtable_memory_budget: i32) {
        unsafe {
            rocksdb_ffi::rocksdb_options_optimize_level_style_compaction(
                self.inner, memtable_memory_budget);
        }
    }

    pub fn create_if_missing(&self, create_if_missing: bool) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_create_if_missing(
                self.inner, create_if_missing);
        }
    }

    pub fn add_merge_operator<'a>( &self, name: &str,
        merge_fn: fn (&[u8], Option<&[u8]>, &mut MergeOperands) -> Vec<u8>) {
        let cb = box MergeOperatorCallback {
            name: name.to_c_str(),
            merge_fn: merge_fn,
        };

        unsafe {
            let mo = rocksdb_ffi::rocksdb_mergeoperator_create(
                mem::transmute(cb),
                destructor_callback,
                full_merge_callback,
                partial_merge_callback,
                None,
                name_callback);
            rocksdb_ffi::rocksdb_options_set_merge_operator(self.inner, mo);
        }
    }

    pub fn set_block_size(&self, size: u64) {
        unsafe {
            rocksdb_ffi::rocksdb_block_based_options_set_block_size(
                self.block_options, size);
            rocksdb_ffi::rocksdb_options_set_block_based_table_factory(
                self.inner,
                self.block_options);
        }
    }

    pub fn set_block_cache_size_mb(&self, cache_size: u64) {
        unsafe {
            rocksdb_ffi::rocksdb_options_optimize_for_point_lookup(
                self.inner, cache_size);
        }
    }

    pub fn set_filter(&self, filter: rocksdb_ffi::RocksDBFilterPolicy) {
        unsafe {
            rocksdb_ffi::rocksdb_block_based_options_set_filter_policy(
                self.block_options, filter);
            rocksdb_ffi::rocksdb_options_set_block_based_table_factory(
                self.inner,
                self.block_options);
        }
    }

    pub fn set_max_open_files(&self, nfiles: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_max_open_files(self.inner, nfiles);
        }
    }

    pub fn set_use_fsync(&self, useit: bool) {
        unsafe {
            match useit {
                true =>
                    rocksdb_ffi::rocksdb_options_set_use_fsync(self.inner, 1),
                false =>
                    rocksdb_ffi::rocksdb_options_set_use_fsync(self.inner, 0),
            }
        }
    }

    pub fn set_bytes_per_sync(&self, nbytes: u64) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_bytes_per_sync(
                self.inner, nbytes);
        }
    }

    pub fn set_disable_data_sync(&self, disable: bool) {
        unsafe {
            match disable {
                true =>
                    rocksdb_ffi::rocksdb_options_set_disable_data_sync(
                        self.inner, 1),
                false =>
                    rocksdb_ffi::rocksdb_options_set_disable_data_sync(
                        self.inner, 0),
            }
        }
    }

    pub fn set_table_cache_num_shard_bits(&self, nbits: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_table_cache_numshardbits(
                self.inner, nbits);
        }
    }

    pub fn set_min_write_buffer_number(&self, nbuf: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_min_write_buffer_number_to_merge(
                self.inner, nbuf);
        }
    }

    pub fn set_max_write_buffer_number(&self, nbuf: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_max_write_buffer_number(
                self.inner, nbuf);
        }
    }

    pub fn set_write_buffer_size(&self, size: size_t) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_write_buffer_size(
                self.inner, size);
        }
    }

    pub fn set_target_file_size_base(&self, size: u64) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_target_file_size_base(
                self.inner, size);
        }
    }

    pub fn set_min_write_buffer_number_to_merge(&self, to_merge: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_min_write_buffer_number_to_merge(
                self.inner, to_merge);
        }
    }

    pub fn set_level_zero_slowdown_writes_trigger(&self, n: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_level0_slowdown_writes_trigger(
                self.inner, n);
        }
    }

    pub fn set_level_zero_stop_writes_trigger(&self, n: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_level0_stop_writes_trigger(
                self.inner, n);
        }
    }

    pub fn set_compaction_style(&self, style:
                                rocksdb_ffi::RocksDBCompactionStyle) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_compaction_style(
                self.inner, style);
        }
    }

    pub fn set_max_background_compactions(&self, n: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_max_background_compactions(
                self.inner, n);
        }
    }

    pub fn set_max_background_flushes(&self, n: c_int) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_max_background_flushes(
                self.inner, n);
        }
    }

    pub fn set_filter_deletes(&self, filter: bool) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_filter_deletes(
                self.inner, filter);
        }
    }

    pub fn set_disable_auto_compactions(&self, disable: bool) {
        unsafe {
            match disable {
                true =>
                    rocksdb_ffi::rocksdb_options_set_disable_auto_compactions(
                        self.inner, 1),
                false =>
                    rocksdb_ffi::rocksdb_options_set_disable_auto_compactions(
                        self.inner, 0),
            }
        }
    }
}

pub struct RocksDB {
    inner: rocksdb_ffi::RocksDBInstance,
}

impl RocksDB {
    pub fn open_default(path: &str) -> Result<RocksDB, String> {
        let opts = RocksDBOptions::new();
        opts.create_if_missing(true);
        RocksDB::open(opts, path)
    }

    pub fn open(opts: RocksDBOptions, path: &str) -> Result<RocksDB, String> {
        unsafe {
            let cpath = path.to_c_str();
            let cpath_ptr = cpath.as_ptr();

            //TODO test path here, as if rocksdb fails it will just crash the
            //     process currently

            let err = 0 as *mut i8;
            let db = rocksdb_ffi::rocksdb_open(opts.inner, cpath_ptr, err);
            let rocksdb_ffi::RocksDBInstance(db_ptr) = db;
            if err.is_not_null() {
                let cs = CString::new(err as *const i8, true);
                match cs.as_str() {
                    Some(error_string) =>
                        return Err(error_string.to_string()),
                    None =>
                        return Err(
                            "Could not initialize database.".to_string()),
                }
            }
            if db_ptr.is_null() {
                return Err("Could not initialize database.".to_string());
            }
            Ok(RocksDB{inner: db})
        }
    }

    pub fn destroy(opts: RocksDBOptions, path: &str) -> Result<(), String> {
        unsafe {
            let cpath = path.to_c_str();
            let cpath_ptr = cpath.as_ptr();

            //TODO test path here, as if rocksdb fails it will just crash the
            //     process currently

            let err = 0 as *mut i8;
            let result = rocksdb_ffi::rocksdb_destroy_db(
                opts.inner, cpath_ptr, err);
            if err.is_not_null() {
                let cs = CString::new(err as *const i8, true);
                match cs.as_str() {
                    Some(error_string) =>
                        return Err(error_string.to_string()),
                    None =>
                        return Err(
                            "Could not initialize database.".to_string()),
                }
            }
            Ok(())
        }
    }

    pub fn put(&self, key: &[u8], value: &[u8]) -> Result<(), String> {
        unsafe {
            let writeopts = rocksdb_ffi::rocksdb_writeoptions_create();
            let err = 0 as *mut i8;
            rocksdb_ffi::rocksdb_put(self.inner, writeopts, key.as_ptr(),
                        key.len() as size_t, value.as_ptr(),
                        value.len() as size_t, err);
            if err.is_not_null() {
                let cs = CString::new(err as *const i8, true);
                match cs.as_str() {
                    Some(error_string) =>
                        return Err(error_string.to_string()),
                    None => {
                        let ie = IoError::last_error();
                        return Err(format!(
                                "ERROR: desc:{}, details:{}",
                                ie.desc,
                                ie.detail.unwrap_or_else(
                                    || {"none provided by OS".to_string()})))
                    }
                }
            }
            return Ok(())
        }
    }

    pub fn merge(&self, key: &[u8], value: &[u8]) -> Result<(), String> {
        unsafe {
            let writeopts = rocksdb_ffi::rocksdb_writeoptions_create();
            let err = 0 as *mut i8;
            rocksdb_ffi::rocksdb_merge(self.inner, writeopts, key.as_ptr(),
                        key.len() as size_t, value.as_ptr(),
                        value.len() as size_t, err);
            if err.is_not_null() {
                let cs = CString::new(err as *const i8, true);
                match cs.as_str() {
                    Some(error_string) =>
                        return Err(error_string.to_string()),
                    None => {
                        let ie = IoError::last_error();
                        return Err(format!(
                                "ERROR: desc:{}, details:{}",
                                ie.desc,
                                ie.detail.unwrap_or_else(
                                    || {"none provided by OS".to_string()})))
                    }
                }
            }
            return Ok(())
        }
    }


    pub fn get<'a>(&self, key: &[u8]) ->
        RocksDBResult<'a, RocksDBVector, String> {
        unsafe {
            let readopts = rocksdb_ffi::rocksdb_readoptions_create();
            let rocksdb_ffi::RocksDBReadOptions(read_opts_ptr) = readopts;
            if read_opts_ptr.is_null() {
                return RocksDBResult::Error("Unable to create rocksdb read \
                    options.  This is a fairly trivial call, and its failure \
                    may be indicative of a mis-compiled or mis-loaded rocksdb \
                    library.".to_string());
            }

            let val_len: size_t = 0;
            let val_len_ptr = &val_len as *const size_t;
            let err = 0 as *mut i8;
            let val = rocksdb_ffi::rocksdb_get(self.inner, readopts,
                key.as_ptr(), key.len() as size_t, val_len_ptr, err) as *mut u8;
            if err.is_not_null() {
                let cs = CString::new(err as *const i8, true);
                match cs.as_str() {
                    Some(error_string) =>
                        return RocksDBResult::Error(error_string.to_string()),
                    None =>
                        return RocksDBResult::Error("Unable to get value from \
                            rocksdb. (non-utf8 error received from underlying \
                            library)".to_string()),
                }
            }
            match val.is_null() {
                true =>    RocksDBResult::None,
                false => {
                    RocksDBResult::Some(RocksDBVector::from_c(val, val_len))
                }
            }
        }
    }

    pub fn delete(&self, key: &[u8]) -> Result<(),String> {
        unsafe {
            let writeopts = rocksdb_ffi::rocksdb_writeoptions_create();
            let err = 0 as *mut i8;
            rocksdb_ffi::rocksdb_delete(self.inner, writeopts, key.as_ptr(),
                        key.len() as size_t, err);
            if err.is_not_null() {
                let cs = CString::new(err as *const i8, true);
                match cs.as_str() {
                    Some(error_string) =>
                        return Err(error_string.to_string()),
                    None => {
                        let ie = IoError::last_error();
                        return Err(format!(
                                "ERROR: desc:{}, details:{}",
                                ie.desc,
                                ie.detail.unwrap_or_else(
                                    || {"none provided by OS".to_string()})))
                    }
                }
            }
            return Ok(())
        }
    }

    pub fn close(&self) {
        unsafe { rocksdb_ffi::rocksdb_close(self.inner); }
    }
}

pub struct RocksDBVector {
    inner: CVec<u8>,
}

impl RocksDBVector {
    pub fn from_c(val: *mut u8, val_len: size_t) -> RocksDBVector {
        unsafe {
            RocksDBVector {
                inner:
                    CVec::new_with_dtor(val, val_len as uint,
                        proc(){ libc::free(val as *mut c_void); })
            }
        }
    }

    pub fn as_slice<'a>(&'a self) -> &'a [u8] {
        self.inner.as_slice()
    }

    pub fn to_utf8<'a>(&'a self) -> Option<&'a str> {
        from_utf8(self.inner.as_slice())
    }
}

// RocksDBResult exists because of the inherent difference between
// an operational failure and the absence of a possible result.
#[deriving(Clone, PartialEq, PartialOrd, Eq, Ord, Show)]
pub enum RocksDBResult<'a,T,E> {
    Some(T),
    None,
    Error(E),
}

impl <'a,T,E> RocksDBResult<'a,T,E> {
    #[unstable = "waiting for unboxed closures"]
    pub fn map<U>(self, f: |T| -> U) -> RocksDBResult<U,E> {
        match self {
            RocksDBResult::Some(x) => RocksDBResult::Some(f(x)),
            RocksDBResult::None => RocksDBResult::None,
            RocksDBResult::Error(e) => RocksDBResult::Error(e),
        }
    }

    pub fn unwrap(self) -> T {
        match self {
            RocksDBResult::Some(x) => x,
            RocksDBResult::None =>
                panic!("Attempted unwrap on RocksDBResult::None"),
            RocksDBResult::Error(_) =>
                panic!("Attempted unwrap on RocksDBResult::Error"),
        }
    }

    #[unstable = "waiting for unboxed closures"]
    pub fn on_error<U>(self, f: |E| -> U) -> RocksDBResult<T,U> {
        match self {
            RocksDBResult::Some(x) => RocksDBResult::Some(x),
            RocksDBResult::None => RocksDBResult::None,
            RocksDBResult::Error(e) => RocksDBResult::Error(f(e)),
        }
    }

    #[unstable = "waiting for unboxed closures"]
    pub fn on_absent(self, f: || -> ()) -> RocksDBResult<T,E> {
        match self {
            RocksDBResult::Some(x) => RocksDBResult::Some(x),
            RocksDBResult::None => {
                f();
                RocksDBResult::None
            },
            RocksDBResult::Error(e) => RocksDBResult::Error(e),
        }
    }

    pub fn is_some(self) -> bool {
        match self {
            RocksDBResult::Some(_) => true,
            RocksDBResult::None => false,
            RocksDBResult::Error(_) => false,
        }
    }
    pub fn is_none(self) -> bool {
        match self {
            RocksDBResult::Some(_) => false,
            RocksDBResult::None => true,
            RocksDBResult::Error(_) => false,
        }
    }
    pub fn is_error(self) -> bool {
        match self {
            RocksDBResult::Some(_) => false,
            RocksDBResult::None => false,
            RocksDBResult::Error(_) => true,
        }
    }
}

#[allow(dead_code)]
#[test]
fn external() {
    let path = "_rust_rocksdb_externaltest";
    let db = RocksDB::open_default(path).unwrap();
    let p = db.put(b"k1", b"v1111");
    assert!(p.is_ok());
    let r: RocksDBResult<RocksDBVector, String> = db.get(b"k1");
    assert!(r.unwrap().to_utf8().unwrap() == "v1111");
    assert!(db.delete(b"k1").is_ok());
    assert!(db.get(b"k1").is_none());
    db.close();
    let opts = RocksDBOptions::new();
    assert!(RocksDB::destroy(opts, path).is_ok());
}

pub struct MergeOperands<'a> {
    operands_list: *const *const c_char,
    operands_list_len: *const size_t,
    num_operands: uint,
    cursor: uint,
}

impl <'a> MergeOperands<'a> {
    fn new<'a>(operands_list: *const *const c_char,
               operands_list_len: *const size_t,
               num_operands: c_int) -> MergeOperands<'a> {
        assert!(num_operands >= 0);
        MergeOperands {
            operands_list: operands_list,
            operands_list_len: operands_list_len,
            num_operands: num_operands as uint,
            cursor: 0,
        }
    }
}

impl <'a> Iterator<&'a [u8]> for &'a mut MergeOperands<'a> {
    fn next(&mut self) -> Option<&'a [u8]> {
        use std::raw::Slice;
        match self.cursor == self.num_operands {
            true => None,
            false => {
                unsafe {
                    let base = self.operands_list as uint;
                    let base_len = self.operands_list_len as uint;
                    let spacing = mem::size_of::<*const *const u8>();
                    let spacing_len = mem::size_of::<*const size_t>();
                    let len_ptr = (base_len + (spacing_len * self.cursor))
                        as *const size_t;
                    let len = *len_ptr as uint;
                    let ptr = base + (spacing * self.cursor);
                    let op = from_buf_len(*(ptr as *const *const u8), len);
                    let des: Option<uint> = from_str(op.as_slice());
                    self.cursor += 1;
                    Some(mem::transmute(Slice{data:*(ptr as *const *const u8)
                        as *const u8, len: len}))
                }
            }
        }
    }

    fn size_hint(&self) -> (uint, Option<uint>) {
        let remaining = self.num_operands - self.cursor;
        (remaining, Some(remaining))
    }
}

struct MergeOperatorCallback {
    name: CString,
    merge_fn: fn (&[u8], Option<&[u8]>, &mut MergeOperands) -> Vec<u8>,
}

extern "C" fn destructor_callback(raw_cb: *mut c_void) {
    // turn this back into a local variable so rust will reclaim it
    let _: Box<MergeOperatorCallback> = unsafe {mem::transmute(raw_cb)};

}

extern "C" fn name_callback(raw_cb: *mut c_void) -> *const c_char {
    unsafe {
        let cb: &mut MergeOperatorCallback =
            &mut *(raw_cb as *mut MergeOperatorCallback);
        let ptr = cb.name.as_ptr();
        ptr as *const c_char
    }
}

extern "C" fn full_merge_callback(
    raw_cb: *mut c_void, key: *const c_char, key_len: size_t,
    existing_value: *const c_char, existing_value_len: size_t,
    operands_list: *const *const c_char, operands_list_len: *const size_t,
    num_operands: c_int,
    success: *mut u8, new_value_length: *mut size_t) -> *const c_char {
    unsafe {
        let cb: &mut MergeOperatorCallback =
            &mut *(raw_cb as *mut MergeOperatorCallback);
        let operands =
            &mut MergeOperands::new(operands_list,
                                    operands_list_len,
                                    num_operands);
        let key = from_buf_len(key as *const u8, key_len as uint);
        let oldval = from_buf_len(existing_value as *const u8,
                                  existing_value_len as uint);
        let mut result =
            (cb.merge_fn)(key.as_bytes(), Some(oldval.as_bytes()), operands);
        result.shrink_to_fit();
        /*
        let ptr = result.as_ptr();
        mem::forget(result);
        ptr as *const c_char
        */
        //TODO(tan) investigate zero-copy techniques to improve performance
        let buf = libc::malloc(result.len() as size_t);
        assert!(buf.is_not_null());
        *new_value_length = result.len() as size_t;
        *success = 1 as u8;
        ptr::copy_memory(&mut *buf, result.as_ptr()
                         as *const c_void, result.len());
        buf as *const c_char
    }
}

extern "C" fn partial_merge_callback(
    raw_cb: *mut c_void, key: *const c_char, key_len: size_t,
    operands_list: *const *const c_char, operands_list_len: *const size_t,
    num_operands: c_int,
    success: *mut u8, new_value_length: *mut size_t) -> *const c_char {
    unsafe {
        let cb: &mut MergeOperatorCallback =
            &mut *(raw_cb as *mut MergeOperatorCallback);
        let operands = &mut MergeOperands::new(operands_list,
                                               operands_list_len,
                                               num_operands);
        let key = from_buf_len(key as *const u8, key_len as uint);
        let mut result = (cb.merge_fn)(key.as_bytes(), None, operands);
        result.shrink_to_fit();
        //TODO(tan) investigate zero-copy techniques to improve performance
        let buf = libc::malloc(result.len() as size_t);
        assert!(buf.is_not_null());
        *new_value_length = 1 as size_t;
        *success = 1 as u8;
        ptr::copy_memory(&mut *buf, result.as_ptr()
                         as *const c_void, result.len());
        buf as *const c_char
    }
}

fn test_provided_merge(new_key: &[u8], existing_val: Option<&[u8]>,
    mut operands: &mut MergeOperands) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::with_capacity(operands.size_hint().val0());
    match existing_val {
        Some(v) => result.push_all(v),
        None => (),
    }
    for op in operands {
        result.push_all(op);
    }
    result
}

#[allow(dead_code)]
#[test]
fn mergetest() {
    let path = "_rust_rocksdb_mergetest";
    unsafe {
        let opts = RocksDBOptions::new();
        opts.create_if_missing(true);
        opts.add_merge_operator("test operator", test_provided_merge);
        let db = RocksDB::open(opts, path).unwrap();
        let p = db.put(b"k1", b"a");
        assert!(p.is_ok());
        db.merge(b"k1", b"b");
        db.merge(b"k1", b"c");
        db.merge(b"k1", b"d");
        db.merge(b"k1", b"efg");
        let m = db.merge(b"k1", b"h");
        assert!(m.is_ok());
        db.get(b"k1").map( |value| {
            match value.to_utf8() {
                Some(v) =>
                    println!("retrieved utf8 value: {}", v),
                None =>
                    println!("did not read valid utf-8 out of the db"),
            }
        }).on_absent( || { println!("value not present!") })
          .on_error( |e| { println!("error reading value")}); //: {", e) });

        assert!(m.is_ok());
        let r: RocksDBResult<RocksDBVector, String> = db.get(b"k1");
        assert!(r.unwrap().to_utf8().unwrap() == "abcdefgh");
        assert!(db.delete(b"k1").is_ok());
        assert!(db.get(b"k1").is_none());
        db.close();
        assert!(RocksDB::destroy(opts, path).is_ok());
    }
}
