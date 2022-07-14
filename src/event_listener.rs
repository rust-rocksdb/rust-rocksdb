// Copyright 2017 PingCAP, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// See the License for the specific language governing permissions and
// limitations under the License.

use crocksdb_ffi::{
    self, CompactionReason, DBBackgroundErrorReason, DBCompactionJobInfo, DBEventListener,
    DBFlushJobInfo, DBIngestionInfo, DBInstance, DBMemTableInfo, DBStatusPtr,
    DBSubcompactionJobInfo, DBWriteStallInfo, WriteStallCondition,
};
use libc::c_void;
use std::path::Path;
use std::{slice, str};
use {TableProperties, TablePropertiesCollectionView};

macro_rules! fetch_str {
    ($func:ident($($arg:expr),*)) => ({
        let mut len = 0;
        let ptr = crocksdb_ffi::$func($($arg),*, &mut len);
        let s = slice::from_raw_parts(ptr as *const u8, len);
        str::from_utf8(s).unwrap()
    })
}

#[repr(transparent)]
pub struct FlushJobInfo(DBFlushJobInfo);

impl FlushJobInfo {
    pub fn cf_name(&self) -> &str {
        unsafe { fetch_str!(crocksdb_flushjobinfo_cf_name(&self.0)) }
    }

    pub fn file_path(&self) -> &Path {
        let p = unsafe { fetch_str!(crocksdb_flushjobinfo_file_path(&self.0)) };
        Path::new(p)
    }

    pub fn table_properties(&self) -> &TableProperties {
        unsafe {
            let prop = crocksdb_ffi::crocksdb_flushjobinfo_table_properties(&self.0);
            TableProperties::from_ptr(prop)
        }
    }

    pub fn triggered_writes_slowdown(&self) -> bool {
        unsafe { crocksdb_ffi::crocksdb_flushjobinfo_triggered_writes_slowdown(&self.0) }
    }

    pub fn triggered_writes_stop(&self) -> bool {
        unsafe { crocksdb_ffi::crocksdb_flushjobinfo_triggered_writes_stop(&self.0) }
    }

    pub fn largest_seqno(&self) -> u64 {
        unsafe { crocksdb_ffi::crocksdb_flushjobinfo_largest_seqno(&self.0) }
    }

    pub fn smallest_seqno(&self) -> u64 {
        unsafe { crocksdb_ffi::crocksdb_flushjobinfo_smallest_seqno(&self.0) }
    }
}

pub struct MutableStatus {
    result: Result<(), String>,
    ptr: *mut DBStatusPtr,
}

impl MutableStatus {
    pub fn reset(&self) {
        unsafe { crocksdb_ffi::crocksdb_reset_status(self.ptr) }
    }

    pub fn result(&self) -> Result<(), String> {
        self.result.clone()
    }
}

#[repr(transparent)]
pub struct CompactionJobInfo(DBCompactionJobInfo);

impl CompactionJobInfo {
    pub fn status(&self) -> Result<(), String> {
        unsafe { ffi_try!(crocksdb_compactionjobinfo_status(&self.0)) }
        Ok(())
    }

    pub fn cf_name(&self) -> &str {
        unsafe { fetch_str!(crocksdb_compactionjobinfo_cf_name(&self.0)) }
    }

    pub fn input_file_count(&self) -> usize {
        unsafe { crocksdb_ffi::crocksdb_compactionjobinfo_input_files_count(&self.0) }
    }

    pub fn input_file_at(&self, pos: usize) -> &Path {
        let p = unsafe { fetch_str!(crocksdb_compactionjobinfo_input_file_at(&self.0, pos)) };
        Path::new(p)
    }

    pub fn output_file_count(&self) -> usize {
        unsafe { crocksdb_ffi::crocksdb_compactionjobinfo_output_files_count(&self.0) }
    }

    pub fn output_file_at(&self, pos: usize) -> &Path {
        let p = unsafe { fetch_str!(crocksdb_compactionjobinfo_output_file_at(&self.0, pos)) };
        Path::new(p)
    }

    pub fn table_properties(&self) -> &TablePropertiesCollectionView {
        unsafe {
            let prop = crocksdb_ffi::crocksdb_compactionjobinfo_table_properties(&self.0);
            TablePropertiesCollectionView::from_ptr(prop)
        }
    }

    pub fn elapsed_micros(&self) -> u64 {
        unsafe { crocksdb_ffi::crocksdb_compactionjobinfo_elapsed_micros(&self.0) }
    }

    pub fn num_corrupt_keys(&self) -> u64 {
        unsafe { crocksdb_ffi::crocksdb_compactionjobinfo_num_corrupt_keys(&self.0) }
    }

    pub fn base_input_level(&self) -> i32 {
        unsafe { crocksdb_ffi::crocksdb_compactionjobinfo_base_input_level(&self.0) }
    }

    pub fn output_level(&self) -> i32 {
        unsafe { crocksdb_ffi::crocksdb_compactionjobinfo_output_level(&self.0) }
    }

    pub fn input_records(&self) -> u64 {
        unsafe { crocksdb_ffi::crocksdb_compactionjobinfo_input_records(&self.0) }
    }

