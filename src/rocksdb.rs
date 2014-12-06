extern crate libc;
use self::libc::{c_char, c_int, c_void, size_t};
use std::io::{IoError};
use std::c_vec::CVec;
use std::c_str::CString;
use std::str::from_utf8;
use std::string::raw::from_buf_len;
use std::ptr;
use std::mem;
use std::num;
use std::slice;

use rocksdb_ffi;

pub struct RocksDBOptions {
    inner: rocksdb_ffi::RocksDBOptions,
}

impl RocksDBOptions {
    pub fn new() -> RocksDBOptions {
        unsafe {
            let opts = rocksdb_ffi::rocksdb_options_create();
            let rocksdb_ffi::RocksDBOptions(opt_ptr) = opts;
            if opt_ptr.is_null() {
                panic!("Could not create rocksdb options".to_string());
            }

            RocksDBOptions{inner: opts}
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
            match create_if_missing {
                true => rocksdb_ffi::rocksdb_options_set_create_if_missing(
                    self.inner, 1),
                false => rocksdb_ffi::rocksdb_options_set_create_if_missing(
                    self.inner, 0),
            }
        }
    }

    pub fn set_merge_operator(&self, mo: rocksdb_ffi::RocksDBMergeOperator) {
        unsafe {
            rocksdb_ffi::rocksdb_options_set_merge_operator(self.inner, mo);
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
                        return Err("Could not initialize database.".to_string()),
                }
            }
            if db_ptr.is_null() {
                return Err("Could not initialize database.".to_string());
            }
            Ok(RocksDB{inner: db})
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
    let db = RocksDB::open_default("externaltest").unwrap();
    let p = db.put(b"k1", b"v1111");
    assert!(p.is_ok());
    let r: RocksDBResult<RocksDBVector, String> = db.get(b"k1");
    assert!(r.unwrap().to_utf8().unwrap() == "v1111");
    assert!(db.delete(b"k1").is_ok());
    assert!(db.get(b"k1").is_none());
    db.close();
}

extern "C" fn null_destructor(args: *mut c_void) {
    println!("in null_destructor");
}
extern "C" fn mo_name(args: *mut c_void) -> *const c_char {
    println!("in mo_name");
    let name = "test_mo".to_c_str();
    unsafe {
        let buf = libc::malloc(8 as size_t);
        ptr::copy_memory(&mut *buf, name.as_ptr() as *const c_void, 8);
        println!("returning from mo_name");
        buf as *const c_char
    }
}

struct MergeOperands {
    operands_list: *const *const c_char,
    operands_list_len: *const size_t,
    num_operands: uint,
    cursor: uint,
}

impl MergeOperands {
    fn new(operands_list: *const *const c_char, operands_list_len: *const size_t,
        num_operands: c_int) -> MergeOperands {
        assert!(num_operands >= 0);
        MergeOperands {
            operands_list: operands_list,
            operands_list_len: operands_list_len,
            num_operands: num_operands as uint,
            cursor: 0,
        }
    }
}

impl <'a> Iterator<&'a [u8]> for MergeOperands {
    fn next(&mut self) -> Option<&'a [u8]> {
        match self.cursor == self.num_operands {
            true => None,
            false => {
                unsafe {
                    let base = self.operands_list as uint;
                    let base_len = self.operands_list_len as uint;
                    let spacing = mem::size_of::<*const *const u8>();
                    let spacing_len = mem::size_of::<*const size_t>();
                    let len_ptr = (base_len + (spacing_len * self.cursor)) as *const size_t;
                    let len = *len_ptr;
                    println!("len: {}", len);
                    let ptr = base + (spacing * self.cursor);
                    let op = slice::from_raw_buf(*(ptr as *const &*const u8), len as uint);
                    self.cursor += 1;
                    println!("returning: {}", from_utf8(op));
                    Some(op)
                }
            }
        }
    }
}

extern "C" fn full_merge(
    arg: *mut c_void, key: *const c_char, key_len: size_t,
    existing_value: *const c_char, existing_value_len: size_t,
    operands_list: *const *const c_char, operands_list_len: *const size_t,
    num_operands: c_int,
    success: *mut u8, new_value_length: *mut size_t) -> *const c_char {
    unsafe {
        println!("in the FULL merge operator");
        /*
        for mo in MergeOperands::new(operands_list, operands_list_len, num_operands) {
            println!("buf: {}", mo);
        }
        */
        let oldkey = from_buf_len(key as *const u8, key_len as uint);
        let oldval = from_buf_len(existing_value as *const u8, existing_value_len as uint);
        println!("old key: {}", oldval);
        let buf = libc::malloc(1 as size_t);
        match buf.is_null() {
            false => {
                *new_value_length = 1 as size_t;
                *success = 1 as u8;
                let newval = "2";
                ptr::copy_memory(&mut *buf, newval.as_ptr() as *const c_void, 1);
                println!("returning from full_merge");
                buf as *const c_char
            },
            true => {
                println!("returning from full_merge");
                0 as *const c_char
            }
        }
    }
}
extern "C" fn partial_merge(
    arg: *mut c_void, key: *const c_char, key_len: size_t,
    operands_list: *const c_void, operands_list_len: *const c_void,
    num_operands: c_int,
    success: *mut u8, new_value_length: *mut size_t) -> *const c_char {
    unsafe {
        println!("in the PARTIAL merge operator");
        *new_value_length = 2;
        *success = 1 as u8;
        let buf = libc::malloc(1 as size_t);
        match buf.is_null() {
            false => {
                println!("number of operands: {}", num_operands);
                println!("first operand: {}", from_buf_len(operands_list as *const u8, 1));
                *new_value_length = 1 as size_t;
                *success = 1 as u8;
                let newval = "2";
                ptr::copy_memory(&mut *buf, newval.as_ptr() as *const c_void, 1);
                println!("returning from partial_merge");
                buf as *const c_char
            },
            true => {
                println!("returning from partial_merge");
                0 as *const c_char
            }
        }
    }
}


#[allow(dead_code)]
#[zest]
fn mergetest() {
    unsafe {
        let opts = RocksDBOptions::new();
        let mo = rocksdb_ffi::rocksdb_mergeoperator_create(
            0 as *mut c_void,
            null_destructor,
            full_merge,
            partial_merge,
            None,
            mo_name);
        opts.create_if_missing(true);
        opts.set_merge_operator(mo);
        let db = RocksDB::open(opts, "externaltest").unwrap();
        let p = db.put(b"k1", b"1");
        assert!(p.is_ok());
        db.merge(b"k1", b"10");
        db.merge(b"k1", b"2");
        db.merge(b"k1", b"3");
        db.merge(b"k1", b"4");
        let m = db.merge(b"k1", b"5");
        assert!(m.is_ok());
        println!("after merge");
        db.get(b"k1").map( |value| {
            match value.to_utf8() {
                Some(v) =>
                    println!("retrieved utf8 value: {}", v),
                None =>
                    println!("did not read valid utf-8 out of the db"),
            }
        }).on_absent( || { println!("value not present!") })
          .on_error( |e| { println!("error reading value")}); //: {}", e) });

        assert!(m.is_ok());
        let r: RocksDBResult<RocksDBVector, String> = db.get(b"k1");
        assert!(r.unwrap().to_utf8().unwrap() == "2");
        assert!(db.delete(b"k1").is_ok());
        assert!(db.get(b"k1").is_none());
        db.close();
    }
}
