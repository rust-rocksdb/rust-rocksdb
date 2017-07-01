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
use std::marker::PhantomData;
use std::slice;

pub struct TablePropertiesCollection {
    handle: TablePropertiesCollectionHandle,
}

impl TablePropertiesCollection {
    pub fn new(handle: TablePropertiesCollectionHandle) -> TablePropertiesCollection {
        TablePropertiesCollection { handle: handle }
    }

    pub fn iter(&self) -> TablePropertiesCollectionIter {
        TablePropertiesCollectionIter::new(self, self.handle.inner)
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
}

pub struct TablePropertiesCollectionIter<'a> {
    props: &'a TablePropertiesCollection,
    inner: *mut DBTablePropertiesCollectionIterator,
}

impl<'a> Drop for TablePropertiesCollectionIter<'a> {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_table_properties_collection_iter_destroy(self.inner);
        }
    }
}

impl<'a> TablePropertiesCollectionIter<'a> {
    fn new(props: &'a TablePropertiesCollection,
           inner: *mut DBTablePropertiesCollection)
           -> TablePropertiesCollectionIter<'a> {
        unsafe {
            TablePropertiesCollectionIter {
                props: props,
                inner: crocksdb_ffi::crocksdb_table_properties_collection_iter_create(inner),
            }
        }
    }

    pub fn valid(&self) -> bool {
        unsafe { crocksdb_ffi::crocksdb_table_properties_collection_iter_valid(self.inner) }
    }

    pub fn next(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_table_properties_collection_iter_next(self.inner);
        }
    }

    pub fn key(&self) -> String {
        unsafe {
            let mut klen: size_t = 0;
            let k = crocksdb_ffi::crocksdb_table_properties_collection_iter_key(self.inner,
                                                                                &mut klen);
            let bytes = slice::from_raw_parts(k, klen);
            String::from_utf8(bytes.to_owned()).unwrap()
        }
    }

    pub fn value(&self) -> TableProperties {
        unsafe {
            let inner = crocksdb_ffi::crocksdb_table_properties_collection_iter_value(self.inner);
            TableProperties::new(self.props, inner)
        }
    }
}

pub struct TableProperties<'a> {
    phantom: PhantomData<&'a TablePropertiesCollection>,
    inner: *mut DBTableProperties,
}

impl<'a> TableProperties<'a> {
    fn new(_: &'a TablePropertiesCollection, inner: *mut DBTableProperties) -> TableProperties {
        TableProperties {
            phantom: PhantomData,
            inner: inner,
        }
    }

    fn get_u64(&self, prop: DBTableProperty) -> u64 {
        unsafe { crocksdb_ffi::crocksdb_table_properties_get_u64(self.inner, prop) }
    }

    fn get_str(&self, prop: DBTableProperty) -> String {
        unsafe {
            let mut slen: size_t = 0;
            let s = crocksdb_ffi::crocksdb_table_properties_get_str(self.inner, prop, &mut slen);
            let bytes = slice::from_raw_parts(s, slen);
            String::from_utf8(bytes.to_owned()).unwrap()
        }
    }

    pub fn data_size(&self) -> u64 {
        self.get_u64(DBTableProperty::DataSize)
    }

    pub fn index_size(&self) -> u64 {
        self.get_u64(DBTableProperty::IndexSize)
    }

    pub fn filter_size(&self) -> u64 {
        self.get_u64(DBTableProperty::FilterSize)
    }

    pub fn raw_key_size(&self) -> u64 {
        self.get_u64(DBTableProperty::RawKeySize)
    }

    pub fn raw_value_size(&self) -> u64 {
        self.get_u64(DBTableProperty::RawValueSize)
    }

    pub fn num_data_blocks(&self) -> u64 {
        self.get_u64(DBTableProperty::NumDataBlocks)
    }

    pub fn num_entries(&self) -> u64 {
        self.get_u64(DBTableProperty::NumEntries)
    }

    pub fn format_version(&self) -> u64 {
        self.get_u64(DBTableProperty::FormatVersion)
    }

    pub fn fixed_key_len(&self) -> u64 {
        self.get_u64(DBTableProperty::FixedKeyLen)
    }

    pub fn column_family_id(&self) -> u64 {
        self.get_u64(DBTableProperty::ColumnFamilyId)
    }

    pub fn column_family_name(&self) -> String {
        self.get_str(DBTableProperty::ColumnFamilyName)
    }

    pub fn filter_policy_name(&self) -> String {
        self.get_str(DBTableProperty::FilterPolicyName)
    }

    pub fn comparator_name(&self) -> String {
        self.get_str(DBTableProperty::ComparatorName)
    }

    pub fn merge_operator_name(&self) -> String {
        self.get_str(DBTableProperty::MergeOperatorName)
    }

    pub fn prefix_extractor_name(&self) -> String {
        self.get_str(DBTableProperty::PrefixExtractorName)
    }

    pub fn property_collectors_names(&self) -> String {
        self.get_str(DBTableProperty::PropertyCollectorsNames)
    }

    pub fn compression_name(&self) -> String {
        self.get_str(DBTableProperty::CompressionName)
    }

    pub fn user_collected_properties(&self) -> HashMap<Vec<u8>, Vec<u8>> {
        let inner =
            unsafe { crocksdb_ffi::crocksdb_table_properties_get_user_properties(self.inner) };
        let mut iter = UserCollectedPropertiesIter::new(self, inner);
        let mut props = HashMap::new();
        while iter.valid() {
            props.insert(iter.key().to_owned(), iter.value().to_owned());
            iter.next();
        }
        props
    }
}

struct UserCollectedPropertiesIter<'a> {
    phantom: PhantomData<&'a TableProperties<'a>>,
    inner: *mut DBUserCollectedPropertiesIterator,
}

impl<'a> Drop for UserCollectedPropertiesIter<'a> {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_user_collected_properties_iter_destroy(self.inner);
        }
    }
}

impl<'a> UserCollectedPropertiesIter<'a> {
    fn new(_: &'a TableProperties,
           inner: *mut DBUserCollectedProperties)
           -> UserCollectedPropertiesIter<'a> {
        unsafe {
            UserCollectedPropertiesIter {
                phantom: PhantomData,
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
            let k = crocksdb_ffi::crocksdb_user_collected_properties_iter_key(self.inner,
                                                                              &mut klen);
            slice::from_raw_parts(k, klen)
        }
    }

    fn value(&self) -> &[u8] {
        unsafe {
            let mut vlen: size_t = 0;
            let v = crocksdb_ffi::crocksdb_user_collected_properties_iter_value(self.inner,
                                                                                &mut vlen);
            slice::from_raw_parts(v, vlen)
        }
    }
}
