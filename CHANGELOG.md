# Changelog

## [Unreleased]

* Bump MSRV to 1.63.0 (mina86)
* Convert properties to `&PropName` which can be converted at no cost to `&CStr`
  and `&str` (mina86)
* Add WriteBufferManager support (benoitmeriaux)

## 0.21.0 (2023-05-09)

* Add doc-check to CI with fix warnings in docs (YuraKotov)
* Fix rustdoc::broken-intra-doc-links errors (YuraKotov)
* Fix 32-bit ARM build (EyeOfPython)
* Allow specifying checksum type (romanz)
* Enable librocksdb-sys to be built by rustc_codegen_cranelift (ZePedroResende)
* Update to RocksDB 8.0.0 (niklasf)
* Block cache creation failure is not recoverable (niklasf)
* Update iOS min version to 12 in the build script (mighty840)
* Actually enable `io-uring` (niklasf)
* Update to RocksDB 8.1.1 (niklasf)
* Add `Cache::new_hyper_clock_cache()` (niklasf)
* Retrieve Value from KeyMayExist if value found in Cache or Memory (Congyuwang)
* Support for comparators as closures (pegesund)
* Fix bug in DBWALIterator that would miss updates (Zagitta)

## 0.20.1 (2023-02-10)

* Fix supporting MSRV 1.60.0 (aleksuss)

## 0.20.0 (2023-02-09)

* Support RocksDB 7.x `BackupEngineOptions` (exabytes18)
* Fix `int128` compatibility check (Dirreke)
* Add `Options::load_latest` method to load the latest options from RockDB (Congyuwang)
* Bump bindgen to 0.64.0 (cwlittle)
* Bump rocksdb to 7.9.2 (kwek20)
* Make `set_snapshot` method public (a14e)
* Add `drop_cf` function to `TransactionDB` (bothra90)
* Bump rocksdb to 7.8.3 (aleksuss)
* Add doc for `set_cache_index_and_filter_blocks` (guerinoni)
* Re-run `build.rs` if env vars change (drahnr)
* Add `WriteBatch::data` method (w41ter)
* Add `DB::open_cf_with_opts` method (w41ter)
* Use lz4-sys crate rather then submodule (niklasf)
* Make create_new_backup_flush generic (minshao)

## 0.19.0 (2022-08-05)

* Add support for building with `io_uring` on Linux (parazyd)
* Change iterators to return Result (mina86)
* Support RocksDB transaction (yiyuanliu)
* Avoid pulling in dependencies via static feature flag (niklasf)
* Bump `rocksdb` to 7.4.4 (niklasf)
* Bump `tikv-jemalloc-sys` to 0.5 (niklasf)
* Update `set_use_fsync` comment (nazar-pc)
* Introduce ReadOptions::set_iterate_range and PrefixRange (mina86)
* Bump `rocksdb` to 7.4.3 (aleksuss)
* Don’t hold onto ReadOptions.inner when iterating (mina86)
* Bump `zstd-sys` from 1.6 to 2.0 (slightknack)
* Enable a building on the iOS platform (dignifiedquire)
* Add DBRawIteratorWithThreadMode::item method (mina86)
* Use NonNull in DBRawIteratorWithThreadMode (mina86)
* Tiny refactoring including fix for UB (niklasf)
* Add batched version MultiGet API (yhchiang-sol)
* Upgrade to rocksdb v7.3.1 (yhchiang-sol)
* Consistently use `ffi_util::to_cpath` to convert `Path` to `CString` (mina86)
* Convert properties to `&CStr` (mina86)
* Allow passing `&CStr` arguments (mina86)
* Fix memory leak when reading properties and avoid memory allocation (mina86)
* Fix Windows UTF-8 build flag (rajivshah3)
* Use more target features to build librocksdb-sys (niklasf)
* Fix `bz_internal_error` symbol multiply defined (nanpuyue)
* Bump rocksdb to 7.1.2 (dignifiedquire)
* Add BlobDB options (dignifiedquire)
* Add snapshot `PinnableSlice` based API (zheland)

## 0.18.0 (2022-02-03)

