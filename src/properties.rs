//! Properties
//!
//! Full list of valid properties and descriptions pulled from
//! [here](https:///github.com/facebook/rocksdb/blob/08809f5e6cd9cc4bc3958dd4d59457ae78c76660/include/rocksdb/db.h#L428-L634).

use std::ffi::{CStr, CString};

macro_rules! property {
    ($suffix: literal) => {
        // SAFETY: We’re appending terminating NUL byte and this macro is always
        // called with values without interior NUL bytes.
        unsafe {
            CStr::from_bytes_with_nul_unchecked(concat!("rocksdb.", $suffix, "\0").as_bytes())
        }
    };
}

/// "rocksdb.num-files-at-level<N>" - returns string containing the number
/// of files at level <N>, where <N> is an ASCII representation of a
/// level number (e.g., "0").
pub fn num_files_at_level(level: usize) -> CString {
    unsafe { level_property("num-files-at-level", level) }
}

/// "rocksdb.compression-ratio-at-level<N>" - returns string containing the
/// compression ratio of data at level <N>, where <N> is an ASCII
/// representation of a level number (e.g., "0"). Here, compression
/// ratio is defined as uncompressed data size / compressed file size.
/// Returns "-1.0" if no open files at level <N>.
pub fn compression_ratio_at_level(level: usize) -> CString {
    unsafe { level_property("compression-ratio-at-level", level) }
}

/// "rocksdb.stats" - returns a multi-line string containing the data
/// described by kCFStats followed by the data described by kDBStats.
pub const STATS: &CStr = property!("stats");

/// "rocksdb.sstables" - returns a multi-line string summarizing current
/// SST files.
pub const SSTABLES: &CStr = property!("sstables");

/// "rocksdb.cfstats" - Both of "rocksdb.cfstats-no-file-histogram" and
/// "rocksdb.cf-file-histogram" together. See below for description
/// of the two.
pub const CFSTATS: &CStr = property!("CFSTATS");

/// "rocksdb.cfstats-no-file-histogram" - returns a multi-line string with
/// general columm family stats per-level over db's lifetime ("L<n>"),
/// aggregated over db's lifetime ("Sum"), and aggregated over the
/// interval since the last retrieval ("Int").
/// It could also be used to return the stats in the format of the map.
/// In this case there will a pair of string to array of double for
/// each level as well as for "Sum". "Int" stats will not be affected
/// when this form of stats are retrieved.
pub const CFSTATS_NO_FILE_HISTOGRAM: &CStr = property!("cfstats-no-file-histogram");

/// "rocksdb.cf-file-histogram" - print out how many file reads to every
/// level, as well as the histogram of latency of single requests.
pub const CF_FILE_HISTOGRAM: &CStr = property!("cf-file-histogram");

/// "rocksdb.dbstats" - returns a multi-line string with general database
/// stats, both cumulative (over the db's lifetime) and interval (since
/// the last retrieval of kDBStats).
pub const DBSTATS: &CStr = property!("dbstats");

/// "rocksdb.levelstats" - returns multi-line string containing the number
/// of files per level and total size of each level (MB).
pub const LEVELSTATS: &CStr = property!("levelstats");

/// "rocksdb.num-immutable-mem-table" - returns number of immutable
/// memtables that have not yet been flushed.
pub const NUM_IMMUTABLE_MEM_TABLE: &CStr = property!("num-immutable-mem-table");

/// "rocksdb.num-immutable-mem-table-flushed" - returns number of immutable
/// memtables that have already been flushed.
pub const NUM_IMMUTABLE_MEM_TABLE_FLUSHED: &CStr = property!("num-immutable-mem-table-flushed");

/// "rocksdb.mem-table-flush-pending" - returns 1 if a memtable flush is
/// pending; otherwise, returns 0.
pub const MEM_TABLE_FLUSH_PENDING: &CStr = property!("mem-table-flush-pending");

/// "rocksdb.num-running-flushes" - returns the number of currently running
/// flushes.
pub const NUM_RUNNING_FLUSHES: &CStr = property!("num-running-flushes");

/// "rocksdb.compaction-pending" - returns 1 if at least one compaction is
/// pending; otherwise, returns 0.
pub const COMPACTION_PENDING: &CStr = property!("compaction-pending");

