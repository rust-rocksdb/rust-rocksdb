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
    self, DBTableProperties, DBTablePropertiesCollection, DBTablePropertiesCollectionIterator,
    DBTableProperty, DBUserCollectedProperties, DBUserCollectedPropertiesIterator,
};
use libc::size_t;
use std::marker::PhantomData;
use std::ops::{Deref, Index};
use std::{slice, str};

#[repr(transparent)]
pub struct TablePropertiesCollectionView(DBTablePropertiesCollection);

impl TablePropertiesCollectionView {
    pub unsafe fn from_ptr<'a>(
        collection: *const DBTablePropertiesCollection,
    ) -> &'a TablePropertiesCollectionView {
        &*(collection as *const TablePropertiesCollectionView)
    }

    pub fn iter(&self) -> TablePropertiesCollectionIter {
        self.into_iter()
    }

    pub fn len(&self) -> usize {
        unsafe { crocksdb_ffi::crocksdb_table_properties_collection_len(&self.0) }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<'a> IntoIterator for &'a TablePropertiesCollectionView {
    type Item = (&'a str, &'a TableProperties);
    type IntoIter = TablePropertiesCollectionIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        TablePropertiesCollectionIter::new(self)
    }
}

pub struct TablePropertiesCollectionIter<'a> {
    props: PhantomData<&'a TablePropertiesCollection>,
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
    fn new(props: &'a TablePropertiesCollectionView) -> TablePropertiesCollectionIter<'a> {
        unsafe {
            TablePropertiesCollectionIter {
                props: PhantomData,
                inner: crocksdb_ffi::crocksdb_table_properties_collection_iter_create(&props.0),
            }
        }
    }
}

impl<'a> Iterator for TablePropertiesCollectionIter<'a> {
    type Item = (&'a str, &'a TableProperties);

    fn next(&mut self) -> Option<(&'a str, &'a TableProperties)> {
        unsafe {
            loop {
                if !crocksdb_ffi::crocksdb_table_properties_collection_iter_valid(self.inner) {
                    return None;
                }

                let mut klen: size_t = 0;
                let k = crocksdb_ffi::crocksdb_table_properties_collection_iter_key(
                    self.inner, &mut klen,
                );
                let bytes = slice::from_raw_parts(k, klen);
                let key = str::from_utf8(bytes).unwrap();
                let props =
                    crocksdb_ffi::crocksdb_table_properties_collection_iter_value(self.inner);
                crocksdb_ffi::crocksdb_table_properties_collection_iter_next(self.inner);
                if !props.is_null() {
                    let val = TableProperties::from_ptr(props);
                    return Some((key, val));
                }
            }
        }
    }
}

pub struct TablePropertiesCollection {
    inner: *mut DBTablePropertiesCollection,
}

impl Drop for TablePropertiesCollection {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_table_properties_collection_destroy(self.inner);
        }
    }
}

impl TablePropertiesCollection {
    pub unsafe fn from_raw(ptr: *mut DBTablePropertiesCollection) -> TablePropertiesCollection {
        TablePropertiesCollection { inner: ptr }
    }
}

impl Deref for TablePropertiesCollection {
    type Target = TablePropertiesCollectionView;

    fn deref(&self) -> &TablePropertiesCollectionView {
        unsafe { TablePropertiesCollectionView::from_ptr(self.inner) }
    }
}

#[repr(transparent)]
pub struct TableProperties {
    inner: DBTableProperties,
}

impl TableProperties {
    pub unsafe fn from_ptr<'a>(ptr: *const DBTableProperties) -> &'a TableProperties {
        &*(ptr as *const TableProperties)
    }

    fn get_u64(&self, prop: DBTableProperty) -> u64 {
        unsafe { crocksdb_ffi::crocksdb_table_properties_get_u64(&self.inner, prop) }
    }

