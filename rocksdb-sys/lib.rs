extern crate libc;
use self::libc::{c_int, c_void, size_t};

#[repr(C)]
pub struct RocksdbOptions(pub *const c_void);
#[repr(C)]
pub struct RocksdbInstance(pub *const c_void);
#[repr(C)]
pub struct RocksdbWriteOptions(pub *const c_void);
#[repr(C)]
pub struct RocksdbReadOptions(pub *const c_void);
#[repr(C)]
pub struct RocksdbCompactionFilter(pub *const c_void);
#[repr(C)]
pub struct RocksdbMergeOperator(pub *const c_void);
#[repr(C)]
pub struct RocksdbFilterPolicy(pub *const c_void);

#[repr(C)]
pub enum RocksdbCompressionType {
  RocksdbNoCompression     = 0,
  RocksdbSnappyCompression = 1,
  RocksdbZlibCompression   = 2,
  RocksdbBz2Compression    = 3,
  RocksdbLz4Compression    = 4,
  RocksdbLz4hcCompression  = 5
}

#[repr(C)]
pub enum RocksdbCompactionStyle {
  RocksdbLevelCompaction     = 0,
  RocksdbUniversalCompaction = 1,
  RocksdbFifoCompaction      = 2
}

#[repr(C)]
pub enum RocksdbUniversalCompactionStyle {
  rocksdb_similar_size_compaction_stop_style = 0,
  rocksdb_total_size_compaction_stop_style   = 1
}

#[link(name = "rocksdb")]
extern {
    pub fn rocksdb_options_create() -> RocksdbOptions;
    pub fn rocksdb_options_increase_parallelism(
        options: RocksdbOptions, threads: c_int);
    pub fn rocksdb_options_optimize_level_style_compaction(
        options: RocksdbOptions, memtable_memory_budget: c_int);
    pub fn rocksdb_options_set_create_if_missing(
        options: RocksdbOptions, v: c_int);
    pub fn rocksdb_options_set_max_open_files(
        options: RocksdbOptions, files: c_int);
    pub fn rocksdb_options_set_use_fsync(
        options: RocksdbOptions, v: c_int);
    pub fn rocksdb_options_set_bytes_per_sync(
        options: RocksdbOptions, bytes: u64);
    pub fn rocksdb_options_set_disable_data_sync(
        options: RocksdbOptions, v: c_int);
    pub fn rocksdb_options_optimize_for_point_lookup(
        options: RocksdbOptions, block_cache_size_mb: u64);
    pub fn rocksdb_options_set_table_cache_numshardbits(
        options: RocksdbOptions, bits: u64);
    pub fn rocksdb_options_set_max_write_buffer_number(
        options: RocksdbOptions, bufno: c_int);
    pub fn rocksdb_options_set_min_write_buffer_number_to_merge(
        options: RocksdbOptions, bufno: c_int);
    pub fn rocksdb_options_set_level0_file_num_compaction_trigger(
        options: RocksdbOptions, no: c_int);
    pub fn rocksdb_options_set_level0_slowdown_writes_trigger(
        options: RocksdbOptions, no: c_int);
    pub fn rocksdb_options_set_level0_stop_writes_trigger(
        options: RocksdbOptions, no: c_int);
    pub fn rocksdb_options_set_write_buffer_size(
        options: RocksdbOptions, bytes: u64);
    pub fn rocksdb_options_set_target_file_size_base(
        options: RocksdbOptions, bytes: u64);
    pub fn rocksdb_options_set_target_file_size_multiplier(
        options: RocksdbOptions, mul: c_int);
    pub fn rocksdb_options_set_max_log_file_size(
        options: RocksdbOptions, bytes: u64);
    pub fn rocksdb_options_set_max_manifest_file_size(
        options: RocksdbOptions, bytes: u64);
    pub fn rocksdb_options_set_hash_skip_list_rep(
        options: RocksdbOptions, bytes: u64, a1: i32, a2: i32);
    pub fn rocksdb_options_set_compaction_style(
        options: RocksdbOptions, cs: RocksdbCompactionStyle);
    pub fn rocksdb_options_set_compression(
        options: RocksdbOptions, compression_style_no: c_int);
    pub fn rocksdb_options_set_max_background_compactions(
        options: RocksdbOptions, max_bg_compactions: c_int);
    pub fn rocksdb_options_set_max_background_flushes(
        options: RocksdbOptions, max_bg_flushes: c_int);
    pub fn rocksdb_options_set_filter_deletes(
        options: RocksdbOptions, v: u8);
    //pub fn rocksdb_compactionfilter_create() -> RocksdbCompactionFilter;
    //pub fn rocksdb_mergeoperator_create() -> RocksdbMergeOperator;
    pub fn rocksdb_filterpolicy_create_bloom(
        bits_per_key: c_int) -> RocksdbFilterPolicy;
    pub fn rocksdb_open(options: RocksdbOptions,
        path: *const i8, err: *mut i8) -> RocksdbInstance;
    pub fn rocksdb_writeoptions_create() -> RocksdbWriteOptions;
    pub fn rocksdb_put(db: RocksdbInstance, writeopts: RocksdbWriteOptions,
        k: *const u8, kLen: size_t, v: *const u8,
        vLen: size_t, err: *mut i8);
    pub fn rocksdb_readoptions_create() -> RocksdbReadOptions;
    pub fn rocksdb_get(db: RocksdbInstance, readopts: RocksdbReadOptions,
        k: *const u8, kLen: size_t,
        valLen: *const size_t, err: *mut i8) -> *mut u8;
    pub fn rocksdb_close(db: RocksdbInstance);
    pub fn rocksdb_destroy_db(
        options: RocksdbOptions, path: *const i8, err: *mut i8);
    pub fn rocksdb_repair_db(
        options: RocksdbOptions, path: *const i8, err: *mut i8);


}

#[allow(dead_code)]
#[test]
fn internal() {
    unsafe {
        let opts = rocksdb_options_create();
        let RocksdbOptions(opt_ptr) = opts;
        assert!(opt_ptr.is_not_null());
        
        rocksdb_options_increase_parallelism(opts, 0);
        rocksdb_options_optimize_level_style_compaction(opts, 0);
        rocksdb_options_set_create_if_missing(opts, 1);

        let rustpath = "internaltest";
        let cpath = rustpath.to_c_str();
        let cpath_ptr = cpath.as_ptr();
        
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

        let val_len: size_t = 0;
        let val_len_ptr = &val_len as *const size_t;
        rocksdb_get(db, readopts, key.as_ptr(), 4, val_len_ptr, err);
        assert!(err.is_null());
        libc::free(err as *mut c_void);
        rocksdb_close(db);
    }
}