/// "rocksdb.num-running-compactions" - returns the number of currently
/// running compactions.
pub const NUM_RUNNING_COMPACTIONS: &CStr = property!("num-running-compactions");

/// "rocksdb.background-errors" - returns accumulated number of background
/// errors.
pub const BACKGROUND_ERRORS: &CStr = property!("background-errors");

/// "rocksdb.cur-size-active-mem-table" - returns approximate size of active
/// memtable (bytes).
pub const CUR_SIZE_ACTIVE_MEM_TABLE: &CStr = property!("cur-size-active-mem-table");

/// "rocksdb.cur-size-all-mem-tables" - returns approximate size of active
/// and unflushed immutable memtables (bytes).
pub const CUR_SIZE_ALL_MEM_TABLES: &CStr = property!("cur-size-all-mem-tables");

/// "rocksdb.size-all-mem-tables" - returns approximate size of active,
/// unflushed immutable, and pinned immutable memtables (bytes).
pub const SIZE_ALL_MEM_TABLES: &CStr = property!("size-all-mem-tables");

/// "rocksdb.num-entries-active-mem-table" - returns total number of entries
/// in the active memtable.
pub const NUM_ENTRIES_ACTIVE_MEM_TABLE: &CStr = property!("num-entries-active-mem-table");

/// "rocksdb.num-entries-imm-mem-tables" - returns total number of entries
/// in the unflushed immutable memtables.
pub const NUM_ENTRIES_IMM_MEM_TABLES: &CStr = property!("num-entries-imm-mem-tables");

/// "rocksdb.num-deletes-active-mem-table" - returns total number of delete
/// entries in the active memtable.
pub const NUM_DELETES_ACTIVE_MEM_TABLE: &CStr = property!("num-deletes-active-mem-table");

/// "rocksdb.num-deletes-imm-mem-tables" - returns total number of delete
/// entries in the unflushed immutable memtables.
pub const NUM_DELETES_IMM_MEM_TABLES: &CStr = property!("num-deletes-imm-mem-tables");

/// "rocksdb.estimate-num-keys" - returns estimated number of total keys in
/// the active and unflushed immutable memtables and storage.
pub const ESTIMATE_NUM_KEYS: &CStr = property!("estimate-num-keys");

/// "rocksdb.estimate-table-readers-mem" - returns estimated memory used for
/// reading SST tables, excluding memory used in block cache (e.g.,
/// filter and index blocks).
pub const ESTIMATE_TABLE_READERS_MEM: &CStr = property!("estimate-table-readers-mem");

/// "rocksdb.is-file-deletions-enabled" - returns 0 if deletion of obsolete
/// files is enabled; otherwise, returns a non-zero number.
pub const IS_FILE_DELETIONS_ENABLED: &CStr = property!("is-file-deletions-enabled");

/// "rocksdb.num-snapshots" - returns number of unreleased snapshots of the
/// database.
pub const NUM_SNAPSHOTS: &CStr = property!("num-snapshots");

/// "rocksdb.oldest-snapshot-time" - returns number representing unix
/// timestamp of oldest unreleased snapshot.
pub const OLDEST_SNAPSHOT_TIME: &CStr = property!("oldest-snapshot-time");

/// "rocksdb.num-live-versions" - returns number of live versions. `Version`
/// is an internal data structure. See version_set.h for details. More
/// live versions often mean more SST files are held from being deleted,
/// by iterators or unfinished compactions.
pub const NUM_LIVE_VERSIONS: &CStr = property!("num-live-versions");

/// "rocksdb.current-super-version-number" - returns number of current LSM
/// version. It is a uint64_t integer number, incremented after there is
/// any change to the LSM tree. The number is not preserved after restarting
/// the DB. After DB restart, it will start from 0 again.
pub const CURRENT_SUPER_VERSION_NUMBER: &CStr = property!("current-super-version-number");

/// "rocksdb.estimate-live-data-size" - returns an estimate of the amount of
/// live data in bytes.
pub const ESTIMATE_LIVE_DATA_SIZE: &CStr = property!("estimate-live-data-size");

/// "rocksdb.min-log-number-to-keep" - return the minimum log number of the
/// log files that should be kept.
pub const MIN_LOG_NUMBER_TO_KEEP: &CStr = property!("min-log-number-to-keep");

