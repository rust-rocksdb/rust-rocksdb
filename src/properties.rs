// Copyright ???
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

/// Full list of valid properties and descriptions pulled from
/// [here](https://github.com/facebook/rocksdb/blob/08809f5e6cd9cc4bc3958dd4d59457ae78c76660/include/rocksdb/db.h#L428-L634).

// concat!() takes string literals only so use a macro instead of another constant
macro_rules! ROCKSDB_PREFIX {
    () => {
        "rocksdb."
    };
}

//  "rocksdb.num-files-at-level<N>" - returns string containing the number
//      of files at level <N>, where <N> is an ASCII representation of a
//      level number (e.g., "0").
pub const NUM_FILES_AT_LEVEL_PREFIX: &str = concat!(ROCKSDB_PREFIX!(), "num-files-at-level");

//  "rocksdb.compression-ratio-at-level<N>" - returns string containing the
//      compression ratio of data at level <N>, where <N> is an ASCII
//      representation of a level number (e.g., "0"). Here, compression
//      ratio is defined as uncompressed data size / compressed file size.
//      Returns "-1.0" if no open files at level <N>.
pub const COMPRESSION_RATIO_AT_LEVEL: &str =
    concat!(ROCKSDB_PREFIX!(), "rocksdb.compression-ratio-at-level");

//  "rocksdb.stats" - returns a multi-line string containing the data
//      described by kCFStats followed by the data described by kDBStats.
pub const STATS: &str = concat!(ROCKSDB_PREFIX!(), "stats");

//  "rocksdb.sstables" - returns a multi-line string summarizing current
//      SST files.
pub const SSTABLES: &str = concat!(ROCKSDB_PREFIX!(), "sstables");

//  "rocksdb.cfstats" - Both of "rocksdb.cfstats-no-file-histogram" and
//      "rocksdb.cf-file-histogram" together. See below for description
//      of the two.
pub const CFSTATS: &str = concat!(ROCKSDB_PREFIX!(), "CFSTATS");

//  "rocksdb.cfstats-no-file-histogram" - returns a multi-line string with
//      general columm family stats per-level over db's lifetime ("L<n>"),
//      aggregated over db's lifetime ("Sum"), and aggregated over the
//      interval since the last retrieval ("Int").
//  It could also be used to return the stats in the format of the map.
//  In this case there will a pair of string to array of double for
//  each level as well as for "Sum". "Int" stats will not be affected
//  when this form of stats are retrieved.
pub const CFSTATS_NO_FILE_HISTOGRAM: &str = concat!(ROCKSDB_PREFIX!(), "cfstats-no-file-histogram");

//  "rocksdb.cf-file-histogram" - print out how many file reads to every
//      level, as well as the histogram of latency of single requests.
pub const CF_FILE_HISTOGRAM: &str = concat!(ROCKSDB_PREFIX!(), "cf-file-histogram");

//  "rocksdb.dbstats" - returns a multi-line string with general database
//      stats, both cumulative (over the db's lifetime) and interval (since
//      the last retrieval of kDBStats).
pub const DBSTATS: &str = concat!(ROCKSDB_PREFIX!(), "dbstats");

//  "rocksdb.levelstats" - returns multi-line string containing the number
//      of files per level and total size of each level (MB).
pub const LEVELSTATS: &str = concat!(ROCKSDB_PREFIX!(), "levelstats");

//  "rocksdb.num-immutable-mem-table" - returns number of immutable
//      memtables that have not yet been flushed.
pub const NUM_IMMUTABLE_MEM_TABLE: &str = concat!(ROCKSDB_PREFIX!(), "num-immutable-mem-table");

//  "rocksdb.num-immutable-mem-table-flushed" - returns number of immutable
//      memtables that have already been flushed.
pub const NUM_IMMUTABLE_MEM_TABLE_FLUSHED: &str =
    concat!(ROCKSDB_PREFIX!(), "num-immutable-mem-table-flushed");

//  "rocksdb.mem-table-flush-pending" - returns 1 if a memtable flush is
//      pending; otherwise, returns 0.
pub const MEM_TABLE_FLUSH_PENDING: &str = concat!(ROCKSDB_PREFIX!(), "mem-table-flush-pending");

//  "rocksdb.num-running-flushes" - returns the number of currently running
//      flushes.
pub const NUM_RUNNING_FLUSHES: &str = concat!(ROCKSDB_PREFIX!(), "num-running-flushes");

