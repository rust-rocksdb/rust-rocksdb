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

use crocksdb_ffi::{self, DBTableProperties, DBTableProperty, DBUserCollectedProperties,
                   DBUserCollectedPropertiesIterator, DBTablePropertiesCollection,
                   DBTablePropertiesCollectionIterator};
use libc::size_t;
use std::collections::HashMap;
use std::slice;

#[derive(Debug)]
pub struct TableProperties {
    pub data_size: u64,
    pub index_size: u64,
    pub filter_size: u64,
    pub raw_key_size: u64,
    pub raw_value_size: u64,
    pub num_data_blocks: u64,
    pub num_entries: u64,
    pub format_version: u64,
    pub fixed_key_len: u64,
    pub column_family_id: u64,
    pub column_family_name: String,
    pub filter_policy_name: String,
    pub comparator_name: String,
    pub merge_operator_name: String,
    pub prefix_extractor_name: String,
    pub property_collectors_names: String,
    pub compression_name: String,
    pub user_collected_properties: HashMap<Vec<u8>, Vec<u8>>,
}

pub type TablePropertiesCollection = HashMap<String, TableProperties>;

pub struct TablePropertiesHandle {
    pub inner: *mut DBTableProperties,
}

impl TablePropertiesHandle {
    fn new(inner: *mut DBTableProperties) -> TablePropertiesHandle {
        TablePropertiesHandle { inner: inner }
    }

    fn get_u64(&self, prop: DBTableProperty) -> u64 {
        unsafe { crocksdb_ffi::crocksdb_table_properties_get_u64(self.inner, prop) }
    }

    fn get_str(&self, prop: DBTableProperty) -> Result<String, String> {
        unsafe {
            let mut slen: size_t = 0;
            let s = crocksdb_ffi::crocksdb_table_properties_get_str(self.inner,
                                                                    prop,
                                                                    &mut slen as *mut size_t);
            let bytes = slice::from_raw_parts(s, slen);
            String::from_utf8(bytes.to_owned()).or_else(|e| Err(format!("{}", e)))
        }
    }

    fn get_user_properties(&self) -> *mut DBUserCollectedProperties {
        unsafe { crocksdb_ffi::crocksdb_table_properties_get_user_properties(self.inner) }
    }
}

pub struct TablePropertiesCollectionHandle {
    pub inner: *mut DBTablePropertiesCollection,
}

impl Drop for TablePropertiesCollectionHandle {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_table_properties_collection_destroy(self.inner);
        }
    }
}

impl TablePropertiesCollectionHandle {
    pub fn new() -> TablePropertiesCollectionHandle {
        unsafe {
            TablePropertiesCollectionHandle {
                inner: crocksdb_ffi::crocksdb_table_properties_collection_create(),
            }
        }
    }

    pub fn normalize(&self) -> Result<TablePropertiesCollection, String> {
        let mut collection = TablePropertiesCollection::new();
        let mut it = TablePropertiesCollectionIter::new(self.inner);
        while it.valid() {
            let k = try!(it.key());
            let v = TablePropertiesHandle::new(it.value());
            let mut props = TableProperties {
                data_size: v.get_u64(DBTableProperty::DataSize),
                index_size: v.get_u64(DBTableProperty::IndexSize),
                filter_size: v.get_u64(DBTableProperty::FilterSize),
                raw_key_size: v.get_u64(DBTableProperty::RawKeySize),
                raw_value_size: v.get_u64(DBTableProperty::RawValueSize),
                num_data_blocks: v.get_u64(DBTableProperty::NumDataBlocks),
                num_entries: v.get_u64(DBTableProperty::NumEntries),
                format_version: v.get_u64(DBTableProperty::FormatVersion),
                fixed_key_len: v.get_u64(DBTableProperty::FixedKeyLen),
                column_family_id: v.get_u64(DBTableProperty::ColumnFamilyId),
                column_family_name: try!(v.get_str(DBTableProperty::ColumnFamilyName)),
                filter_policy_name: try!(v.get_str(DBTableProperty::FilterPolicyName)),
                comparator_name: try!(v.get_str(DBTableProperty::ComparatorName)),
                merge_operator_name: try!(v.get_str(DBTableProperty::MergeOperatorName)),
                prefix_extractor_name: try!(v.get_str(DBTableProperty::PrefixExtractorName)),
                property_collectors_names:
                    try!(v.get_str(DBTableProperty::PropertyCollectorsNames)),
                compression_name: try!(v.get_str(DBTableProperty::CompressionName)),
                user_collected_properties: HashMap::new(),
            };
            let mut user_it = UserCollectedPropertiesIter::new(v.get_user_properties());
            while user_it.valid() {
                {
                    let k = user_it.key();
                    let v = user_it.value();
                    props.user_collected_properties.insert(k.to_owned(), v.to_owned());
                }
                user_it.next();
            }
            collection.insert(k, props);
            it.next();
        }
        Ok(collection)
    }
}

