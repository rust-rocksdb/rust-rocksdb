//! Reference counted handles to various TableProperties* types, that safely
//! preserve destruction order.

use crocksdb_ffi::{
    DBTableProperties, DBTablePropertiesCollection, DBTablePropertiesCollectionIterator,
    DBUserCollectedProperties,
};
use librocksdb_sys as crocksdb_ffi;
use std::rc::Rc;

/// This is a shared wrapper around a DBTablePropertiesCollection w/ dtor
#[derive(Clone)]
pub struct TablePropertiesCollectionHandle {
    shared: Rc<TablePropertiesCollectionHandleWithDrop>,
}

impl TablePropertiesCollectionHandle {
    pub unsafe fn new(ptr: *mut DBTablePropertiesCollection) -> TablePropertiesCollectionHandle {
        assert!(!ptr.is_null());
        TablePropertiesCollectionHandle {
            shared: Rc::new(TablePropertiesCollectionHandleWithDrop { ptr }),
        }
    }

    pub fn ptr(&self) -> *mut DBTablePropertiesCollection {
        self.shared.ptr
    }
}

struct TablePropertiesCollectionHandleWithDrop {
    ptr: *mut DBTablePropertiesCollection,
}

impl Drop for TablePropertiesCollectionHandleWithDrop {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_table_properties_collection_destroy(self.ptr);
        }
    }
}

/// This is a shared wrapper around a DBTablePropertiesCollection w/ dtor.
//
// # Safety
//
// The safety of this struct depends on drop order, with the iterator
// needing to drop before the collection.
#[derive(Clone)]
pub struct TablePropertiesCollectionIteratorHandle {
    shared: Rc<TablePropertiesCollectionIteratorHandleWithDrop>,
    collection: TablePropertiesCollectionHandle,
}

impl TablePropertiesCollectionIteratorHandle {
    pub fn new(
        collection: TablePropertiesCollectionHandle,
    ) -> TablePropertiesCollectionIteratorHandle {
        unsafe {
            let ptr =
                crocksdb_ffi::crocksdb_table_properties_collection_iter_create(collection.ptr());
            TablePropertiesCollectionIteratorHandle {
                shared: Rc::new(TablePropertiesCollectionIteratorHandleWithDrop { ptr }),
                collection,
            }
        }
    }

    pub fn ptr(&self) -> *mut DBTablePropertiesCollectionIterator {
        self.shared.ptr
    }
}

struct TablePropertiesCollectionIteratorHandleWithDrop {
    ptr: *mut DBTablePropertiesCollectionIterator,
}

impl Drop for TablePropertiesCollectionIteratorHandleWithDrop {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_table_properties_collection_iter_destroy(self.ptr);
        }
    }
}

// # Safety
//
// `ptr` is valid as long as the iterator is
#[derive(Clone)]
pub struct TablePropertiesHandle {
    ptr: *const DBTableProperties,
    iter_handle: TablePropertiesCollectionIteratorHandle,
}

impl TablePropertiesHandle {
    pub fn new(
        ptr: *const DBTableProperties,
        iter_handle: TablePropertiesCollectionIteratorHandle,
    ) -> TablePropertiesHandle {
        TablePropertiesHandle { ptr, iter_handle }
    }

    pub fn ptr(&self) -> *const DBTableProperties {
        self.ptr
    }
}

// # Safety
//
// `ptr` is valid as long as the table properties are
#[derive(Clone)]
pub struct UserCollectedPropertiesHandle {
    ptr: *const DBUserCollectedProperties,
    table_props_handle: TablePropertiesHandle,
}

impl UserCollectedPropertiesHandle {
    pub fn new(table_props_handle: TablePropertiesHandle) -> UserCollectedPropertiesHandle {
        unsafe {
            let ptr = crocksdb_ffi::crocksdb_table_properties_get_user_properties(
                table_props_handle.ptr(),
            );
            UserCollectedPropertiesHandle {
                ptr,
                table_props_handle,
            }
        }
    }

    pub fn ptr(&self) -> *const DBUserCollectedProperties {
        self.ptr
    }
}