* Add open_cf_descriptor methods for Secondary and ReadOnly AccessType (steviez)
* Make Ribbon filters available (niklasf)
* Change versioning scheme of `librocksdb-sys` crate (aleksuss)
* Upgrade to RocksDB 6.28.2 (akrylysov)
* Fix theoretical UB while transmuting Arc (niklasf)
* Support configuring bottom-most compression level (mina86)
* Add BlockBasedOptions::set_whole_key_filtering (niklasf)
* Add constants for all supported properties (steviez)
* Make CacheWrapper and EnvWrapper Send and Sync (aleksuss)
* Replace mem::transmute with narrower conversions (niklasf)
* Optimize non-overlapping copy in raw_data (niklasf)
* Support multi_get_* methods (olegnn)
* Optimize multi_get_cf_opt() to use size hint (niklasf)
* Fix typo in set_background_purge_on_iterator_cleanup method (Congyuwang)
* Use external compression crates where possible (Dr-Emann)
* Update compression dependencies (akrylysov)
* Add method for opening DB with ro access and cf descriptors (nikurt)
* Support restoring from a specified backup (GoldenLeaves)
* Add merge operands iterator (0xdeafbeef)
* Derive serde::{Serialize, Deserialize} for configuration enums (thibault-martinez)
* Add feature flag for runtime type information and metadata (jgraettinger)
* Add set_info_log_level to control log verbosity (tkintscher)
* Replace jemalloc-sys for tikv-jemalloc-sys (Rexagon)
* Support UTF-8 file paths on Windows (rajivshah3)
* Support building RocksDB with jemalloc (akrylysov)
* Add rocksdb WAL flush api (duarten)
* Update rocksdb to v6.22.1 (duarten)

## 0.17.0 (2021-07-22)

* Fix `multi_get` method (mikhailOK)
* Bump `librocksdb-sys` up to 6.19.3 (olegnn)
* Add support for the cuckoo table format (rbost)
* RocksDB is not compiled with SSE4 instructions anymore unless the corresponding features are enabled in rustc (mbargull)
* Bump `librocksdb-sys` up to 6.20.3 (olegnn, akrylysov)
* Add `DB::key_may_exist_cf_opt` method (stanislav-tkach)
* Add `Options::set_zstd_max_train_bytes` method (stanislav-tkach)
* Mark Cache and Env as Send and Sync (akrylysov)
* Allow cloning the Cache and Env (duarten)
* Make SSE inclusion conditional for target features (mbargull)
* Use Self where possible (adamnemecek)
* Don't leak dropped column families (ryoqun)

## 0.16.0 (2021-04-18)

* Add `DB::cancel_all_background_work` method (stanislav-tkach)
* Bump `librocksdb-sys` up to 6.13.3 (aleksuss)
* Add `multi_get`, `multi_get_opt`, `multi_get_cf` and `multi_get_cf_opt` `DB` methods (stanislav-tkach)
* Allow setting options on a ColumnFamily (romanz)
* Fix logic related to merge operator settings (BoOTheFurious)
* Export persist_period_sec option and background_threads (developerfred)
* Remove unneeded bindgen features (Kixunil)
* Add merge delete_callback omitted by mistake (zhangsoledad)
* Bump `librocksdb-sys` up to 6.17.3 (ordian)
* Remove the need for `&mut self` in `create_cf` and `drop_cf` (v2) (ryoqun)
* Keep Cache and Env alive with Rc (acrrd)
* Add `DB::open_cf_with_ttl` method (fdeantoni)

## 0.15.0 (2020-08-25)

