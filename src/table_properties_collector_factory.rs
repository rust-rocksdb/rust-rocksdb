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

use crocksdb_ffi::{self, DBTablePropertiesCollector, DBTablePropertiesCollectorFactory};
use libc::{c_void, c_char, uint32_t};
use table_properties_collector::{TablePropertiesCollector, new_table_properties_collector};

/// Constructs `TablePropertiesCollector`.
/// Internals create a new `TablePropertiesCollector` for each new table.
pub trait TablePropertiesCollectorFactory {
    /// The name of the properties collector factory.
    fn name(&self) -> &str;
    /// Has to be thread-safe.
    fn create_table_properties_collector(&mut self, cf: u32) -> Box<TablePropertiesCollector>;
}

extern "C" fn name(factory: *mut c_void) -> *const c_char {
    unsafe {
        let factory = &mut *(factory as *mut Box<TablePropertiesCollectorFactory>);
        factory.name().as_ptr() as *const c_char
    }
}

extern "C" fn destruct(factory: *mut c_void) {
    unsafe {
        Box::from_raw(factory as *mut Box<TablePropertiesCollectorFactory>);
    }
}

extern "C" fn create_table_properties_collector(factory: *mut c_void,
                                                cf: uint32_t)
                                                -> *mut DBTablePropertiesCollector {
    unsafe {
        let factory = &mut *(factory as *mut Box<TablePropertiesCollectorFactory>);
        let collector = factory.create_table_properties_collector(cf);
        new_table_properties_collector(collector)
    }
}

pub unsafe fn new_table_properties_collector_factory
    (factory: Box<TablePropertiesCollectorFactory>)
     -> *mut DBTablePropertiesCollectorFactory {
    crocksdb_ffi::crocksdb_table_properties_collector_factory_create(
            Box::into_raw(Box::new(factory)) as *mut c_void,
            name,
            destruct,
            create_table_properties_collector,
    )
}
