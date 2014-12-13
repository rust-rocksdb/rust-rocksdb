extern crate libc;
use self::libc::{c_char, c_int, c_void, size_t};

#[repr(C)]
pub struct RocksDBOptions(pub *const c_void);
#[repr(C)]
pub struct RocksDBInstance(pub *const c_void);
#[repr(C)]
pub struct RocksDBWriteOptions(pub *const c_void);
#[repr(C)]
pub struct RocksDBReadOptions(pub *const c_void);
#[repr(C)]
pub struct RocksDBCompactionFilter(pub *const c_void);
#[repr(C)]
pub struct RocksDBMergeOperator(pub *const c_void);
#[repr(C)]
pub struct RocksDBFilterPolicy(pub *const c_void);

#[repr(C)]
pub enum RocksDBCompressionType {
    RocksDBNoCompression     = 0,
    RocksDBSnappyCompression = 1,
    RocksDBZlibCompression   = 2,
    RocksDBBz2Compression    = 3,
    RocksDBLz4Compression    = 4,
    RocksDBLz4hcCompression  = 5
}

#[repr(C)]
pub enum RocksDBCompactionStyle {
    RocksDBLevelCompaction     = 0,
    RocksDBUniversalCompaction = 1,
    RocksDBFifoCompaction      = 2
}

#[repr(C)]
pub enum RocksDBUniversalCompactionStyle {
    rocksdb_similar_size_compaction_stop_style = 0,
    rocksdb_total_size_compaction_stop_style   = 1
}

#[link(name = "rocksdb")]
extern {
    pub fn rocksdb_options_create() -> RocksDBOptions;
    pub fn rocksdb_options_increase_parallelism(
        options: RocksDBOptions, threads: c_int);
    pub fn rocksdb_options_optimize_level_style_compaction(
        options: RocksDBOptions, memtable_memory_budget: c_int);
    pub fn rocksdb_options_set_create_if_missing(
        options: RocksDBOptions, v: c_int);
    pub fn rocksdb_options_set_max_open_files(
        options: RocksDBOptions, files: c_int);
    pub fn rocksdb_options_set_use_fsync(
        options: RocksDBOptions, v: c_int);
    pub fn rocksdb_options_set_bytes_per_sync(
        options: RocksDBOptions, bytes: u64);
    pub fn rocksdb_options_set_disable_data_sync(
        options: RocksDBOptions, v: c_int);
    pub fn rocksdb_options_optimize_for_point_lookup(
        options: RocksDBOptions, block_cache_size_mb: u64);
    pub fn rocksdb_options_set_table_cache_numshardbits(
        options: RocksDBOptions, bits: u64);
    pub fn rocksdb_options_set_max_write_buffer_number(
        options: RocksDBOptions, bufno: c_int);
    pub fn rocksdb_options_set_max_write_buffer_number_to_merge(
        options: RocksDBOptions, bufno: c_int);
    pub fn rocksdb_options_set_min_write_buffer_number_to_merge(
        options: RocksDBOptions, bufno: c_int);
    pub fn rocksdb_options_set_level0_file_num_compaction_trigger(
        options: RocksDBOptions, no: c_int);
    pub fn rocksdb_options_set_level0_slowdown_writes_trigger(
        options: RocksDBOptions, no: c_int);
    pub fn rocksdb_options_set_level0_stop_writes_trigger(
        options: RocksDBOptions, no: c_int);
    pub fn rocksdb_options_set_write_buffer_size(
        options: RocksDBOptions, bytes: u64);
    pub fn rocksdb_options_set_target_file_size_base(
        options: RocksDBOptions, bytes: u64);
    pub fn rocksdb_options_set_target_file_size_multiplier(
        options: RocksDBOptions, mul: c_int);
    pub fn rocksdb_options_set_max_log_file_size(
        options: RocksDBOptions, bytes: u64);
    pub fn rocksdb_options_set_max_manifest_file_size(
        options: RocksDBOptions, bytes: u64);
    pub fn rocksdb_options_set_hash_skip_list_rep(
        options: RocksDBOptions, bytes: u64, a1: i32, a2: i32);
    pub fn rocksdb_options_set_compaction_style(
        options: RocksDBOptions, cs: RocksDBCompactionStyle);
    pub fn rocksdb_options_set_compression(
        options: RocksDBOptions, compression_style_no: c_int);
    pub fn rocksdb_options_set_max_background_compactions(
        options: RocksDBOptions, max_bg_compactions: c_int);
    pub fn rocksdb_options_set_max_background_flushes(
        options: RocksDBOptions, max_bg_flushes: c_int);
    pub fn rocksdb_options_set_filter_deletes(
        options: RocksDBOptions, v: u8);
    pub fn rocksdb_options_set_disable_auto_compactions(
        options: RocksDBOptions, v: u8);
    //pub fn rocksdb_compactionfilter_create() -> RocksDBCompactionFilter;
    pub fn rocksdb_filterpolicy_create_bloom(
        bits_per_key: c_int) -> RocksDBFilterPolicy;
    pub fn rocksdb_open(options: RocksDBOptions,
        path: *const i8, err: *mut i8) -> RocksDBInstance;
    pub fn rocksdb_writeoptions_create() -> RocksDBWriteOptions;
    pub fn rocksdb_put(db: RocksDBInstance, writeopts: RocksDBWriteOptions,
        k: *const u8, kLen: size_t, v: *const u8,
        vLen: size_t, err: *mut i8);
    pub fn rocksdb_readoptions_create() -> RocksDBReadOptions;
    pub fn rocksdb_get(db: RocksDBInstance, readopts: RocksDBReadOptions,
        k: *const u8, kLen: size_t,
        valLen: *const size_t, err: *mut i8) -> *mut c_void;
    pub fn rocksdb_delete(db: RocksDBInstance, writeopts: RocksDBWriteOptions,
        k: *const u8, kLen: size_t, err: *mut i8) -> *mut c_void;
    pub fn rocksdb_close(db: RocksDBInstance);
    pub fn rocksdb_destroy_db(
        options: RocksDBOptions, path: *const i8, err: *mut i8);
    pub fn rocksdb_repair_db(
        options: RocksDBOptions, path: *const i8, err: *mut i8);
    // Merge
    pub fn rocksdb_merge(db: RocksDBInstance, writeopts: RocksDBWriteOptions,
        k: *const u8, kLen: size_t,
        v: *const u8, vLen: size_t, err: *mut i8);
    pub fn rocksdb_mergeoperator_create(
        state: *mut c_void,
        destroy: extern fn(*mut c_void) -> (),
        full_merge: extern fn (
            arg: *mut c_void, key: *const c_char, key_len: size_t,
            existing_value: *const c_char, existing_value_len: size_t,
            operands_list: *const *const c_char, operands_list_len: *const size_t,
            num_operands: c_int,
            success: *mut u8, new_value_length: *mut size_t
        ) -> *const c_char,
        partial_merge: extern fn(
            arg: *mut c_void, key: *const c_char, key_len: size_t,
            operands_list: *const *const c_char, operands_list_len: *const size_t,
            num_operands: c_int,
            success: *mut u8, new_value_length: *mut size_t
        ) -> *const c_char,
        delete_value: Option<extern "C" fn(*mut c_void, value: *const c_char,
            value_len: *mut size_t) -> ()>,
        name_fn: extern fn(*mut c_void) -> *const c_char,
    ) -> RocksDBMergeOperator;
    pub fn rocksdb_mergeoperator_destroy(mo: RocksDBMergeOperator);
    pub fn rocksdb_options_set_merge_operator(
        options: RocksDBOptions,
        mo: RocksDBMergeOperator);
}