    pub fn output_records(&self) -> u64 {
        unsafe { crocksdb_ffi::crocksdb_compactionjobinfo_output_records(&self.0) }
    }

    pub fn total_input_bytes(&self) -> u64 {
        unsafe { crocksdb_ffi::crocksdb_compactionjobinfo_total_input_bytes(&self.0) }
    }

    pub fn total_output_bytes(&self) -> u64 {
        unsafe { crocksdb_ffi::crocksdb_compactionjobinfo_total_output_bytes(&self.0) }
    }

    pub fn num_input_files_at_output_level(&self) -> usize {
        unsafe { crocksdb_ffi::crocksdb_compactionjobinfo_num_input_files_at_output_level(&self.0) }
    }

    pub fn compaction_reason(&self) -> CompactionReason {
        unsafe { crocksdb_ffi::crocksdb_compactionjobinfo_compaction_reason(&self.0) }
    }
}

#[repr(transparent)]
pub struct SubcompactionJobInfo(DBSubcompactionJobInfo);

impl SubcompactionJobInfo {
    pub fn status(&self) -> Result<(), String> {
        unsafe { ffi_try!(crocksdb_subcompactionjobinfo_status(&self.0)) }
        Ok(())
    }

    pub fn cf_name(&self) -> &str {
        unsafe { fetch_str!(crocksdb_subcompactionjobinfo_cf_name(&self.0)) }
    }

    pub fn thread_id(&self) -> u64 {
        unsafe { crocksdb_ffi::crocksdb_subcompactionjobinfo_thread_id(&self.0) }
    }

    pub fn base_input_level(&self) -> i32 {
        unsafe { crocksdb_ffi::crocksdb_subcompactionjobinfo_base_input_level(&self.0) }
    }

    pub fn output_level(&self) -> i32 {
        unsafe { crocksdb_ffi::crocksdb_subcompactionjobinfo_output_level(&self.0) }
    }
}

#[repr(transparent)]
pub struct IngestionInfo(DBIngestionInfo);

impl IngestionInfo {
    pub fn cf_name(&self) -> &str {
        unsafe { fetch_str!(crocksdb_externalfileingestioninfo_cf_name(&self.0)) }
    }

    pub fn internal_file_path(&self) -> &Path {
        let p = unsafe {
            fetch_str!(crocksdb_externalfileingestioninfo_internal_file_path(
                &self.0
            ))
        };
        Path::new(p)
    }

    pub fn table_properties(&self) -> &TableProperties {
        unsafe {
            let prop = crocksdb_ffi::crocksdb_externalfileingestioninfo_table_properties(&self.0);
            TableProperties::from_ptr(prop)
        }
    }

    pub fn picked_level(&self) -> i32 {
        unsafe { crocksdb_ffi::crocksdb_externalfileingestioninfo_picked_level(&self.0) }
    }
}

#[repr(transparent)]
pub struct WriteStallInfo(DBWriteStallInfo);

impl WriteStallInfo {
    pub fn cf_name(&self) -> &str {
        unsafe { fetch_str!(crocksdb_writestallinfo_cf_name(&self.0)) }
    }
    pub fn cur(&self) -> WriteStallCondition {
        unsafe { *crocksdb_ffi::crocksdb_writestallinfo_cur(&self.0) }
    }
    pub fn prev(&self) -> WriteStallCondition {
        unsafe { *crocksdb_ffi::crocksdb_writestallinfo_prev(&self.0) }
    }
}

pub struct MemTableInfo(DBMemTableInfo);

impl MemTableInfo {
    pub fn cf_name(&self) -> &str {
        unsafe { fetch_str!(crocksdb_memtableinfo_cf_name(&self.0)) }
    }
    pub fn first_seqno(&self) -> u64 {
        unsafe { crocksdb_ffi::crocksdb_memtableinfo_first_seqno(&self.0) }
    }
    pub fn earliest_seqno(&self) -> u64 {
        unsafe { crocksdb_ffi::crocksdb_memtableinfo_earliest_seqno(&self.0) }
    }
    pub fn num_entries(&self) -> u64 {
        unsafe { crocksdb_ffi::crocksdb_memtableinfo_num_entries(&self.0) }
    }
    pub fn num_deletes(&self) -> u64 {
        unsafe { crocksdb_ffi::crocksdb_memtableinfo_num_deletes(&self.0) }
    }
}

/// EventListener trait contains a set of call-back functions that will
/// be called when specific RocksDB event happens such as flush.  It can
/// be used as a building block for developing custom features such as
/// stats-collector or external compaction algorithm.
///
/// Note that call-back functions should not run for an extended period of
/// time before the function returns, otherwise RocksDB may be blocked.
/// For more information, please see
/// [doc of rocksdb](https://github.com/facebook/rocksdb/blob/master/include/rocksdb/listener.h).
pub trait EventListener: Send + Sync {
    fn on_flush_begin(&self, _: &FlushJobInfo) {}
    fn on_flush_completed(&self, _: &FlushJobInfo) {}
    fn on_compaction_begin(&self, _: &CompactionJobInfo) {}
    fn on_compaction_completed(&self, _: &CompactionJobInfo) {}
    fn on_subcompaction_begin(&self, _: &SubcompactionJobInfo) {}
    fn on_subcompaction_completed(&self, _: &SubcompactionJobInfo) {}
    fn on_external_file_ingested(&self, _: &IngestionInfo) {}
    fn on_background_error(&self, _: DBBackgroundErrorReason, _: MutableStatus) {}
    fn on_stall_conditions_changed(&self, _: &WriteStallInfo) {}
    fn on_memtable_sealed(&self, _: &MemTableInfo) {}
}

