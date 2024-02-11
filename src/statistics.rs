use crate::ffi;

#[derive(Debug, Clone)]
pub struct NameParseError;
impl core::fmt::Display for NameParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unrecognized name")
    }
}

impl std::error::Error for NameParseError {}

// Helper macro to generate iterable nums that translate into static strings mapped from the cpp
// land.
macro_rules! iterable_named_enum {
    (
    $(#[$m:meta])*
    $type_vis:vis enum $typename:ident {
        $(
            $(#[$variant_meta:meta])*
            $variant:ident($variant_str:literal) $(= $value:expr)?,
        )+
    }
    ) => {
        // Main Type
        #[allow(clippy::all)]
        $(#[$m])*
        $type_vis enum $typename {
            $(
            $(#[$variant_meta])*
            $variant$( = $value)?,
            )+
        }

        #[automatically_derived]
        impl $typename {
            #[doc = "The corresponding rocksdb string identifier for this variant"]
            pub const fn name(&self) -> &'static str {
                match self {
                    $(
                        $typename::$variant => $variant_str,
                    )+
                }
            }
            pub fn iter() -> ::core::slice::Iter<'static, $typename> {
                static VARIANTS: &'static [$typename] = &[
                    $(
                        $typename::$variant,
                    )+
                ];
                VARIANTS.iter()
            }
        }


        #[automatically_derived]
        impl ::core::str::FromStr for $typename {
            type Err = NameParseError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $(
                        $variant_str => Ok($typename::$variant),
                    )+
                    _ => Err(NameParseError),
                }
            }
        }

        #[automatically_derived]
        impl ::core::fmt::Display for $typename {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                self.name().fmt(f)
            }
        }
    };
}

/// StatsLevel can be used to reduce statistics overhead by skipping certain
/// types of stats in the stats collection process.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum StatsLevel {
    /// Disable all metrics
    DisableAll = 0,
    /// Disable timer stats, and skip histogram stats
    ExceptHistogramOrTimers = 2,
    /// Skip timer stats
    ExceptTimers,
    /// Collect all stats except time inside mutex lock AND time spent on
    /// compression.
    ExceptDetailedTimers,
    /// Collect all stats except the counters requiring to get time inside the
    /// mutex lock.
    ExceptTimeForMutex,
    /// Collect all stats, including measuring duration of mutex operations.
    /// If getting time is expensive on the platform to run, it can
    /// reduce scalability to more threads, especially for writes.
    All,
}