* Fix building rocksdb library on windows host (aleksuss)
* Add github actions CI for windows build (aleksuss)
* Update doc for `Options::set_compression_type` (wqfish)
* Add clippy linter in CI (aleksuss)
* Use DBPath for backup_restore test (wqfish)
* Allow to build RocksDB with a different stdlib (calavera)
* Add some doc-comments and tiny refactoring (aleksuss)
* Expose `open_with_ttl`. (calavera)
* Fixed build for `x86_64-linux-android` that doesn't support PCLMUL (vimmerru)
* Add support for `SstFileWriter` and `DB::ingest_external_file` (methyl)
* Add set_max_log_file_size and set_recycle_log_file_num to the Options (stanislav-tkach)
* Export the `DEFAULT_COLUMN_FAMILY_NAME` constant (stanislav-tkach)
* Fix slice transformers with no in_domain callback (nelhage)
* Don't segfault on failed a merge operator (nelhage)
* Adding read/write/db/compaction options (linxGnu)
* Add dbpath and env options (linxGnu)
* Add compaction filter factory API (unrealhoang)
* Add link stdlib when linking prebuilt rocksdb (unrealhoang)
* Support fetching sst files metadata, delete files in range, get mem usage (linxGnu)
* Do not set rerun-if-changed=build.rs (xu-cheng)
* Use pretty_assertions in tests (stanislav-tkach)
* librocksdb-sys: update rocksdb to 6.11.4 (ordian)
* Adding backup engine info (linxGnu)
* Implement `Clone` trait for `Options` (stanislav-tkach)
* Added `Send` implementation to `WriteBatch` (stanislav-tkach)
* Extend github actions (stanislav-tkach)
* Avoid copy for merge operator result using delete_callback (xuchen-plus)

## 0.14.0 (2020-04-22)

* Updated lz4 to v1.9.2 (ordian)
* BlockBasedOptions: expose `format_version`, `[index_]block_restart_interval` (ordian)
* Improve `ffi_try` macro to make trailing comma optional (wqfish)
* Add `set_ratelimiter` to the `Options` (PatrickNicholas)
* Add `set_max_total_wal_size` to the `Options` (wqfish)
* Simplify conversion on iterator item (zhangsoledad)
* Add `flush_cf` method to the `DB` (wqfish)
* Fix potential segfault when calling `next` on the `DBIterator` that is at the end of the range (wqfish)
* Move to Rust 2018 (wqfish)
* Fix doc for `WriteBatch::delete` (wqfish)
* Bump `uuid` and `bindgen` dependencies (jonhoo)
* Change APIs that never return error to not return `Result` (wqfish)
* Fix lifetime parameter for iterators (wqfish)
* Add a doc for `optimize_level_style_compaction` method (NikVolf)
* Make `DBPath` use `tempfile` (jder)
* Refactor `db.rs` and `lib.rs` into smaller pieces (jder)
* Check if we're on a big endian system and act upon it (knarz)
* Bump internal snappy version up to 1.1.8 (aleksuss)
* Bump rocksdb version up to 6.7.3 (aleksuss)
* Atomic flush option (mappum)
* Make `set_iterate_upper_bound` method safe (wqfish)
* Add support for data block hash index (dvdplm)
* Add some extra config options (casualjim)
* Add support for range delete APIs (wqfish)
* Improve building `librocksdb-sys` with system libraries (basvandijk)
* Add support for `open_for_read_only` APIs (wqfish)
* Fix doc for `DBRawIterator::prev` and `next` methods (wqfish)
* Add support for `open_as_secondary` APIs (calavera)

## 0.13.0 (2019-11-12)

### Changes

* Added `ReadOptions::set_verify_checksums` and
  `Options::set_level_compaction_dynamic_level_bytes` methods (ordian)
* Array of bytes has been changed for pinnable slice for get operations (nbdd0121)
* Implemented `Sync` for `DBRawIterator` (nbdd0121)
* Removed extra copy in DBRawIterator (nbdd0121)
* Added `Options::max_dict_bytes` and `Options::zstd_max_training_bytes` methods(methyl)
* Added Android support (rtsisyk)
* Added lifetimes for `DBIterator` return types (ngotchac)
* Bumped rocksdb up to 6.2.4 (aleksuss)
* Disabled trait derivation for librocksdb-sys (EyeOfPython)
* Added `DB::get_updates_since()` to iterate write batches in a given sequence (nlfiedler)
* Added `ReadOptions::set_tailing()` to create a tailing iterator that continues to
  iterate over the database as new records are added (cjbradfield)
* Changed column families storing (aleksuss)
* Exposed the `status` method on iterators (rnarubin)

