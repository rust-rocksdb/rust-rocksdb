extern crate libc;
use self::libc::{c_int, c_void, size_t};
use std::io::{IoResult, IoError, BufferedStream};
use std::c_vec::CVec;

#[repr(C)]
struct RocksdbOptions(*const c_void);
#[repr(C)]
struct RocksdbInstance(*const c_void);
#[repr(C)]
struct RocksdbWriteOptions(*const c_void);
#[repr(C)]
struct RocksdbReadOptions(*const c_void);
#[repr(C)]
struct RocksdbCompactionFilter(*const c_void);
#[repr(C)]
struct RocksdbMergeOperator(*const c_void);
#[repr(C)]
struct RocksdbFilterPolicy(*const c_void);

#[repr(C)]
enum RocksdbCompressionType {
  RocksdbNoCompression     = 0,
  RocksdbSnappyCompression = 1,
  RocksdbZlibCompression   = 2,
  RocksdbBz2Compression    = 3,
  RocksdbLz4Compression    = 4,
  RocksdbLz4hcCompression  = 5
}

#[repr(C)]
enum RocksdbCompactionStyle {
  RocksdbLevelCompaction     = 0,
  RocksdbUniversalCompaction = 1,
  RocksdbFifoCompaction      = 2
}

#[repr(C)]
enum RocksdbUniversalCompactionStyle {
  rocksdb_similar_size_compaction_stop_style = 0,
  rocksdb_total_size_compaction_stop_style   = 1
}


#[link(name = "rocksdb")]
extern {
    fn rocksdb_options_create() -> RocksdbOptions;
    fn rocksdb_options_increase_parallelism(
        options: RocksdbOptions, threads: c_int);
    fn rocksdb_options_optimize_level_style_compaction(
        options: RocksdbOptions, memtable_memory_budget: c_int);
    fn rocksdb_options_set_create_if_missing(
        options: RocksdbOptions, v: c_int);
    fn rocksdb_options_set_max_open_files(
        options: RocksdbOptions, files: c_int);
    fn rocksdb_options_set_use_fsync(
        options: RocksdbOptions, v: c_int);
    fn rocksdb_options_set_bytes_per_sync(
        options: RocksdbOptions, bytes: u64);
    fn rocksdb_options_set_disable_data_sync(
        options: RocksdbOptions, v: c_int);
    fn rocksdb_options_optimize_for_point_lookup(
        options: RocksdbOptions, block_cache_size_mb: u64);
    fn rocksdb_options_set_table_cache_numshardbits(
        options: RocksdbOptions, bits: u64);
    fn rocksdb_options_set_max_write_buffer_number(
        options: RocksdbOptions, bufno: c_int);
    fn rocksdb_options_set_min_write_buffer_number_to_merge(
        options: RocksdbOptions, bufno: c_int);
    fn rocksdb_options_set_level0_file_num_compaction_trigger(
        options: RocksdbOptions, no: c_int);
    fn rocksdb_options_set_level0_slowdown_writes_trigger(
        options: RocksdbOptions, no: c_int);
    fn rocksdb_options_set_level0_stop_writes_trigger(
        options: RocksdbOptions, no: c_int);
    fn rocksdb_options_set_write_buffer_size(
        options: RocksdbOptions, bytes: u64);
    fn rocksdb_options_set_target_file_size_base(
        options: RocksdbOptions, bytes: u64);
    fn rocksdb_options_set_target_file_size_multiplier(
        options: RocksdbOptions, mul: c_int);
    fn rocksdb_options_set_max_log_file_size(
        options: RocksdbOptions, bytes: u64);
    fn rocksdb_options_set_max_manifest_file_size(
        options: RocksdbOptions, bytes: u64);
    fn rocksdb_options_set_hash_skip_list_rep(
        options: RocksdbOptions, bytes: u64, a1: i32, a2: i32);
    fn rocksdb_options_set_compaction_style(
        options: RocksdbOptions, cs: RocksdbCompactionStyle);
    fn rocksdb_options_set_compression(
        options: RocksdbOptions, compression_style_no: c_int);
    fn rocksdb_options_set_max_background_compactions(
        options: RocksdbOptions, max_bg_compactions: c_int);
    fn rocksdb_options_set_max_background_flushes(
        options: RocksdbOptions, max_bg_flushes: c_int);
    fn rocksdb_options_set_filter_deletes(
        options: RocksdbOptions, v: u8);
    //fn rocksdb_compactionfilter_create() -> RocksdbCompactionFilter;
    //fn rocksdb_mergeoperator_create() -> RocksdbMergeOperator;
    fn rocksdb_filterpolicy_create_bloom(
        bits_per_key: c_int) -> RocksdbFilterPolicy;
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
    path: String,
}

impl Rocksdb {
    pub fn put(&self, key: &[u8], value: &[u8]) -> IoResult<bool> {
        unsafe {   
            let writeopts = rocksdb_writeoptions_create();
            let err = 0 as *mut i8;
            rocksdb_put(self.inner, writeopts, key.as_ptr(),
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

    pub fn get(&self, key: &[u8]) -> IoResult<CVec<u8>> {
        unsafe {
            let readopts = rocksdb_readoptions_create();
            let RocksdbReadOptions(read_opts_ptr) = readopts;
            if read_opts_ptr.is_null() {
                return Err(IoError::last_error());
            }

            let val_len: size_t = 0;
            let val_len_ptr = &val_len as *const size_t;
            let err = 0 as *mut i8;
            let val = rocksdb_get(self.inner, readopts, key.as_ptr(),
                                  key.len() as size_t, val_len_ptr, err);
            if err.is_not_null() {
                libc::free(err as *mut c_void);
                return Err(IoError::last_error());
            }
            libc::free(err as *mut c_void);
            return Ok(CVec::new_with_dtor(val, val_len as uint,
                        proc(){
                            libc::free(val as *mut c_void);
                        }))
        }
    }

    pub fn close(&self) {
        unsafe { rocksdb_close(self.inner); }
    }

}

pub fn open(path: String, create_if_missing: bool) -> Result<Rocksdb, String> {
    unsafe {
        let opts = rocksdb_options_create();
        let RocksdbOptions(opt_ptr) = opts;
        if opt_ptr.is_null() {
            return Err("Could not create options".to_string());
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
        libc::free(err as *mut c_void);

        let writeopts = rocksdb_writeoptions_create();
        let RocksdbWriteOptions(write_opt_ptr) = writeopts;
        assert!(write_opt_ptr.is_not_null());

        let key = b"name\x00";
        let val = b"spacejam\x00";

        rocksdb_put(db, writeopts, key.as_ptr(), 4, val.as_ptr(), 8, err);
        assert!(err.is_null());
        libc::free(err as *mut c_void);

        let readopts = rocksdb_readoptions_create();
        let RocksdbReadOptions(read_opts_ptr) = readopts;
        assert!(read_opts_ptr.is_not_null());
        libc::free(err as *mut c_void);

        let mut val_len: size_t = 0;
        let val_len_ptr = &val_len as *const size_t;
        rocksdb_get(db, readopts, key.as_ptr(), 4, val_len_ptr, err);
        assert!(err.is_null());
        libc::free(err as *mut c_void);
        rocksdb_close(db);
    }
}

#[test]
fn external() {
    let db = open("testdb".to_string(), true).unwrap();
    db.put(b"k1", b"v1111");
    let r = db.get(b"k1").unwrap();
    assert!(r.len() == 5);
    let v = r.get(0).unwrap();
    db.close();
}