// Keep in-sync with rocksdb/include/rocksdb/statistics.h
iterable_named_enum! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    #[repr(u32)]
    pub enum Ticker {
        /// total block cache misses
        /// REQUIRES: BlockCacheMiss == BlockCacheIndexMiss +
        ///                               BlockCacheFilterMiss +
        ///                               BlockCacheDataMiss;
        BlockCacheMiss("rocksdb.block.cache.miss") = 0,
        /// total block cache hit
        /// REQUIRES: BlockCacheHit == BlockCacheIndexHit +
        ///                              BlockCacheFilterHit +
        ///                              BlockCacheDataHit;
        BlockCacheHit("rocksdb.block.cache.hit"),
        /// # of blocks added to block cache.
        BlockCacheAdd("rocksdb.block.cache.add"),
        /// # of failures when adding blocks to block cache.
        BlockCacheAddFailures("rocksdb.block.cache.add.failures"),
        /// # of times cache miss when accessing index block from block cache.
        BlockCacheIndexMiss("rocksdb.block.cache.index.miss"),
        /// # of times cache hit when accessing index block from block cache.
        BlockCacheIndexHit("rocksdb.block.cache.index.hit"),
        /// # of index blocks added to block cache.
        BlockCacheIndexAdd("rocksdb.block.cache.index.add"),
        /// # of bytes of index blocks inserted into cache
        BlockCacheIndexBytesInsert("rocksdb.block.cache.index.bytes.insert"),
        /// # of times cache miss when accessing filter block from block cache.
        BlockCacheFilterMiss("rocksdb.block.cache.filter.miss"),
        /// # of times cache hit when accessing filter block from block cache.
        BlockCacheFilterHit("rocksdb.block.cache.filter.hit"),
        /// # of filter blocks added to block cache.
        BlockCacheFilterAdd("rocksdb.block.cache.filter.add"),
        /// # of bytes of bloom filter blocks inserted into cache
        BlockCacheFilterBytesInsert("rocksdb.block.cache.filter.bytes.insert"),
        /// # of times cache miss when accessing data block from block cache.
        BlockCacheDataMiss("rocksdb.block.cache.data.miss"),
        /// # of times cache hit when accessing data block from block cache.
        BlockCacheDataHit("rocksdb.block.cache.data.hit"),
        /// # of data blocks added to block cache.
        BlockCacheDataAdd("rocksdb.block.cache.data.add"),
        /// # of bytes of data blocks inserted into cache
        BlockCacheDataBytesInsert("rocksdb.block.cache.data.bytes.insert"),
        /// # of bytes read from cache.
        BlockCacheBytesRead("rocksdb.block.cache.bytes.read"),
        /// # of bytes written into cache.
        BlockCacheBytesWrite("rocksdb.block.cache.bytes.write"),

        /// # of times bloom filter has avoided file reads, i.e., negatives.
        BloomFilterUseful("rocksdb.bloom.filter.useful"),
        /// # of times bloom FullFilter has not avoided the reads.
        BloomFilterFullPositive("rocksdb.bloom.filter.full.positive"),
        /// # of times bloom FullFilter has not avoided the reads and data actually
        /// exist.
        BloomFilterFullTruePositive("rocksdb.bloom.filter.full.true.positive"),

        /// # persistent cache hit
        PersistentCacheHit("rocksdb.persistent.cache.hit"),
        /// # persistent cache miss
        PersistentCacheMiss("rocksdb.persistent.cache.miss"),

        /// # total simulation block cache hits
        SimBlockCacheHit("rocksdb.sim.block.cache.hit"),
        /// # total simulation block cache misses
        SimBlockCacheMiss("rocksdb.sim.block.cache.miss"),

        /// # of memtable hits.
        MemtableHit("rocksdb.memtable.hit"),
        /// # of memtable misses.
        MemtableMiss("rocksdb.memtable.miss"),

        /// # of Get() queries served by L0
        GetHitL0("rocksdb.l0.hit"),
        /// # of Get() queries served by L1
        GetHitL1("rocksdb.l1.hit"),
        /// # of Get() queries served by L2 and up
        GetHitL2AndUp("rocksdb.l2andup.hit"),

        /**
         * Compaction_KeyDrop* count the reasons for key drop during compaction
         * There are 4 reasons currently.
         */
        CompactionKeyDropNewerEntry("rocksdb.compaction.key.drop.new"),
        /// key was written with a newer value.
        /// Also includes keys dropped for range del.
        CompactionKeyDropObsolete("rocksdb.compaction.key.drop.obsolete"),
        /// The key is obsolete.
        CompactionKeyDropRangeDel("rocksdb.compaction.key.drop.range_del"),
        /// key was covered by a range tombstone.
        CompactionKeyDropUser("rocksdb.compaction.key.drop.user"),
        /// user compaction function has dropped the key.
        CompactionRangeDelDropObsolete("rocksdb.compaction.range_del.drop.obsolete"),
        /// all keys in range were deleted.
        /// Deletions obsoleted before bottom level due to file gap optimization.
        CompactionOptimizedDelDropObsolete("rocksdb.compaction.optimized.del.drop.obsolete"),
        /// If a compaction was canceled in sfm to prevent ENOSPC
        CompactionCancelled("rocksdb.compaction.cancelled"),

        /// Number of keys written to the database via the Put and Write call's
        NumberKeysWritten("rocksdb.number.keys.written"),
        /// Number of Keys read,
        NumberKeysRead("rocksdb.number.keys.read"),
        /// Number keys updated, if inplace update is enabled
        NumberKeysUpdated("rocksdb.number.keys.updated"),
        /// The number of uncompressed bytes issued by DB::Put(), DB::Delete(),
        /// DB::Merge(), and DB::Write().
        BytesWritten("rocksdb.bytes.written"),
        /// The number of uncompressed bytes read from DB::Get().  It could be
        /// either from memtables, cache, or table files.
        /// For the number of logical bytes read from DB::MultiGet(),
        /// please use NumberMultigetBytesRead.
        BytesRead("rocksdb.bytes.read"),
        /// The number of calls to seek/next/prev
        NumberDbSeek("rocksdb.number.db.seek"),
        NumberDbNext("rocksdb.number.db.next"),
        NumberDbPrev("rocksdb.number.db.prev"),
        /// The number of calls to seek/next/prev that returned data
        NumberDbSeekFound("rocksdb.number.db.seek.found"),
        NumberDbNextFound("rocksdb.number.db.next.found"),
        NumberDbPrevFound("rocksdb.number.db.prev.found"),
        /// The number of uncompressed bytes read from an iterator.
        /// Includes size of key and value.
        IterBytesRead("rocksdb.db.iter.bytes.read"),
        NoFileOpens("rocksdb.no.file.opens"),
        NoFileErrors("rocksdb.no.file.errors"),
        /// Writer has to wait for compaction or flush to finish.
        StallMicros("rocksdb.stall.micros"),
        /// The wait time for db mutex.
        /// Disabled by default. To enable it set stats level to kAll
        DbMutexWaitMicros("rocksdb.db.mutex.wait.micros"),

        /// Number of MultiGet calls, keys read, and bytes read
        NumberMultigetCalls("rocksdb.number.multiget.get"),
        NumberMultigetKeysRead("rocksdb.number.multiget.keys.read"),
        NumberMultigetBytesRead("rocksdb.number.multiget.bytes.read"),

        NumberMergeFailures("rocksdb.number.merge.failures"),

        /// Prefix filter stats when used for point lookups (Get / MultiGet).
        /// (For prefix filter stats on iterators, see *_LEVEL_Seek_*.)
        /// Checked: filter was queried
        BloomFilterPrefixChecked("rocksdb.bloom.filter.prefix.checked"),
        /// Useful: filter returned false so prevented accessing data+index blocks
        BloomFilterPrefixUseful("rocksdb.bloom.filter.prefix.useful"),
        /// True positive: found a key matching the point query. When another key
        /// with the same prefix matches, it is considered a false positive by
        /// these statistics even though the filter returned a true positive.
        BloomFilterPrefixTruePositive("rocksdb.bloom.filter.prefix.true.positive"),

        /// Number of times we had to reseek inside an iteration to skip
        /// over large number of keys with same userkey.
        NumberOfReseeksInIteration("rocksdb.number.reseeks.iteration"),

        /// Record the number of calls to GetUpdatesSince. Useful to keep track of
        /// transaction log iterator refreshes
        GetUpdatesSinceCalls("rocksdb.getupdatessince.calls"),
        /// Number of times WAL sync is done
        WalFileSynced("rocksdb.wal.synced"),
        /// Number of bytes written to WAL
        WalFileBytes("rocksdb.wal.bytes"),

        /// Writes can be processed by requesting thread or by the thread at the
        /// head of the writers queue.
        WriteDoneBySelf("rocksdb.write.self"),
        WriteDoneByOther("rocksdb.write.other"),
        /// Equivalent to writes done for others
        WriteWithWal("rocksdb.write.wal"),
        /// Number of Write calls that request WAL
        CompactReadBytes("rocksdb.compact.read.bytes"),
        /// Bytes read during compaction
        CompactWriteBytes("rocksdb.compact.write.bytes"),
        /// Bytes written during compaction
        FlushWriteBytes("rocksdb.flush.write.bytes"),
        /// Bytes written during flush

        /// Compaction read and write statistics broken down by CompactionReason
        CompactReadBytesMarked("rocksdb.compact.read.marked.bytes"),
        CompactReadBytesPeriodic("rocksdb.compact.read.periodic.bytes"),
        CompactReadBytesTtl("rocksdb.compact.read.ttl.bytes"),
        CompactWriteBytesMarked("rocksdb.compact.write.marked.bytes"),
        CompactWriteBytesPeriodic("rocksdb.compact.write.periodic.bytes"),
        CompactWriteBytesTtl("rocksdb.compact.write.ttl.bytes"),

        /// Number of table's properties loaded directly from file, without creating
        /// table reader object.
        NumberDirectLoadTableProperties("rocksdb.number.direct.load.table.properties"),
        NumberSuperversionAcquires("rocksdb.number.superversion_acquires"),
        NumberSuperversionReleases("rocksdb.number.superversion_releases"),
        NumberSuperversionCleanups("rocksdb.number.superversion_cleanups"),

        /// # of compressions/decompressions executed
        NumberBlockCompressed("rocksdb.number.block.compressed"),
        NumberBlockDecompressed("rocksdb.number.block.decompressed"),

        /// DEPRECATED / unused (see NumberBlockCompression_*)
        NumberBlockNotCompressed("rocksdb.number.block.not_compressed"),

        /// Tickers that record cumulative time.
        MergeOperationTotalTime("rocksdb.merge.operation.time.nanos"),
        FilterOperationTotalTime("rocksdb.filter.operation.time.nanos"),
        CompactionCpuTotalTime("rocksdb.compaction.total.time.cpu_micros"),

        /// Row cache.
        RowCacheHit("rocksdb.row.cache.hit"),
        RowCacheMiss("rocksdb.row.cache.miss"),

        /// Read amplification statistics.
        /// Read amplification can be calculated using this formula
        /// (ReadAMP_ToTAL_ReadBytes / Read_AMP_Estimate_UsefulBytes)
        //
        /// REQUIRES: ReadOptions::read_amp_bytes_per_bit to be enabled
        ReadAmpEstimateUsefulBytes("rocksdb.read.amp.estimate.useful.bytes"),
        /// Estimate of total bytes actually used.
        ReadAmpTotalReadBytes("rocksdb.read.amp.total.read.bytes"),
        /// Total size of loaded data blocks.

        /// Number of refill intervals where rate limiter's bytes are fully consumed.
        NumberRateLimiterDrains("rocksdb.number.rate_limiter.drains"),

        /// Number of internal keys skipped by Iterator
        NumberIterSkip("rocksdb.number.iter.skip"),

        /// BlobDB specific stats
        /// # of Put/PutTtl/PutUntil to BlobDB. Only applicable to legacy BlobDB.
        BlobDbNumPut("rocksdb.blobdb.num.put"),
        /// # of Write to BlobDB. Only applicable to legacy BlobDB.
        BlobDbNumWrite("rocksdb.blobdb.num.write"),
        /// # of Get to BlobDB. Only applicable to legacy BlobDB.
        BlobDbNumGet("rocksdb.blobdb.num.get"),
        /// # of MultiGet to BlobDB. Only applicable to legacy BlobDB.
        BlobDbNumMultiget("rocksdb.blobdb.num.multiget"),
        /// # of Seek/SeekToFirst/SeekToLast/SeekForPrev to BlobDB iterator. Only
        /// applicable to legacy BlobDB.
        BlobDbNumSeek("rocksdb.blobdb.num.seek"),
        /// # of Next to BlobDB iterator. Only applicable to legacy BlobDB.
        BlobDbNumNext("rocksdb.blobdb.num.next"),
        /// # of Prev to BlobDB iterator. Only applicable to legacy BlobDB.
        BlobDbNumPrev("rocksdb.blobdb.num.prev"),
        /// # of keys written to BlobDB. Only applicable to legacy BlobDB.
        BlobDbNumKeysWritten("rocksdb.blobdb.num.keys.written"),
        /// # of keys read from BlobDB. Only applicable to legacy BlobDB.
        BlobDbNumKeysRead("rocksdb.blobdb.num.keys.read"),
        /// # of bytes (key + value) written to BlobDB. Only applicable to legacy
        /// BlobDB.
        BlobDbBytesWritten("rocksdb.blobdb.bytes.written"),
        /// # of bytes (keys + value) read from BlobDB. Only applicable to legacy
        /// BlobDB.
        BlobDbBytesRead("rocksdb.blobdb.bytes.read"),
        /// # of keys written by BlobDB as non-Ttl inlined value. Only applicable to
        /// legacy BlobDB.
        BlobDbWriteInlined("rocksdb.blobdb.write.inlined"),
        /// # of keys written by BlobDB as Ttl inlined value. Only applicable to legacy
        /// BlobDB.
        BlobDbWriteInlinedTtl("rocksdb.blobdb.write.inlined.ttl"),
        /// # of keys written by BlobDB as non-Ttl blob value. Only applicable to
        /// legacy BlobDB.
        BlobDbWriteBlob("rocksdb.blobdb.write.blob"),
        /// # of keys written by BlobDB as Ttl blob value. Only applicable to legacy
        /// BlobDB.
        BlobDbWriteBlobTtl("rocksdb.blobdb.write.blob.ttl"),
        /// # of bytes written to blob file.
        BlobDbBlobFileBytesWritten("rocksdb.blobdb.blob.file.bytes.written"),
        /// # of bytes read from blob file.
        BlobDbBlobFileBytesRead("rocksdb.blobdb.blob.file.bytes.read"),
        /// # of times a blob files being synced.
        BlobDbBlobFileSynced("rocksdb.blobdb.blob.file.synced"),
        /// # of blob index evicted from base DB by BlobDB compaction filter because
        /// of expiration. Only applicable to legacy BlobDB.
        BlobDbBlobIndexExpiredCount("rocksdb.blobdb.blob.index.expired.count"),
        /// size of blob index evicted from base DB by BlobDB compaction filter
        /// because of expiration. Only applicable to legacy BlobDB.
        BlobDbBlobIndexExpiredSize("rocksdb.blobdb.blob.index.expired.size"),
        /// # of blob index evicted from base DB by BlobDB compaction filter because
        /// of corresponding file deleted. Only applicable to legacy BlobDB.
        BlobDbBlobIndexEvictedCount("rocksdb.blobdb.blob.index.evicted.count"),
        /// size of blob index evicted from base DB by BlobDB compaction filter
        /// because of corresponding file deleted. Only applicable to legacy BlobDB.
        BlobDbBlobIndexEvictedSize("rocksdb.blobdb.blob.index.evicted.size"),
        /// # of blob files that were obsoleted by garbage collection. Only applicable
        /// to legacy BlobDB.
        BlobDbGcNumFiles("rocksdb.blobdb.gc.num.files"),
        /// # of blob files generated by garbage collection. Only applicable to legacy
        /// BlobDB.
        BlobDbGcNumNewFiles("rocksdb.blobdb.gc.num.new.files"),
        /// # of BlobDB garbage collection failures. Only applicable to legacy BlobDB.
        BlobDbGcFailures("rocksdb.blobdb.gc.failures"),
        /// # of keys relocated to new blob file by garbage collection.
        BlobDbGcNumKeysRelocated("rocksdb.blobdb.gc.num.keys.relocated"),
        /// # of bytes relocated to new blob file by garbage collection.
        BlobDbGcBytesRelocated("rocksdb.blobdb.gc.bytes.relocated"),
        /// # of blob files evicted because of BlobDB is full. Only applicable to
        /// legacy BlobDB.
        BlobDbFifoNumFilesEvicted("rocksdb.blobdb.fifo.num.files.evicted"),
        /// # of keys in the blob files evicted because of BlobDB is full. Only
        /// applicable to legacy BlobDB.
        BlobDbFifoNumKeysEvicted("rocksdb.blobdb.fifo.num.keys.evicted"),
        /// # of bytes in the blob files evicted because of BlobDB is full. Only
        /// applicable to legacy BlobDB.
        BlobDbFifoBytesEvicted("rocksdb.blobdb.fifo.bytes.evicted"),

        /// These counters indicate a performance issue in WritePrepared transactions.
        /// We should not seem them ticking them much.
        /// # of times prepare_mutex_ is acquired in the fast path.
        TxnPrepareMutexOverhead("rocksdb.txn.overhead.mutex.prepare"),
        /// # of times old_commit_map_mutex_ is acquired in the fast path.
        TxnOldCommitMapMutexOverhead("rocksdb.txn.overhead.mutex.old.commit.map"),
        /// # of times we checked a batch for duplicate keys.
        TxnDuplicateKeyOverhead("rocksdb.txn.overhead.duplicate.key"),
        /// # of times snapshot_mutex_ is acquired in the fast path.
        TxnSnapshotMutexOverhead("rocksdb.txn.overhead.mutex.snapshot"),
        /// # of times ::Get returned TryAgain due to expired snapshot seq
        TxnGetTryAgain("rocksdb.txn.get.tryagain"),

        /// Number of keys actually found in MultiGet calls (vs number requested by
        /// caller)
        /// NumberMultigetKeys_Read gives the number requested by caller
        NumberMultigetKeysFound("rocksdb.number.multiget.keys.found"),

        NoIteratorCreated("rocksdb.num.iterator.created"),
        /// number of iterators created
        NoIteratorDeleted("rocksdb.num.iterator.deleted"),
        /// number of iterators deleted
        BlockCacheCompressionDictMiss("rocksdb.block.cache.compression.dict.miss"),
        BlockCacheCompressionDictHit("rocksdb.block.cache.compression.dict.hit"),
        BlockCacheCompressionDictAdd("rocksdb.block.cache.compression.dict.add"),
        BlockCacheCompressionDictBytesInsert("rocksdb.block.cache.compression.dict.bytes.insert"),

        /// # of blocks redundantly inserted into block cache.
        /// REQUIRES: BlockCacheAddRedundant <= BlockCacheAdd
        BlockCacheAddRedundant("rocksdb.block.cache.add.redundant"),
        /// # of index blocks redundantly inserted into block cache.
        /// REQUIRES: BlockCacheIndexAddRedundant <= BlockCacheIndexAdd
        BlockCacheIndexAddRedundant("rocksdb.block.cache.index.add.redundant"),
        /// # of filter blocks redundantly inserted into block cache.
        /// REQUIRES: BlockCacheFilterAddRedundant <= BlockCacheFilterAdd
        BlockCacheFilterAddRedundant("rocksdb.block.cache.filter.add.redundant"),
        /// # of data blocks redundantly inserted into block cache.
        /// REQUIRES: BlockCacheDataAddRedundant <= BlockCacheDataAdd
        BlockCacheDataAddRedundant("rocksdb.block.cache.data.add.redundant"),
        /// # of dict blocks redundantly inserted into block cache.
        /// REQUIRES: BlockCacheCompressionDictAddRedundant
        ///           <= BlockCacheCompressionDictAdd
        BlockCacheCompressionDictAddRedundant("rocksdb.block.cache.compression.dict.add.redundant"),

        /// # of files marked as trash by sst file manager and will be deleted
        /// later by background thread.
        FilesMarkedTrash("rocksdb.files.marked.trash"),
        /// # of trash files deleted by the background thread from the trash queue.
        FilesDeletedFromTrashQueue("rocksdb.files.marked.trash.deleted"),
        /// # of files deleted immediately by sst file manager through delete
        /// scheduler.
        FilesDeletedImmediately("rocksdb.files.deleted.immediately"),

        /// The counters for error handler, not that, bg_io_error is the subset of
        /// bg_error and bg_retryable_io_error is the subset of bg_io_error.
        /// The misspelled versions are deprecated and only kept for compatibility.
        /// ToDO: remove the misspelled tickers in the next major release.
        ErrorHandlerBgErrorCount("rocksdb.error.handler.bg.error.count"),
        ErrorHandlerBgErrorCountMisspelled("rocksdb.error.handler.bg.errro.count"),
        ErrorHandlerBgIoErrorCount("rocksdb.error.handler.bg.io.error.count"),
        ErrorHandlerBgIoErrorCountMisspelled("rocksdb.error.handler.bg.io.errro.count"),
        ErrorHandlerBgRetryableIoErrorCount("rocksdb.error.handler.bg.retryable.io.error.count"),
        ErrorHandlerBgRetryableIoErrorCountMisspelled("rocksdb.error.handler.bg.retryable.io.errro.count"),
        ErrorHandlerAutoResumeCount("rocksdb.error.handler.autoresume.count"),
        ErrorHandlerAutoResumeRetryTotalCount("rocksdb.error.handler.autoresume.retry.total.count"),
        ErrorHandlerAutoResumeSuccessCount("rocksdb.error.handler.autoresume.success.count"),

        /// Statistics for memtable garbage collection:
        /// Raw bytes of data (payload) present on memtable at flush time.
        MemtablePayloadBytesAtFlush("rocksdb.memtable.payload.bytes.at.flush"),
        /// Outdated bytes of data present on memtable at flush time.
        MemtableGarbageBytesAtFlush("rocksdb.memtable.garbage.bytes.at.flush"),

        /// Secondary cache statistics
        SecondaryCacheHits("rocksdb.secondary.cache.hits"),

        /// Bytes read by `VerifyChecksum()` and `VerifyFileChecksums()` APIs.
        VerifyChecksumReadBytes("rocksdb.verify_checksum.read.bytes"),

        /// Bytes read/written while creating backups
        BackupReadBytes("rocksdb.backup.read.bytes"),
        BackupWriteBytes("rocksdb.backup.write.bytes"),

        /// Remote compaction read/write statistics
        RemoteCompactReadBytes("rocksdb.remote.compact.read.bytes"),
        RemoteCompactWriteBytes("rocksdb.remote.compact.write.bytes"),

        /// Tiered storage related statistics
        HotFileReadBytes("rocksdb.hot.file.read.bytes"),
        WarmFileReadBytes("rocksdb.warm.file.read.bytes"),
        ColdFileReadBytes("rocksdb.cold.file.read.bytes"),
        HotFileReadCount("rocksdb.hot.file.read.count"),
        WarmFileReadCount("rocksdb.warm.file.read.count"),
        ColdFileReadCount("rocksdb.cold.file.read.count"),

        /// Last level and non-last level read statistics
        LastLevelReadBytes("rocksdb.last.level.read.bytes"),
        LastLevelReadCount("rocksdb.last.level.read.count"),
        NonLastLevelReadBytes("rocksdb.non.last.level.read.bytes"),
        NonLastLevelReadCount("rocksdb.non.last.level.read.count"),

        /// Statistics on iterator Seek() (and variants) for each sorted run. I.e. a
        /// single user Seek() can result in many sorted run Seek()s.
        /// The stats are split between last level and non-last level.
        /// Filtered: a filter such as prefix Bloom filter indicate the Seek() would
        /// not find anything relevant, so avoided a likely access to data+index
        /// blocks.
        LastLevelSeekFiltered("rocksdb.last.level.seek.filtered"),
        /// Filter match: a filter such as prefix Bloom filter was queried but did
        /// not filter out the seek.
        LastLevelSeekFilterMatch("rocksdb.last.level.seek.filter.match"),
        /// At least one data block was accessed for a Seek() (or variant) on a
        /// sorted run.
        LastLevelSeekData("rocksdb.last.level.seek.data"),
        /// At least one value() was accessed for the seek (suggesting it was useful),
        /// and no filter such as prefix Bloom was queried.
        LastLevelSeekDataUsefulNoFilter("rocksdb.last.level.seek.data.useful.no.filter"),
        /// At least one value() was accessed for the seek (suggesting it was useful),
        /// after querying a filter such as prefix Bloom.
        LastLevelSeekDataUsefulFilterMatch("rocksdb.last.level.seek.data.useful.filter.match"),
        /// The same set of stats, but for non-last level seeks.
        NonLastLevelSeekFiltered("rocksdb.non.last.level.seek.filtered"),
        NonLastLevelSeekFilterMatch("rocksdb.non.last.level.seek.filter.match"),
        NonLastLevelSeekData("rocksdb.non.last.level.seek.data"),
        NonLastLevelSeekDataUsefulNoFilter("rocksdb.non.last.level.seek.data.useful.no.filter"),
        NonLastLevelSeekDataUsefulFilterMatch("rocksdb.non.last.level.seek.data.useful.filter.match"),

        /// Number of block checksum verifications
        BlockChecksumComputeCount("rocksdb.block.checksum.compute.count"),
        /// Number of times RocksDB detected a corruption while verifying a block
        /// checksum. RocksDB does not remember corruptions that happened during user
        /// reads so the same block corruption may be detected multiple times.
        BlockChecksumMismatchCount("rocksdb.block.checksum.mismatch.count"),

        MultigetCoroutineCount("rocksdb.multiget.coroutine.count"),

        /// Integrated BlobDB specific stats
        /// # of times cache miss when accessing blob from blob cache.
        BlobDbCacheMiss("rocksdb.blobdb.cache.miss"),
        /// # of times cache hit when accessing blob from blob cache.
        BlobDbCacheHit("rocksdb.blobdb.cache.hit"),
        /// # of data blocks added to blob cache.
        BlobDbCacheAdd("rocksdb.blobdb.cache.add"),
        /// # of failures when adding blobs to blob cache.
        BlobDbCacheAddFailures("rocksdb.blobdb.cache.add.failures"),
        /// # of bytes read from blob cache.
        BlobDbCacheBytesRead("rocksdb.blobdb.cache.bytes.read"),
        /// # of bytes written into blob cache.
        BlobDbCacheBytesWrite("rocksdb.blobdb.cache.bytes.write"),

        /// Time spent in the ReadAsync file system call
        ReadAsyncMicros("rocksdb.read.async.micros"),
        /// Number of errors returned to the async read callback
        AsyncReadErrorCount("rocksdb.async.read.error.count"),

        /// Fine grained secondary cache stats
        SecondaryCacheFilterHits("rocksdb.secondary.cache.filter.hits"),
        SecondaryCacheIndexHits("rocksdb.secondary.cache.index.hits"),
        SecondaryCacheDataHits("rocksdb.secondary.cache.data.hits"),

        /// Number of lookup into the prefetched tail (see
        /// `TableOpenPrefetchTailReadBytes`)
        /// that can't find its data for table open
        TableOpenPrefetchTailMiss("rocksdb.table.open.prefetch.tail.miss"),
        /// Number of lookup into the prefetched tail (see
        /// `TableOpenPrefetchTailReadBytes`)
        /// that finds its data for table open
        TableOpenPrefetchTailHit("rocksdb.table.open.prefetch.tail.hit"),

        /// Statistics on the filtering by user-defined timestamps
        /// # of times timestamps are checked on accessing the table
        TimestampFilterTableChecked("rocksdb.timestamp.filter.table.checked"),
        /// # of times timestamps can successfully help skip the table access
        TimestampFilterTableFiltered("rocksdb.timestamp.filter.table.filtered"),

        /// Number of input bytes (uncompressed) to compression for SST blocks that
        /// are stored compressed.
        BytesCompressedFrom("rocksdb.bytes.compressed.from"),
        /// Number of output bytes (compressed) from compression for SST blocks that
        /// are stored compressed.
        BytesCompressedTo("rocksdb.bytes.compressed.to"),
        /// Number of uncompressed bytes for SST blocks that are stored uncompressed
        /// because compression type is kNoCompression, or some error case caused
        /// compression not to run or produce an output. Index blocks are only counted
        /// if enable_index_compression is true.
        BytesCompressionBypassed("rocksdb.bytes.compression_bypassed"),
        /// Number of input bytes (uncompressed) to compression for SST blocks that
        /// are stored uncompressed because the compression result was rejected,
        /// either because the ratio was not acceptable (see
        /// CompressionOptions::max_compressed_bytes_per_kb) or found invalid by the
        /// `verify_compression` option.
        BytesCompressionRejected("rocksdb.bytes.compression.rejected"),

        /// Like BytesCompressionBypassed but counting number of blocks
        NumberBlockCompressionBypassed("rocksdb.number.block_compression_bypassed"),
        /// Like BytesCompressionRejected but counting number of blocks
        NumberBlockCompressionRejected("rocksdb.number.block_compression_rejected"),

        /// Number of input bytes (compressed) to decompression in reading compressed
        /// SST blocks from storage.
        BytesDecompressedFrom("rocksdb.bytes.decompressed.from"),
        /// Number of output bytes (uncompressed) from decompression in reading
        /// compressed SST blocks from storage.
        BytesDecompressedTo("rocksdb.bytes.decompressed.to"),

        /// Number of times readahead is trimmed during scans when
        /// ReadOptions.auto_readahead_size is set.
        ReadAheadTrimmed("rocksdb.readahead.trimmed"),

        /// Number of Fifo compactions that drop files based on different reasons
        FifoMaxSizeCompactions("rocksdb.fifo.max.size.compactions"),
        FifoTtlCompactions("rocksdb.fifo.ttl.compactions"),

        /// Number of bytes prefetched during user initiated scan
        PrefetchBytes("rocksdb.prefetch.bytes"),

        /// Number of prefetched bytes that were actually useful
        PrefetchBytesUseful("rocksdb.prefetch.bytes.useful"),

        /// Number of FS reads avoided due to scan prefetching
        PrefetchHits("rocksdb.prefetch.hits"),

        /// Compressed secondary cache related stats
        CompressedSecondaryCacheDummyHits("rocksdb.compressed.secondary.cache.dummy.hits"),
        CompressedSecondaryCacheHits("rocksdb.compressed.secondary.cache.hits"),
        CompressedSecondaryCachePromotions("rocksdb.compressed.secondary.cache.promotions"),
        CompressedSecondaryCachePromotionSkips("rocksdb.compressed.secondary.cache.promotion.skips"),
    }
}

