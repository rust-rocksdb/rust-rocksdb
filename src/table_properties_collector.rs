use crocksdb_ffi::{self, DBEntryType, DBTablePropertiesCollectorContext};
use libc::{c_void, c_char, c_int, uint8_t, uint64_t, size_t};
use std::collections::HashMap;
use std::mem;
use std::slice;

/// `TablePropertiesCollector` provides the mechanism for users to collect
/// their own properties that they are interested in. This class is essentially
/// a collection of callback functions that will be invoked during table
/// building. It is construced with TablePropertiesCollectorFactory. The methods
/// don't need to be thread-safe, as we will create exactly one
/// TablePropertiesCollector object per table and then call it sequentially
pub trait TablePropertiesCollector {
    /// The name of the properties collector.
    fn name(&self) -> &str;

    /// Will be called when a new key/value pair is inserted into the table.
    fn add_userkey(&mut self, key: &[u8], value: &[u8], entry_type: DBEntryType);

    /// Will be called when a table has already been built and is ready for
    /// writing the properties block.
    fn finish(&mut self) -> HashMap<Vec<u8>, Vec<u8>>;

    /// Return the human-readable properties, where the key is property name and
    /// the value is the human-readable form of value.
    fn readable_properties(&self) -> HashMap<String, String>;
}

extern "C" fn name(context: *mut c_void) -> *const c_char {
    unsafe {
        let context = &mut *(context as *mut DBTablePropertiesCollectorContext);
        let collector = &mut *(context.collector as *mut Box<TablePropertiesCollector>);
        collector.name().as_ptr() as *const c_char
    }
}

extern "C" fn destructor(context: *mut c_void) {
    unsafe {
        let context = Box::from_raw(context as *mut DBTablePropertiesCollectorContext);
        Box::from_raw(context.collector as *mut Box<TablePropertiesCollector>);
    }
}

pub extern "C" fn add_userkey(context: *mut c_void,
                              key: *const uint8_t,
                              key_len: size_t,
                              value: *const uint8_t,
                              value_len: size_t,
                              entry_type: c_int,
                              _: uint64_t,
                              _: uint64_t) {
    unsafe {
        let context = &mut *(context as *mut DBTablePropertiesCollectorContext);
        let collector = &mut *(context.collector as *mut Box<TablePropertiesCollector>);
        let key = slice::from_raw_parts(key, key_len);
        let value = slice::from_raw_parts(value, value_len);
        collector.add_userkey(key, value, mem::transmute(entry_type))
    }
}

pub extern "C" fn finish(context: *mut c_void, props: *mut c_void) {
    unsafe {
        let context = &mut *(context as *mut DBTablePropertiesCollectorContext);
        let collector = &mut *(context.collector as *mut Box<TablePropertiesCollector>);
        for (key, value) in collector.finish() {
            crocksdb_ffi::crocksdb_user_collected_properties_add(props,
                                                                 key.as_ptr(),
                                                                 key.len(),
                                                                 value.as_ptr(),
                                                                 value.len());
        }
    }
}

pub extern "C" fn readable_properties(context: *mut c_void, props: *mut c_void) {
    unsafe {
        let context = &mut *(context as *mut DBTablePropertiesCollectorContext);
        let collector = &mut *(context.collector as *mut Box<TablePropertiesCollector>);
        for (key, value) in collector.readable_properties() {
            crocksdb_ffi::crocksdb_user_collected_properties_add(props,
                                                                 key.as_ptr(),
                                                                 key.len(),
                                                                 value.as_ptr(),
                                                                 value.len());
        }
    }
}

pub unsafe fn new_table_properties_collector_context(collector: Box<TablePropertiesCollector>)
                                                     -> Box<DBTablePropertiesCollectorContext> {
    Box::new(DBTablePropertiesCollectorContext {
        collector: Box::into_raw(Box::new(collector)) as *mut c_void,
        name: name,
        destructor: destructor,
        add_userkey: add_userkey,
        finish: finish,
        readable_properties: readable_properties,
    })
}
