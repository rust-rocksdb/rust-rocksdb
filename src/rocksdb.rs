extern crate libc;
use self::libc::{c_void, size_t};
use std::io::{IoError};
use std::c_vec::CVec;
use std::c_str::CString;
use std::str::from_utf8;

use rocksdb_ffi;

pub struct Rocksdb {
    inner: rocksdb_ffi::RocksdbInstance,
}

impl Rocksdb {
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

    pub fn get<'a>(&self, key: &[u8]) -> RocksdbResult<'a, RocksdbVector, String> {
        unsafe {
            let readopts = rocksdb_ffi::rocksdb_readoptions_create();
            let rocksdb_ffi::RocksdbReadOptions(read_opts_ptr) = readopts;
            if read_opts_ptr.is_null() {
                return RocksdbResult::Error("Unable to create rocksdb read \
                    options.  This is a fairly trivial call, and its failure \
                    may be indicative of a mis-compiled or mis-loaded rocksdb \
                    library.".to_string());
            }

            let val_len: size_t = 0;
            let val_len_ptr = &val_len as *const size_t;
            let err = 0 as *mut i8;
            let val = rocksdb_ffi::rocksdb_get(self.inner, readopts, key.as_ptr(),
                                  key.len() as size_t, val_len_ptr, err) as *mut u8;
            if err.is_not_null() {
                let cs = CString::new(err as *const i8, true);
                match cs.as_str() {
                    Some(error_string) =>
                        return RocksdbResult::Error(error_string.to_string()),
                    None =>
                        return RocksdbResult::Error("Unable to get value from \
                            rocksdb. (non-utf8 error received from underlying \
                            library)".to_string()),
                }
            }
            match val.is_null() {
                true =>  RocksdbResult::None,
                false => {
                    RocksdbResult::Some(RocksdbVector::from_c(val, val_len))
                }
            }
        }
    }

    pub fn close(&self) {
        unsafe { rocksdb_ffi::rocksdb_close(self.inner); }
    }

}

pub struct RocksdbVector {
    inner: CVec<u8>,
}

impl RocksdbVector {
    pub fn from_c(val: *mut u8, val_len: size_t) -> RocksdbVector {
        unsafe {
            RocksdbVector {
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

// RocksdbResult exists because of the inherent difference between
// an operational failure and the absence of a possible result.
#[deriving(Clone, PartialEq, PartialOrd, Eq, Ord, Show)]
pub enum RocksdbResult<'a,T,E> {
    Some(T),
    None,
    Error(E),
}

impl <'a,T,E> RocksdbResult<'a,T,E> {
    #[unstable = "waiting for unboxed closures"]
    pub fn map<U>(self, f: |T| -> U) -> RocksdbResult<U,E> {
        match self {
            RocksdbResult::Some(x) => RocksdbResult::Some(f(x)),
            RocksdbResult::None => RocksdbResult::None,
            RocksdbResult::Error(e) => RocksdbResult::Error(e),
        }
    }

    pub fn unwrap(self) -> T {
        match self {
            RocksdbResult::Some(x) => x,
            RocksdbResult::None => panic!("Attempted unwrap on RocksdbResult::None"),
            RocksdbResult::Error(_) => panic!("Attempted unwrap on RocksdbResult::Error"),
        }
    }

    #[unstable = "waiting for unboxed closures"]
    pub fn on_error<U>(self, f: |E| -> U) -> RocksdbResult<T,U> {
        match self {
            RocksdbResult::Some(x) => RocksdbResult::Some(x),
            RocksdbResult::None => RocksdbResult::None,
            RocksdbResult::Error(e) => RocksdbResult::Error(f(e)),
        }
    }

    #[unstable = "waiting for unboxed closures"]
    pub fn on_absent(self, f: || -> ()) -> RocksdbResult<T,E> {
        match self {
            RocksdbResult::Some(x) => RocksdbResult::Some(x),
            RocksdbResult::None => {
                f();
                RocksdbResult::None
            },
            RocksdbResult::Error(e) => RocksdbResult::Error(e),
        }
    }

    pub fn is_some(self) -> bool {
        match self {
            RocksdbResult::Some(_) => true,
            RocksdbResult::None => false,
            RocksdbResult::Error(_) => false,
        }
    }
    pub fn is_none(self) -> bool {
        match self {
            RocksdbResult::Some(_) => false,
            RocksdbResult::None => true,
            RocksdbResult::Error(_) => false,
        }
    }
    pub fn is_error(self) -> bool {
        match self {
            RocksdbResult::Some(_) => false,
            RocksdbResult::None => false,
            RocksdbResult::Error(_) => true,
        }
    }
}

pub fn create_or_open(path: String) -> Result<Rocksdb, String> {
  open(path, true)
}

pub fn open(path: String, create_if_missing: bool) -> Result<Rocksdb, String> {
    unsafe {
        let opts = rocksdb_ffi::rocksdb_options_create();
        let rocksdb_ffi::RocksdbOptions(opt_ptr) = opts;
        if opt_ptr.is_null() {
            return Err("Could not create options".to_string());
        }

        //rocksdb_ffi::rocksdb_options_increase_parallelism(opts, 2);
        //rocksdb_ffi::rocksdb_options_optimize_level_style_compaction(opts, 0);

        match create_if_missing {
            true => rocksdb_ffi::rocksdb_options_set_create_if_missing(opts, 1),
            false => rocksdb_ffi::rocksdb_options_set_create_if_missing(opts, 0),
        }

        let cpath = path.to_c_str();
        let cpath_ptr = cpath.as_ptr();

        //TODO test path here, as if rocksdb fails it will just crash the
        //     process currently

        let err = 0 as *mut i8;
        let db = rocksdb_ffi::rocksdb_open(opts, cpath_ptr, err);
        let rocksdb_ffi::RocksdbInstance(db_ptr) = db;
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
        Ok(Rocksdb{inner: db})
    }
}

#[allow(dead_code)]
#[test]
fn external() {
    let db = open("externaltest".to_string(), true).unwrap();
    let p = db.put(b"k1", b"v1111");
    assert!(p.is_ok());
    let r: RocksdbResult<RocksdbVector, String> = db.get(b"k1");
    //assert!(r.is_some());
    r.map(|v| { assert!(v.as_slice().len() == 5); } );
    db.close();
}