iterable_named_enum! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    #[repr(u32)]
    pub enum Histogram {
        DbGet("rocksdb.db.get.micros") = 0,
        DbWrite("rocksdb.db.write.micros"),
        CompactionTime("rocksdb.compaction.times.micros"),
        CompactionCpuTime("rocksdb.compaction.times.cpu_micros"),
        SubcompactionSetupTime("rocksdb.subcompaction.setup.times.micros"),
        TableSyncMicros("rocksdb.table.sync.micros"),
        CompactionOutfileSyncMicros("rocksdb.compaction.outfile.sync.micros"),
        WalFileSyncMicros("rocksdb.wal.file.sync.micros"),
        ManifestFileSyncMicros("rocksdb.manifest.file.sync.micros"),
        /// Time spent in IO during table open
        TableOpenIoMicros("rocksdb.table.open.io.micros"),
        DbMultiget("rocksdb.db.multiget.micros"),
        ReadBlockCompactionMicros("rocksdb.read.block.compaction.micros"),
        ReadBlockGetMicros("rocksdb.read.block.get.micros"),
        WriteRawBlockMicros("rocksdb.write.raw.block.micros"),
        NumFilesInSingleCompaction("rocksdb.numfiles.in.singlecompaction"),
        DbSeek("rocksdb.db.seek.micros"),
        WriteStall("rocksdb.db.write.stall"),
        /// Time spent in reading block-based or plain SST table
        SstReadMicros("rocksdb.sst.read.micros"),
        /// Time spent in reading SST table (currently only block-based table) or blob
        /// file corresponding to `Env::IOActivity`
        FileReadFlushMicros("rocksdb.file.read.flush.micros"),
        FileReadCompactionMicros("rocksdb.file.read.compaction.micros"),
        FileReadDbOpenMicros("rocksdb.file.read.db.open.micros"),
        /// The following `FILE_READ_*` require stats level greater than
        /// `StatsLevel::kExceptDetailedTimers`
        FileReadGetMicros("rocksdb.file.read.get.micros"),
        FileReadMultigetMicros("rocksdb.file.read.multiget.micros"),
        FileReadDbIteratorMicros("rocksdb.file.read.db.iterator.micros"),
        FileReadVerifyDbChecksumMicros("rocksdb.file.read.verify.db.checksum.micros"),
        FileReadVerifyFileChecksumsMicros("rocksdb.file.read.verify.file.checksums.micros"),
        /// The number of subcompactions actually scheduled during a compaction
        NumSubcompactionsScheduled("rocksdb.num.subcompactions.scheduled"),
        /// Value size distribution in each operation
        BytesPerRead("rocksdb.bytes.per.read"),
        BytesPerWrite("rocksdb.bytes.per.write"),
        BytesPerMultiget("rocksdb.bytes.per.multiget"),
        BytesCompressed("rocksdb.bytes.compressed"),
        /// DEPRECATED / unused (see BytesCompressed{From,To})
        BytesDecompressed("rocksdb.bytes.decompressed"),
        /// DEPRECATED / unused (see BytesDecompressed{From,To})
        CompressionTimesNanos("rocksdb.compression.times.nanos"),
        DecompressionTimesNanos("rocksdb.decompression.times.nanos"),
        /// Number of merge operands passed to the merge operator in user read
        /// requests.
        ReadNumMergeOperands("rocksdb.read.num.merge_operands"),
        /// BlobDB specific stats
        /// Size of keys written to BlobDB. Only applicable to legacy BlobDB.
        BlobDbKeySize("rocksdb.blobdb.key.size"),
        /// Size of values written to BlobDB. Only applicable to legacy BlobDB.
        BlobDbValueSize("rocksdb.blobdb.value.size"),
        /// BlobDB Put/PutWithTTL/PutUntil/Write latency. Only applicable to legacy
        /// BlobDB.
        BlobDbWriteMicros("rocksdb.blobdb.write.micros"),
        /// BlobDB Get latency. Only applicable to legacy BlobDB.
        BlobDbGetMicros("rocksdb.blobdb.get.micros"),
        /// BlobDB MultiGet latency. Only applicable to legacy BlobDB.
        BlobDbMultigetMicros("rocksdb.blobdb.multiget.micros"),
        /// BlobDB Seek/SeekToFirst/SeekToLast/SeekForPrev latency. Only applicable to
        /// legacy BlobDB.
        BlobDbSeekMicros("rocksdb.blobdb.seek.micros"),
        /// BlobDB Next latency. Only applicable to legacy BlobDB.
        BlobDbNextMicros("rocksdb.blobdb.next.micros"),
        /// BlobDB Prev latency. Only applicable to legacy BlobDB.
        BlobDbPrevMicros("rocksdb.blobdb.prev.micros"),
        /// Blob file write latency.
        BlobDbBlobFileWriteMicros("rocksdb.blobdb.blob.file.write.micros"),
        /// Blob file read latency.
        BlobDbBlobFileReadMicros("rocksdb.blobdb.blob.file.read.micros"),
        /// Blob file sync latency.
        BlobDbBlobFileSyncMicros("rocksdb.blobdb.blob.file.sync.micros"),
        /// BlobDB compression time.
        BlobDbCompressionMicros("rocksdb.blobdb.compression.micros"),
        /// BlobDB decompression time.
        BlobDbDecompressionMicros("rocksdb.blobdb.decompression.micros"),
        /// Time spent flushing memtable to disk
        FlushTime("rocksdb.db.flush.micros"),
        SstBatchSize("rocksdb.sst.batch.size"),
        /// MultiGet stats logged per level
        /// Num of index and filter blocks read from file system per level.
        NumIndexAndFilterBlocksReadPerLevel("rocksdb.num.index.and.filter.blocks.read.per.level"),
        /// Num of sst files read from file system per level.
        NumSstReadPerLevel("rocksdb.num.sst.read.per.level"),
        /// Error handler statistics
        ErrorHandlerAutoresumeRetryCount("rocksdb.error.handler.autoresume.retry.count"),
        /// Stats related to asynchronous read requests.
        AsyncReadBytes("rocksdb.async.read.bytes"),
        PollWaitMicros("rocksdb.poll.wait.micros"),
        /// Number of prefetched bytes discarded by RocksDB.
        PrefetchedBytesDiscarded("rocksdb.prefetched.bytes.discarded"),
        /// Number of IOs issued in parallel in a MultiGet batch
        MultigetIoBatchSize("rocksdb.multiget.io.batch.size"),
        /// Number of levels requiring IO for MultiGet
        NumLevelReadPerMultiget("rocksdb.num.level.read.per.multiget"),
        /// Wait time for aborting async read in FilePrefetchBuffer destructor
        AsyncPrefetchAbortMicros("rocksdb.async.prefetch.abort.micros"),
        /// Number of bytes read for RocksDB's prefetching contents (as opposed to file
        /// system's prefetch) from the end of SST table during block based table open
        TableOpenPrefetchTailReadBytes("rocksdb.table.open.prefetch.tail.read.bytes"),
    }
}