//  "rocksdb.compaction-pending" - returns 1 if at least one compaction is
//      pending; otherwise, returns 0.
pub const COMPACTION_PENDING: &str = concat!(ROCKSDB_PREFIX!(), "compaction-pending");

//  "rocksdb.num-running-compactions" - returns the number of currently
//      running compactions.
pub const NUM_RUNNING_COMPACTIONS: &str = concat!(ROCKSDB_PREFIX!(), "num-running-compactions");

//  "rocksdb.background-errors" - returns accumulated number of background
//      errors.
pub const BACKGROUND_ERRORS: &str = concat!(ROCKSDB_PREFIX!(), "background-errors");

//  "rocksdb.cur-size-active-mem-table" - returns approximate size of active
//      memtable (bytes).
pub const CUR_SIZE_ACTIVE_MEM_TABLE: &str = concat!(ROCKSDB_PREFIX!(), "cur-size-active-mem-table");

//  "rocksdb.cur-size-all-mem-tables" - returns approximate size of active
//      and unflushed immutable memtables (bytes).
pub const CUR_SIZE_ALL_MEM_TABLES: &str = concat!(ROCKSDB_PREFIX!(), "cur-size-all-mem-tables");

//  "rocksdb.size-all-mem-tables" - returns approximate size of active,
//      unflushed immutable, and pinned immutable memtables (bytes).
pub const SIZE_ALL_MEM_TABLES: &str = concat!(ROCKSDB_PREFIX!(), "size-all-mem-tables");

//  "rocksdb.num-entries-active-mem-table" - returns total number of entries
//      in the active memtable.
pub const NUM_ENTRIES_ACTIVE_MEM_TABLE: &str =
    concat!(ROCKSDB_PREFIX!(), "num-entries-active-mem-table");

//  "rocksdb.num-entries-imm-mem-tables" - returns total number of entries
//      in the unflushed immutable memtables.
pub const NUM_ENTRIES_IMM_MEM_TABLES: &str =
    concat!(ROCKSDB_PREFIX!(), "num-entries-imm-mem-tables");

//  "rocksdb.num-deletes-active-mem-table" - returns total number of delete
//      entries in the active memtable.
pub const NUM_DELETES_ACTIVE_MEM_TABLE: &str =
    concat!(ROCKSDB_PREFIX!(), "num-deletes-active-mem-table");

//  "rocksdb.num-deletes-imm-mem-tables" - returns total number of delete
//      entries in the unflushed immutable memtables.
pub const NUM_DELETES_IMM_MEM_TABLES: &str =
    concat!(ROCKSDB_PREFIX!(), "num-deletes-imm-mem-tables");

//  "rocksdb.estimate-num-keys" - returns estimated number of total keys in
//      the active and unflushed immutable memtables and storage.
pub const ESTIMATE_NUM_KEYS: &str = concat!(ROCKSDB_PREFIX!(), "estimate-num-keys");

//  "rocksdb.estimate-table-readers-mem" - returns estimated memory used for
//      reading SST tables, excluding memory used in block cache (e.g.,
//      filter and index blocks).
pub const ESTIMATE_TABLE_READERS_MEM: &str =
    concat!(ROCKSDB_PREFIX!(), "estimate-table-readers-mem");

//  "rocksdb.is-file-deletions-enabled" - returns 0 if deletion of obsolete
//      files is enabled; otherwise, returns a non-zero number.
pub const IS_FILE_DELETIONS_ENABLED: &str = concat!(ROCKSDB_PREFIX!(), "is-file-deletions-enabled");

//  "rocksdb.num-snapshots" - returns number of unreleased snapshots of the
//      database.
pub const NUM_SNAPSHOTS: &str = concat!(ROCKSDB_PREFIX!(), "num-snapshots");

//  "rocksdb.oldest-snapshot-time" - returns number representing unix
//      timestamp of oldest unreleased snapshot.
pub const OLDEST_SNAPSHOT_TIME: &str = concat!(ROCKSDB_PREFIX!(), "oldest-snapshot-time");

//  "rocksdb.num-live-versions" - returns number of live versions. `Version`
//      is an internal data structure. See version_set.h for details. More
//      live versions often mean more SST files are held from being deleted,
//      by iterators or unfinished compactions.
pub const NUM_LIVE_VERSIONS: &str = concat!(ROCKSDB_PREFIX!(), "num-live-versions");

//  "rocksdb.current-super-version-number" - returns number of current LSM
//  version. It is a uint64_t integer number, incremented after there is
//  any change to the LSM tree. The number is not preserved after restarting
//  the DB. After DB restart, it will start from 0 again.
pub const CURRENT_SUPER_VERSION_NUMBER: &str =
    concat!(ROCKSDB_PREFIX!(), "current-super-version-number");