## 0.12.3 (2019-07-19)

### Changes

* Enabled sse4.2/pclmul for accelerated crc32c (yjh0502)
* Added `set_db_write_buffer_size` to the Options API (rnarubin)
* Bumped RocksDB to 6.1.2 (lispy)
* Added `Sync` and `Send` implementations to `Snapshot` (pavel-mukhanov)
* Added `raw_iterator_cf_opt` to the DB API (rnarubin)
* Added `DB::latest_sequence_number` method (vitvakatu)

## 0.12.2 (2019-05-03)

### Changes

* Updated `compact_range_cf` to use generic arguments (romanz)
* Removed allocations from `SliceTransform` implementation (ekmartin)
* Bumped RocksDB to 5.18.3 (baptistejamin)
* Implemented `delete_range` and `delete_range_cf` (baptistejamin)
* Added contribution guide (rhurkes)
* Cleaned up documentation for `ReadOptions.set_iterate_upper_bound` method (xiaobogaga)
* Added `flush` and `flush_opt` operations (valeriansaliou)

## 0.12.1 (2019-03-27)

### Changes

* Added `iterator_cf_opt` function to `DB` (elichai)
* Added `set_allow_mmap_writes` and `set_allow_mmap_reads` functions to `Options` (aleksuss)


## 0.12.0 (2019-03-10)

### Changes

* Added support for PlainTable factories (ekmartin)
* Added ability to restore latest backup (rohitjoshi)
* Added support for pinnable slices (xxuejie)
* Added ability to get property values (ekmartin)
* Simplified opening database when using non-default column families (iSynaptic)
* `ColumnFamily`, `DBIterator` and `DBRawIterator` now have lifetime parameters to prevent using them after the `DB` has been dropped (iSynaptic)
* Creating `DBIterator` and `DBRawIterator` now accept `ReadOptions` (iSynaptic)
* All database operations that accepted byte slices, `&[u8]`, are now generic and accept anything that implements `AsRef<[u8]>` (iSynaptic)
* Bumped RocksDB to version 5.17.2 (aleksuss)
* Added `set_readahead_size` to `ReadOptions` (iSynaptic)
* Updated main example in doc tests (mohanson)
* Updated requirements documentation (jamesray1)
* Implemented `AsRef<[u8]>` for `DBVector` (iSynaptic)


## 0.11.0 (2019-01-10)

### Announcements