struct TablePropertiesCollectionIter {
    pub inner: *mut DBTablePropertiesCollectionIterator,
}

impl Drop for TablePropertiesCollectionIter {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_table_properties_collection_iter_destroy(self.inner);
        }
    }
}

impl TablePropertiesCollectionIter {
    fn new(inner: *mut DBTablePropertiesCollection) -> TablePropertiesCollectionIter {
        unsafe {
            TablePropertiesCollectionIter {
                inner: crocksdb_ffi::crocksdb_table_properties_collection_iter_create(inner),
            }
        }
    }

    fn valid(&self) -> bool {
        unsafe { crocksdb_ffi::crocksdb_table_properties_collection_iter_valid(self.inner) }
    }

    fn next(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_table_properties_collection_iter_next(self.inner);
        }
    }

    fn key(&self) -> Result<String, String> {
        unsafe {
            let mut klen: size_t = 0;
            let k = crocksdb_ffi::crocksdb_table_properties_collection_iter_key(
                self.inner, &mut klen as *mut size_t);
            let bytes = slice::from_raw_parts(k, klen);
            String::from_utf8(bytes.to_owned()).or_else(|e| Err(format!("{}", e)))
        }
    }

    fn value(&self) -> *mut DBTableProperties {
        unsafe { crocksdb_ffi::crocksdb_table_properties_collection_iter_value(self.inner) }
    }
}

struct UserCollectedPropertiesIter {
    pub inner: *mut DBUserCollectedPropertiesIterator,
}

impl Drop for UserCollectedPropertiesIter {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_user_collected_properties_iter_destroy(self.inner);
        }
    }
}

impl UserCollectedPropertiesIter {
    fn new(inner: *mut DBUserCollectedProperties) -> UserCollectedPropertiesIter {
        unsafe {
            UserCollectedPropertiesIter {
                inner: crocksdb_ffi::crocksdb_user_collected_properties_iter_create(inner),
            }
        }
    }

    fn valid(&self) -> bool {
        unsafe { crocksdb_ffi::crocksdb_user_collected_properties_iter_valid(self.inner) }
    }

    fn next(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_user_collected_properties_iter_next(self.inner);
        }
    }

    fn key(&self) -> &[u8] {
        unsafe {
            let mut klen: size_t = 0;
            let k =
                crocksdb_ffi::crocksdb_user_collected_properties_iter_key(self.inner,
                                                                          &mut klen as *mut size_t);
            slice::from_raw_parts(k, klen)
        }
    }

    fn value(&self) -> &[u8] {
        unsafe {
            let mut vlen: size_t = 0;
            let v = crocksdb_ffi::crocksdb_user_collected_properties_iter_value(
                self.inner, &mut vlen as *mut size_t);
            slice::from_raw_parts(v, vlen)
        }
    }
}