//  "rocksdb.estimate-live-data-size" - returns an estimate of the amount of
//      live data in bytes.
pub const ESTIMATE_LIVE_DATA_SIZE: &str = concat!(ROCKSDB_PREFIX!(), "estimate-live-data-size");

//  "rocksdb.min-log-number-to-keep" - return the minimum log number of the
//      log files that should be kept.
pub const MIN_LOG_NUMBER_TO_KEEP: &str = concat!(ROCKSDB_PREFIX!(), "min-log-number-to-keep");

//  "rocksdb.min-obsolete-sst-number-to-keep" - return the minimum file
//      number for an obsolete SST to be kept. The max value of `uint64_t`
//      will be returned if all obsolete files can be deleted.
pub const MIN_OBSOLETE_SST_NUMBER_TO_KEEP: &str =
    concat!(ROCKSDB_PREFIX!(), "min-obsolete-sst-number-to-keep");

//  "rocksdb.total-sst-files-size" - returns total size (bytes) of all SST
//      files.
//  WARNING: may slow down online queries if there are too many files.
pub const TOTAL_SST_FILES_SIZE: &str = concat!(ROCKSDB_PREFIX!(), "total-sst-files-size");

//  "rocksdb.live-sst-files-size" - returns total size (bytes) of all SST
//      files belong to the latest LSM tree.
pub const LIVE_SST_FILES_SIZE: &str = concat!(ROCKSDB_PREFIX!(), "live-sst-files-size");

//  "rocksdb.base-level" - returns number of level to which L0 data will be
//      compacted.
pub const BASE_LEVEL: &str = concat!(ROCKSDB_PREFIX!(), "base-level");

//  "rocksdb.estimate-pending-compaction-bytes" - returns estimated total
//      number of bytes compaction needs to rewrite to get all levels down
//      to under target size. Not valid for other compactions than level-
//      based.
pub const ESTIMATE_PENDING_COMPACTION_BYTES: &str =
    concat!(ROCKSDB_PREFIX!(), "estimate-pending-compaction-bytes");

//  "rocksdb.aggregated-table-properties" - returns a string representation
//      of the aggregated table properties of the target column family.
pub const AGGREGATED_TABLE_PROPERTIES: &str =
    concat!(ROCKSDB_PREFIX!(), "aggregated-table-properties");

//  "rocksdb.aggregated-table-properties-at-level<N>", same as the previous
//      one but only returns the aggregated table properties of the
//      specified level "N" at the target column family.
pub const AGGREGATED_TABLE_PROPERTIES_AT_LEVEL: &str =
    concat!(ROCKSDB_PREFIX!(), "aggregated-table-properties-at-level");

//  "rocksdb.actual-delayed-write-rate" - returns the current actual delayed
//      write rate. 0 means no delay.
pub const ACTUAL_DELAYED_WRITE_RATE: &str = concat!(ROCKSDB_PREFIX!(), "actual-delayed-write-rate");

//  "rocksdb.is-write-stopped" - Return 1 if write has been stopped.
pub const IS_WRITE_STOPPED: &str = concat!(ROCKSDB_PREFIX!(), "is-write-stopped");

//  "rocksdb.estimate-oldest-key-time" - returns an estimation of
//      oldest key timestamp in the DB. Currently only available for
//      FIFO compaction with
//      compaction_options_fifo.allow_compaction = false.
pub const ESTIMATE_OLDEST_KEY_TIME: &str = concat!(ROCKSDB_PREFIX!(), "estimate-oldest-key-time");

//  "rocksdb.block-cache-capacity" - returns block cache capacity.
pub const BLOCK_CACHE_CAPACITY: &str = concat!(ROCKSDB_PREFIX!(), "block-cache-capacity");

//  "rocksdb.block-cache-usage" - returns the memory size for the entries
//      residing in block cache.
pub const BLOCK_CACHE_USAGE: &str = concat!(ROCKSDB_PREFIX!(), "block-cache-usage");

// "rocksdb.block-cache-pinned-usage" - returns the memory size for the
//      entries being pinned.
pub const BLOCK_CACHE_PINNED_USAGE: &str = concat!(ROCKSDB_PREFIX!(), "block-cache-pinned-usage");

// "rocksdb.options-statistics" - returns multi-line string
//      of options.statistics
pub const OPTIONS_STATISTICS: &str = concat!(ROCKSDB_PREFIX!(), "options-statistics");