extern "C" fn destructor<E: EventListener>(ctx: *mut c_void) {
    unsafe {
        Box::from_raw(ctx as *mut E);
    }
}

// Maybe we should reuse db instance?
// TODO: refactor DB implement so that we can convert DBInstance to DB.
extern "C" fn on_flush_begin<E: EventListener>(
    ctx: *mut c_void,
    _: *mut DBInstance,
    info: *const DBFlushJobInfo,
) {
    let (ctx, info) = unsafe { (&*(ctx as *mut E), &*(info as *const FlushJobInfo)) };
    ctx.on_flush_begin(info);
}

extern "C" fn on_flush_completed<E: EventListener>(
    ctx: *mut c_void,
    _: *mut DBInstance,
    info: *const DBFlushJobInfo,
) {
    let (ctx, info) = unsafe { (&*(ctx as *mut E), &*(info as *const FlushJobInfo)) };
    ctx.on_flush_completed(info);
}

extern "C" fn on_compaction_begin<E: EventListener>(
    ctx: *mut c_void,
    _: *mut DBInstance,
    info: *const DBCompactionJobInfo,
) {
    let (ctx, info) = unsafe { (&*(ctx as *mut E), &*(info as *const CompactionJobInfo)) };
    ctx.on_compaction_begin(info);
}

extern "C" fn on_compaction_completed<E: EventListener>(
    ctx: *mut c_void,
    _: *mut DBInstance,
    info: *const DBCompactionJobInfo,
) {
    let (ctx, info) = unsafe { (&*(ctx as *mut E), &*(info as *const CompactionJobInfo)) };
    ctx.on_compaction_completed(info);
}

extern "C" fn on_subcompaction_begin<E: EventListener>(
    ctx: *mut c_void,
    info: *const DBSubcompactionJobInfo,
) {
    let (ctx, info) = unsafe { (&*(ctx as *mut E), &*(info as *const SubcompactionJobInfo)) };
    ctx.on_subcompaction_begin(info);
}

extern "C" fn on_subcompaction_completed<E: EventListener>(
    ctx: *mut c_void,
    info: *const DBSubcompactionJobInfo,
) {
    let (ctx, info) = unsafe { (&*(ctx as *mut E), &*(info as *const SubcompactionJobInfo)) };
    ctx.on_subcompaction_completed(info);
}

extern "C" fn on_external_file_ingested<E: EventListener>(
    ctx: *mut c_void,
    _: *mut DBInstance,
    info: *const DBIngestionInfo,
) {
    let (ctx, info) = unsafe { (&*(ctx as *mut E), &*(info as *const IngestionInfo)) };
    ctx.on_external_file_ingested(info);
}

extern "C" fn on_background_error<E: EventListener>(
    ctx: *mut c_void,
    reason: DBBackgroundErrorReason,
    status_ptr: *mut DBStatusPtr,
) {
    let (ctx, result) = unsafe {
        (
            &*(ctx as *mut E),
            || -> Result<(), String> {
                ffi_try!(crocksdb_status_ptr_get_error(status_ptr));
                Ok(())
            }(),
        )
    };
    let status = MutableStatus {
        result: result,
        ptr: status_ptr,
    };
    ctx.on_background_error(reason, status);
}

extern "C" fn on_stall_conditions_changed<E: EventListener>(
    ctx: *mut c_void,
    info: *const DBWriteStallInfo,
) {
    let (ctx, info) = unsafe { (&*(ctx as *mut E), &*(info as *const WriteStallInfo)) };
    ctx.on_stall_conditions_changed(info);
}

extern "C" fn on_memtable_sealed<E: EventListener>(ctx: *mut c_void, info: *const DBMemTableInfo) {
    let (ctx, info) = unsafe { (&*(ctx as *mut E), &*(info as *const MemTableInfo)) };
    ctx.on_memtable_sealed(info);
}

pub fn new_event_listener<E: EventListener>(e: E) -> *mut DBEventListener {
    let p: Box<dyn EventListener> = Box::new(e);
    unsafe {
        crocksdb_ffi::crocksdb_eventlistener_create(
            Box::into_raw(p) as *mut c_void,
            destructor::<E>,
            on_flush_begin::<E>,
            on_flush_completed::<E>,
            on_compaction_begin::<E>,
            on_compaction_completed::<E>,
            on_subcompaction_begin::<E>,
            on_subcompaction_completed::<E>,
            on_external_file_ingested::<E>,
            on_background_error::<E>,
            on_stall_conditions_changed::<E>,
            on_memtable_sealed::<E>,
        )
    }
}
