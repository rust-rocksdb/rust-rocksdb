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

use libc::c_void;
use std::path::Path;
use std::{mem, slice, str};

use crocksdb_ffi::{self, DBCompactionJobInfo, DBEventListener, DBFlushJobInfo, DBIngestionInfo,
                   DBInstance};
use {TableProperties, TablePropertiesCollectionView};

macro_rules! fetch_str {
    ($func:ident($($arg:expr),*)) => ({
        let mut len = 0;
        let ptr = crocksdb_ffi::$func($($arg),*, &mut len);
        let s = slice::from_raw_parts(ptr as *const u8, len);
        str::from_utf8(s).unwrap()
    })
}

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
}

pub struct CompactionJobInfo(DBCompactionJobInfo);

impl CompactionJobInfo {
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
}

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
    fn on_flush_completed(&self, _: &FlushJobInfo) {}
    fn on_compaction_completed(&self, _: &CompactionJobInfo) {}
    fn on_external_file_ingested(&self, _: &IngestionInfo) {}
}

extern "C" fn destructor(ctx: *mut c_void) {
    unsafe {
        Box::from_raw(ctx as *mut Box<EventListener>);
    }
}

// Maybe we should reuse db instance?
// TODO: refactor DB implement so that we can convert DBInstance to DB.
extern "C" fn on_flush_completed(
    ctx: *mut c_void,
    _: *mut DBInstance,
    info: *const DBFlushJobInfo,
) {
    let (ctx, info) = unsafe { (&*(ctx as *mut Box<EventListener>), mem::transmute(&*info)) };
    ctx.on_flush_completed(info);
}

extern "C" fn on_compaction_completed(
    ctx: *mut c_void,
    _: *mut DBInstance,
    info: *const DBCompactionJobInfo,
) {
    let (ctx, info) = unsafe { (&*(ctx as *mut Box<EventListener>), mem::transmute(&*info)) };
    ctx.on_compaction_completed(info);
}

extern "C" fn on_external_file_ingested(
    ctx: *mut c_void,
    _: *mut DBInstance,
    info: *const DBIngestionInfo,
) {
    let (ctx, info) = unsafe { (&*(ctx as *mut Box<EventListener>), mem::transmute(&*info)) };
    ctx.on_external_file_ingested(info);
}

pub fn new_event_listener<L: EventListener>(l: L) -> *mut DBEventListener {
    let p: Box<EventListener> = Box::new(l);
    unsafe {
        crocksdb_ffi::crocksdb_eventlistener_create(
            Box::into_raw(Box::new(p)) as *mut c_void,
            destructor,
            on_flush_completed,
            on_compaction_completed,
            on_external_file_ingested,
        )
    }
}
