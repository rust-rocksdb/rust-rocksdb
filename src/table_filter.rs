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

use crocksdb_ffi::DBTableProperties;
use libc::{c_int, c_void};
use std::mem;
use table_properties::TableProperties;

pub trait TableFilter {
    // A callback to determine whether relevant keys for this scan exist in a
    // given table based on the table's properties. The callback is passed the
    // properties of each table during iteration. If the callback returns false,
    // the table will not be scanned. This option only affects Iterators and has
    // no impact on point lookups.
    fn table_filter(&self, props: &TableProperties) -> bool;
}

pub extern "C" fn table_filter(ctx: *mut c_void, props: *const DBTableProperties) -> c_int {
    unsafe {
        let filter = &*(ctx as *mut Box<dyn TableFilter>);
        filter.table_filter(mem::transmute(&*props)) as c_int
    }
}

pub extern "C" fn destroy_table_filter(filter: *mut c_void) {
    unsafe {
        Box::from_raw(filter as *mut Box<dyn TableFilter>);
    }
}