pub struct HistogramData {
    pub(crate) inner: *mut ffi::rocksdb_statistics_histogram_data_t,
}

impl HistogramData {
    pub fn new() -> HistogramData {
        HistogramData::default()
    }
    pub fn median(&self) -> f64 {
        unsafe { ffi::rocksdb_statistics_histogram_data_get_median(self.inner) }
    }
    pub fn average(&self) -> f64 {
        unsafe { ffi::rocksdb_statistics_histogram_data_get_average(self.inner) }
    }
    pub fn p95(&self) -> f64 {
        unsafe { ffi::rocksdb_statistics_histogram_data_get_p95(self.inner) }
    }
    pub fn p99(&self) -> f64 {
        unsafe { ffi::rocksdb_statistics_histogram_data_get_p99(self.inner) }
    }
    pub fn max(&self) -> f64 {
        unsafe { ffi::rocksdb_statistics_histogram_data_get_max(self.inner) }
    }
    pub fn min(&self) -> f64 {
        unsafe { ffi::rocksdb_statistics_histogram_data_get_min(self.inner) }
    }
    pub fn sum(&self) -> u64 {
        unsafe { ffi::rocksdb_statistics_histogram_data_get_sum(self.inner) }
    }
    pub fn count(&self) -> u64 {
        unsafe { ffi::rocksdb_statistics_histogram_data_get_count(self.inner) }
    }
    pub fn std_dev(&self) -> f64 {
        unsafe { ffi::rocksdb_statistics_histogram_data_get_std_dev(self.inner) }
    }
}

impl Default for HistogramData {
    fn default() -> Self {
        let histogram_data_inner = unsafe { ffi::rocksdb_statistics_histogram_data_create() };
        assert!(
            !histogram_data_inner.is_null(),
            "Could not create RocksDB histogram data"
        );

        Self {
            inner: histogram_data_inner,
        }
    }
}

impl Drop for HistogramData {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_statistics_histogram_data_destroy(self.inner);
        }
    }
}

#[test]
fn sanity_checks() {
    let want = "rocksdb.async.read.bytes";
    assert_eq!(want, Histogram::AsyncReadBytes.name());

    let want = "rocksdb.block.cache.index.miss";
    assert_eq!(want, Ticker::BlockCacheIndexMiss.to_string());

    // assert enum lengths
    assert_eq!(Ticker::iter().count(), 215 /* TICKER_ENUM_MAX */);
    assert_eq!(Histogram::iter().count(), 60 /* HISTOGRAM_ENUM_MAX */);
}