#[allow(dead_code)]
#[test]
fn internal() {
    unsafe {
        let opts = rocksdb_options_create();
        let RocksDBOptions(opt_ptr) = opts;
        assert!(opt_ptr.is_not_null());

        rocksdb_options_increase_parallelism(opts, 0);
        rocksdb_options_optimize_level_style_compaction(opts, 0);
        rocksdb_options_set_create_if_missing(opts, 1);

        let rustpath = "_rust_rocksdb_internaltest";
        let cpath = rustpath.to_c_str();
        let cpath_ptr = cpath.as_ptr();

        let err = 0 as *mut i8;
        let db = rocksdb_open(opts, cpath_ptr, err);
        assert!(err.is_null());

        let writeopts = rocksdb_writeoptions_create();
        let RocksDBWriteOptions(write_opt_ptr) = writeopts;
        assert!(write_opt_ptr.is_not_null());

        let key = b"name\x00";
        let val = b"spacejam\x00";

        rocksdb_put(db, writeopts, key.as_ptr(), 4, val.as_ptr(), 8, err);
        assert!(err.is_null());

        let readopts = rocksdb_readoptions_create();
        let RocksDBReadOptions(read_opts_ptr) = readopts;
        assert!(read_opts_ptr.is_not_null());

        let val_len: size_t = 0;
        let val_len_ptr = &val_len as *const size_t;
        rocksdb_get(db, readopts, key.as_ptr(), 4, val_len_ptr, err);
        assert!(err.is_null());
        rocksdb_close(db);
        rocksdb_destroy_db(opts, cpath_ptr, err);
        assert!(err.is_null());
    }
}
