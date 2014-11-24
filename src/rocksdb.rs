extern crate libc;
use self::libc::{c_int, c_void, size_t};
use std::io::{IoResult, IoError};
use std::c_vec::CVec;
use std::c_str::CString;

use ffi;

// TODO learn more about lifetimes and determine if it's appropriate to keep
// inner on the stack, instead.
pub struct RocksdbVector {
    inner: Box<CVec<u8>>,
}

impl RocksdbVector {
    pub fn from_c(val: *mut u8, val_len: size_t) -> RocksdbVector {
        unsafe {
            RocksdbVector{
                inner:
                    box CVec::new_with_dtor(val, val_len as uint,
                        proc(){
                            libc::free(val as *mut c_void);
                        })}
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        self.inner.as_slice()
    }
}


#[deriving(Clone, PartialEq, PartialOrd, Eq, Ord, Show)]
pub enum RocksdbResult<T, E> {
    Some(T),
    None,
    Error(E),
}

impl <T,E> RocksdbResult<T,E> {
    #[unstable = "waiting for unboxed closures"]
    pub fn map<U>(self, f: |T| -> U) -> RocksdbResult<U,E> {
        match self {
            RocksdbResult::Some(x) => RocksdbResult::Some(f(x)),
            RocksdbResult::None => RocksdbResult::None,
            RocksdbResult::Error(e) => RocksdbResult::Error(e),
        }
    }

    pub fn is_some(self) -> bool {
        match self {
            RocksdbResult::Some(T) => true,
            RocksdbResult::None => false,
            RocksdbResult::Error(E) => false,
        }
    }
    pub fn is_none(self) -> bool {
        match self {
            RocksdbResult::Some(T) => false,
            RocksdbResult::None => true,
            RocksdbResult::Error(E) => false,
        }
    }
    pub fn is_error(self) -> bool {
        match self {
            RocksdbResult::Some(T) => false,
            RocksdbResult::None => false,
            RocksdbResult::Error(E) => true,
        }
    }
}

pub struct Rocksdb {
    inner: ffi::RocksdbInstance,
    path: String,
}

impl Rocksdb {
    pub fn put(&self, key: &[u8], value: &[u8]) -> IoResult<bool> {
        unsafe {   
            let writeopts = ffi::rocksdb_writeoptions_create();
            let err = 0 as *mut i8;
            ffi::rocksdb_put(self.inner, writeopts, key.as_ptr(),
                        key.len() as size_t, value.as_ptr(),
                        value.len() as size_t, err);
            if err.is_not_null() {
                libc::free(err as *mut c_void);
                return Err(IoError::last_error());
            }
            libc::free(err as *mut c_void);
            return Ok(true)
        }
    }

    pub fn get(&self, key: &[u8]) -> RocksdbResult<RocksdbVector, String> {
        unsafe {
            let readopts = ffi::rocksdb_readoptions_create();
            let ffi::RocksdbReadOptions(read_opts_ptr) = readopts;
            if read_opts_ptr.is_null() {
                return RocksdbResult::Error("Unable to create rocksdb read \
                    options.  This is a fairly trivial call, and its failure \
                    may be indicative of a mis-compiled or mis-loaded rocksdb \
                    library.".to_string());
            }

            let val_len: size_t = 0;
            let val_len_ptr = &val_len as *const size_t;
            let err = 0 as *mut i8;
            let val = ffi::rocksdb_get(self.inner, readopts, key.as_ptr(),
                                  key.len() as size_t, val_len_ptr, err);
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
        unsafe { ffi::rocksdb_close(self.inner); }
    }

}

pub fn open(path: String, create_if_missing: bool) -> Result<Rocksdb, String> {
    unsafe {
        let opts = ffi::rocksdb_options_create();
        let ffi::RocksdbOptions(opt_ptr) = opts;
        if opt_ptr.is_null() {
            return Err("Could not create options".to_string());
        }
        
        ffi::rocksdb_options_increase_parallelism(opts, 0);
        ffi::rocksdb_options_optimize_level_style_compaction(opts, 0);

        match create_if_missing {
            true => ffi::rocksdb_options_set_create_if_missing(opts, 1),
            false => ffi::rocksdb_options_set_create_if_missing(opts, 0),
        }

        let cpath = path.to_c_str();
        let cpath_ptr = cpath.as_ptr();
        
        let err = 0 as *mut i8;
        let db = ffi::rocksdb_open(opts, cpath_ptr, err); 
        let ffi::RocksdbInstance(db_ptr) = db;
        if err.is_not_null() {
            libc::free(err as *mut c_void);
            return Err("Could not initialize database.".to_string());
        }
        libc::free(err as *mut c_void);
        if db_ptr.is_null() {
            return Err("Could not initialize database.".to_string());
        }
        Ok(Rocksdb{inner: db, path: path})
    }
}

#[test]
fn external() {
    let db = open("testdb".to_string(), true).unwrap();
    db.put(b"k1", b"v1111");
    let r: RocksdbResult<RocksdbVector, String> = db.get(b"k1");
    //assert!(r.is_some());
    r.map(|v| { assert!(v.as_slice().len() == 5); } );
    db.close();
}