    fn get_str(&self, prop: DBTableProperty) -> &str {
        unsafe {
            let mut slen: size_t = 0;
            let s = crocksdb_ffi::crocksdb_table_properties_get_str(&self.inner, prop, &mut slen);
            let bytes = slice::from_raw_parts(s, slen);
            str::from_utf8(bytes).unwrap()
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

    pub fn column_family_name(&self) -> &str {
        self.get_str(DBTableProperty::ColumnFamilyName)
    }

    pub fn filter_policy_name(&self) -> &str {
        self.get_str(DBTableProperty::FilterPolicyName)
    }

    pub fn comparator_name(&self) -> &str {
        self.get_str(DBTableProperty::ComparatorName)
    }

    pub fn merge_operator_name(&self) -> &str {
        self.get_str(DBTableProperty::MergeOperatorName)
    }

    pub fn prefix_extractor_name(&self) -> &str {
        self.get_str(DBTableProperty::PrefixExtractorName)
    }

    pub fn property_collectors_names(&self) -> &str {
        self.get_str(DBTableProperty::PropertyCollectorsNames)
    }

    pub fn compression_name(&self) -> &str {
        self.get_str(DBTableProperty::CompressionName)
    }

    pub fn user_collected_properties(&self) -> &UserCollectedProperties {
        unsafe {
            let ptr = crocksdb_ffi::crocksdb_table_properties_get_user_properties(&self.inner);
            UserCollectedProperties::from_ptr(ptr)
        }
    }
}

#[repr(transparent)]
pub struct UserCollectedProperties {
    inner: DBUserCollectedProperties,
}

impl UserCollectedProperties {
    unsafe fn from_ptr<'a>(ptr: *const DBUserCollectedProperties) -> &'a UserCollectedProperties {
        &*(ptr as *const UserCollectedProperties)
    }

    pub fn get<Q: AsRef<[u8]>>(&self, index: Q) -> Option<&[u8]> {
        let bytes = index.as_ref();
        let mut size = 0;
        unsafe {
            let ptr = crocksdb_ffi::crocksdb_user_collected_properties_get(
                &self.inner,
                bytes.as_ptr(),
                bytes.len(),
                &mut size,
            );
            if ptr.is_null() {
                return None;
            }
            Some(slice::from_raw_parts(ptr, size))
        }
    }

    pub fn len(&self) -> usize {
        unsafe { crocksdb_ffi::crocksdb_user_collected_properties_len(&self.inner) }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<Q: AsRef<[u8]>> Index<Q> for UserCollectedProperties {
    type Output = [u8];

    fn index(&self, index: Q) -> &[u8] {
        let key = index.as_ref();
        self.get(key)
            .unwrap_or_else(|| panic!("no entry found for key {:?}", key))
    }
}

impl<'a> IntoIterator for &'a UserCollectedProperties {
    type Item = (&'a [u8], &'a [u8]);
    type IntoIter = UserCollectedPropertiesIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        UserCollectedPropertiesIter::new(self)
    }
}

pub struct UserCollectedPropertiesIter<'a> {
    props: PhantomData<&'a UserCollectedProperties>,
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
    fn new(props: &'a UserCollectedProperties) -> UserCollectedPropertiesIter<'a> {
        unsafe {
            UserCollectedPropertiesIter {
                props: PhantomData,
                inner: crocksdb_ffi::crocksdb_user_collected_properties_iter_create(&props.inner),
            }
        }
    }
}

impl<'a> Iterator for UserCollectedPropertiesIter<'a> {
    type Item = (&'a [u8], &'a [u8]);

    fn next(&mut self) -> Option<(&'a [u8], &'a [u8])> {
        unsafe {
            if !crocksdb_ffi::crocksdb_user_collected_properties_iter_valid(self.inner) {
                return None;
            }
            let mut klen: size_t = 0;
            let k =
                crocksdb_ffi::crocksdb_user_collected_properties_iter_key(self.inner, &mut klen);
            let key = slice::from_raw_parts(k, klen);

            let mut vlen: size_t = 0;
            let v =
                crocksdb_ffi::crocksdb_user_collected_properties_iter_value(self.inner, &mut vlen);
            let val = slice::from_raw_parts(v, vlen);

            crocksdb_ffi::crocksdb_user_collected_properties_iter_next(self.inner);

            Some((key, val))
        }
    }
}