* This is the first release under the new [Maintainership](MAINTAINERSHIP.md) model.
  Three contributors have been selected to help maintain this library -- Oleksandr Anyshchenko ([@aleksuss](https://github.com/aleksuss)), Jordan Terrell ([@iSynaptic](https://github.com/iSynaptic)), and Ilya Bogdanov ([@vitvakatu](https://github.com/vitvakatu)). Many thanks to Tyler Neely ([@spacejam](https://github.com/spacejam)) for your support while taking on this new role.

* A [gitter.im chat room](https://gitter.im/rust-rocksdb/Lobby) has been created. Although it's not guaranteed to be "staffed", it may help to collaborate on changes to `rust-rocksdb`.

### Changes

* added LZ4, ZSTD, ZLIB, and BZIP2 compression support (iSynaptic)
* added support for `Checkpoint` (aleksuss)
* added support for `SliceTransform` (spacejam)
* added `DBPath` struct to ensure test databases are cleaned up (ekmartin, iSynaptic)
* fixed `rustfmt.toml` to work with newer `rustfmt` version (ekmartin, iSynaptic)
* bindgen bumped up to 0.43 (s-panferov)
* made `ColumnFamily` struct `Send` (Tpt)
* made `DBIterator` struct `Send` (Elzor)
* `create_cf` and `drop_cf` methods on `DB` now work with immutable references (aleksuss)
* fixed crash in `test_column_family` test on macOS (aleksuss)
* fixed/implemented CI builds for macOS and Windows (aleksuss, iSynaptic)
* exposed `set_skip_stats_update_on_db_open` option (romanz)
* exposed `keep_log_file_num` option (romanz)
* added ability to retrieve `WriteBatch` serialized size (romanz)
* added `set_options` method to `DB` to allow changing options without closing and re-opening the database (romanz)


## 0.10.1 (2018-07-17)

* bump bindgen to 0.37 (ekmartin)
* bump rocksdb to 5.14.2 (ekmartin)
* add disable_cache to block-based options (ekmartin)
* add set_wal_dir (ekmartin)
* add set_memtable_prefix_bloom_ratio (ekmartin)
* add MemtableFactory support (ekmartin)
* add full_iterator (ekmartin)
* allow index type specification on block options (ekmartin)
* fix windows build (iSynaptic)

## 0.10.0 (2018-03-17)

* Bump rocksdb to 5.11.3 (spacejam)

### New Features

* Link with system rocksdb and snappy libs through envvars (ozkriff)

### Breaking Changes

* Fix reverse iteration from a given key (ongardie)

## 0.9.1 (2018-02-10)

### New Features

* SliceTransform support (spacejam)

## 0.9.0 (2018-02-10)

### New Features

* Allow creating iterators over prefixes (glittershark)

### Breaking Changes

* Open cfs with options (garyttierney, rrichardson)
* Non-Associative merge ops (rrichardson)

## 0.8.3 (2018-02-10)

* Bump rocksdb to 5.10.2 (ongardie)
* Add Send marker to Options (iSynaptic)
* Expose advise_random_on_open option (ongardie)

## 0.8.2 (2017-12-28)

* Bump rocksdb to 5.7.1 (jquesnelle)

## 0.8.1 (2017-09-08)

* Added list_cf (jeizsm)

## 0.8.0 (2017-09-02)

* Removed set_disable_data_sync (glittershark)

## 0.7.2 (2017-09-02)

* Bumped rocksdb to 5.6.2 (spacejam)

## 0.7.1 (2017-08-29)

* Bumped rocksdb to 5.6.1 (vmx)

## 0.7 (2017-07-26)

### Breaking Changes

* Bumped rocksdb to 5.4.6 (derekdreery)
* Remove `use_direct_writes` now that `use_direct_io_for_flush_and_compaction` exists (derekdreery)

### New Features

* ReadOptions is now public (rschmukler)
* Implement Clone and AsRef<str> for Error (daboross)
* Support for `seek_for_prev` (kaedroho)
* Support for DirectIO (kaedroho)

### Internal Cleanups

* Fixed race condition in tests (debris)
* Move tests to the default `tests` directory (vmx)

## 0.6.1 (2017-03-13)

### New Features

* Support for raw iterator access (kaedroho)

## 0.6 (2016-12-18)

### Breaking Changes

* Comparator function now returns an Ordering (alexreg)

### New Features

* Compaction filter (tmccombs)
* Support for backups (alexreg)

0.5 (2016-11-20)

### Breaking changes

* No more Writable trait, as WriteBatch is not thread-safe as a DB (spacejam)
* All imports of `rocksdb::rocksdb::*` should now be simply `rocksdb::*` (alexreg)
* All errors changed to use a new `rocksdb::Error` type (kaedroho, alexreg)
* Removed `Options.set_filter_deletes` as it was removed in RocksDB (kaedroho)
* Renamed `add_merge_operator` to `set_merge_operator` and `add_comparator` to `set_comparator` (kaedroho)

### New Features

* Windows support (development by jsgf and arkpar. ported by kaedroho)
* The RocksDB library is now built at crate compile-time and statically linked with the resulting binary (development by jsgf and arkpar. ported by kaedroho)
* Cleaned up and improved coverage and tests of the ffi module (alexreg)
* Added many new methods to the `Options` type (development by ngaut, BusyJay, zhangjinpeng1987, siddontang and hhkbp2. ported by kaedroho)
* Added `len` and `is_empty` methods to `WriteBatch` (development by siddontang. ported by kaedroho)
* Added `path` mathod to `DB` (development by siddontang. ported by kaedroho)
* `DB::open` now accepts any type that implements `Into<Path>` as the path argument (kaedroho)
* `DB` now implements the `Debug` trait (kaedroho)
* Add iterator_cf to snapshot (jezell)
* Changelog started
