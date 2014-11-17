extern crate libc;
use libc::{c_int, size_t};
use std::io::{IoResult, IoError};
use std::c_vec::CVec;

#[repr(C)]
struct RocksdbOptions(*const c_int);
#[repr(C)]
struct RocksdbInstance(*const c_int);
#[repr(C)]
struct RocksdbWriteOptions(*const c_int);
#[repr(C)]
struct RocksdbReadOptions(*const c_int);

#[link(name = "rocksdb")]
extern {
    fn rocksdb_options_create() -> RocksdbOptions;
	fn rocksdb_options_increase_parallelism(
        options: RocksdbOptions, threads: c_int);
	fn rocksdb_options_optimize_level_style_compaction(
        options: RocksdbOptions, memtable_memory_budget: c_int);
	fn rocksdb_options_set_create_if_missing(
        options: RocksdbOptions, v: c_int);
	fn rocksdb_open(options: RocksdbOptions,
        path: *const i8, err: *mut i8) -> RocksdbInstance;
    fn rocksdb_writeoptions_create() -> RocksdbWriteOptions;
	fn rocksdb_put(db: RocksdbInstance, writeopts: RocksdbWriteOptions,
                   k: *const u8, kLen: size_t, v: *const u8,
                   vLen: size_t, err: *mut i8);
	fn rocksdb_readoptions_create() -> RocksdbReadOptions;
    fn rocksdb_get(db: RocksdbInstance, readopts: RocksdbReadOptions,
                   k: *const u8, kLen: size_t,
                   valLen: *const size_t, err: *mut i8) -> *mut u8;
	fn rocksdb_close(db: RocksdbInstance);
}

pub struct Rocksdb {
    inner: RocksdbInstance,
}

impl Rocksdb {
   pub fn open(path: &str, create_if_missing: bool) -> IoResult<Rocksdb> {
        unsafe {
            let opts = rocksdb_options_create();
            let RocksdbOptions(opt_ptr) = opts;
            if opt_ptr.is_null() {
                return Err(IoError::last_error());
            }
            
            rocksdb_options_increase_parallelism(opts, 0);
            rocksdb_options_optimize_level_style_compaction(opts, 0);

            match create_if_missing {
                true => rocksdb_options_set_create_if_missing(opts, 1),
                false => rocksdb_options_set_create_if_missing(opts, 0),
            }

            let cpath = path.to_c_str();
            let cpath_ptr = cpath.as_ptr();
            
            let err = 0 as *mut i8;
            let db = rocksdb_open(opts, cpath_ptr, err); 
            let RocksdbInstance(db_ptr) = db;
            if err.is_not_null() {
                return Err(IoError::last_error());
            }
            if db_ptr.is_null() {
                return Err(IoError::last_error());
            }
            Ok(Rocksdb{inner: db})
        }
    }

    pub fn put(&self, key: &[u8], value: &[u8]) -> IoResult<bool> {
        unsafe {   
            let writeopts = rocksdb_writeoptions_create();
            let err = 0 as *mut i8;
            rocksdb_put(self.inner, writeopts, key.as_ptr(),
                        key.len() as size_t, value.as_ptr(),
                        value.len() as size_t, err);
            if err.is_not_null() {
                return Err(IoError::last_error());
            }
            return Ok(true)
        }
    }

    pub fn get(&self, key: &[u8]) -> IoResult<CVec<u8>> {
        unsafe {
            let readopts = rocksdb_readoptions_create();
            let RocksdbReadOptions(read_opts_ptr) = readopts;
            if read_opts_ptr.is_null() {
                return Err(IoError::last_error());
            }

            let mut val_len: size_t = 0;
            let val_len_ptr = &val_len as *const size_t;
            let err = 0 as *mut i8;
            let val = rocksdb_get(self.inner, readopts, key.as_ptr(),
                                  key.len() as size_t, val_len_ptr, err);
            if err.is_not_null() {
                return Err(IoError::last_error());
            }
            return Ok(CVec::new_with_dtor(val, val_len as uint,
                        proc(){
                            libc::free(val as *mut libc::c_void);
                        }))
        }
    }

    pub fn close(&self) {
        unsafe { rocksdb_close(self.inner); }
    }

}

#[test]
fn external() {
    let db = Rocksdb::open("testdb", true).unwrap();
    db.put(b"k1", b"v1111");
    let r = db.get(b"k1").unwrap();
    db.close();
    let v = r.get(0).unwrap();
    assert!(r.len() == 5);
}

#[test]
fn internal() {
    unsafe {
        let opts = rocksdb_options_create();
        let RocksdbOptions(opt_ptr) = opts;
        assert!(opt_ptr.is_not_null());
        
        rocksdb_options_increase_parallelism(opts, 0);
        rocksdb_options_optimize_level_style_compaction(opts, 0);
        rocksdb_options_set_create_if_missing(opts, 1);

        let rustpath = "datadir";
        let cpath = rustpath.to_c_str();
        let cpath_ptr = cpath.as_ptr();
        
        //TODO this will SIGSEGV
        let err = 0 as *mut i8;
        let db = rocksdb_open(opts, cpath_ptr, err); 
        assert!(err.is_null());

        let writeopts = rocksdb_writeoptions_create();
        let RocksdbWriteOptions(write_opt_ptr) = writeopts;
        assert!(write_opt_ptr.is_not_null());

        let key = b"name\x00";
        let val = b"spacejam\x00";

	    rocksdb_put(db, writeopts, key.as_ptr(), 4, val.as_ptr(), 8, err);
        assert!(err.is_null());

        let readopts = rocksdb_readoptions_create();
        let RocksdbReadOptions(read_opts_ptr) = readopts;
        assert!(read_opts_ptr.is_not_null());

        let mut val_len: size_t = 0;
        let val_len_ptr = &val_len as *const size_t;
        rocksdb_get(db, readopts, key.as_ptr(), 4, val_len_ptr, err);
        assert!(err.is_null());
        rocksdb_close(db);
    }
}