/// "rocksdb.min-obsolete-sst-number-to-keep" - return the minimum file
/// number for an obsolete SST to be kept. The max value of `uint64_t`
/// will be returned if all obsolete files can be deleted.
pub const MIN_OBSOLETE_SST_NUMBER_TO_KEEP: &CStr = property!("min-obsolete-sst-number-to-keep");

/// "rocksdb.total-sst-files-size" - returns total size (bytes) of all SST
/// files.
/// WARNING: may slow down online queries if there are too many files.
pub const TOTAL_SST_FILES_SIZE: &CStr = property!("total-sst-files-size");

/// "rocksdb.live-sst-files-size" - returns total size (bytes) of all SST
/// files belong to the latest LSM tree.
pub const LIVE_SST_FILES_SIZE: &CStr = property!("live-sst-files-size");

/// "rocksdb.base-level" - returns number of level to which L0 data will be
/// compacted.
pub const BASE_LEVEL: &CStr = property!("base-level");

/// "rocksdb.estimate-pending-compaction-bytes" - returns estimated total
/// number of bytes compaction needs to rewrite to get all levels down
/// to under target size. Not valid for other compactions than level-
/// based.
pub const ESTIMATE_PENDING_COMPACTION_BYTES: &CStr = property!("estimate-pending-compaction-bytes");

/// "rocksdb.aggregated-table-properties" - returns a string representation
/// of the aggregated table properties of the target column family.
pub const AGGREGATED_TABLE_PROPERTIES: &CStr = property!("aggregated-table-properties");

/// "rocksdb.aggregated-table-properties-at-level<N>", same as the previous
/// one but only returns the aggregated table properties of the
/// specified level "N" at the target column family.
pub fn aggregated_table_properties_at_level(level: usize) -> CString {
    unsafe { level_property("aggregated-table-properties-at-level", level) }
}

/// "rocksdb.actual-delayed-write-rate" - returns the current actual delayed
/// write rate. 0 means no delay.
pub const ACTUAL_DELAYED_WRITE_RATE: &CStr = property!("actual-delayed-write-rate");

/// "rocksdb.is-write-stopped" - Return 1 if write has been stopped.
pub const IS_WRITE_STOPPED: &CStr = property!("is-write-stopped");

/// "rocksdb.estimate-oldest-key-time" - returns an estimation of
/// oldest key timestamp in the DB. Currently only available for
/// FIFO compaction with
/// compaction_options_fifo.allow_compaction = false.
pub const ESTIMATE_OLDEST_KEY_TIME: &CStr = property!("estimate-oldest-key-time");

/// "rocksdb.block-cache-capacity" - returns block cache capacity.
pub const BLOCK_CACHE_CAPACITY: &CStr = property!("block-cache-capacity");

/// "rocksdb.block-cache-usage" - returns the memory size for the entries
/// residing in block cache.
pub const BLOCK_CACHE_USAGE: &CStr = property!("block-cache-usage");

/// "rocksdb.block-cache-pinned-usage" - returns the memory size for the
/// entries being pinned.
pub const BLOCK_CACHE_PINNED_USAGE: &CStr = property!("block-cache-pinned-usage");

/// "rocksdb.options-statistics" - returns multi-line string
/// of options.statistics
pub const OPTIONS_STATISTICS: &CStr = property!("options-statistics");

/// Constructs a property name for an ‘at level’ property.
///
/// `name` is the infix of the property name (e.g. `"num-files-at-level"`) and
/// `level` is level to get statistics of.  The property name is constructed as
/// `"rocksdb.<name><level>"`.
///
/// Expects `name` not to contain any interior NUL bytes.
unsafe fn level_property(name: &str, level: usize) -> CString {
    let bytes = format!("rocksdb.{name}{level}\0").into_bytes();
    // SAFETY: We’re appending terminating NUL and all our call sites pass
    // a string without interior NUL bytes.
    CString::from_vec_with_nul_unchecked(bytes)
}

#[test]
fn sanity_checks() {
    let want = CString::new("rocksdb.cfstats-no-file-histogram".to_string()).unwrap();
    assert_eq!(want.as_c_str(), CFSTATS_NO_FILE_HISTOGRAM);

    let want = CString::new("rocksdb.num-files-at-level5".to_string()).unwrap();
    assert_eq!(want, num_files_at_level(5));
}
