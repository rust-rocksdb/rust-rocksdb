use crocksdb_ffi::{self, DBTablePropertiesCollection};
use libc::{c_void, c_char, size_t, uint64_t};
use std::collections::HashMap;
use std::ffi::CStr;
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
    pub readable_properties: HashMap<String, String>,
}

pub type TablePropertiesCollection = HashMap<String, TableProperties>;

fn ptr_to_string(ptr: *const c_char) -> Result<String, String> {
    unsafe {
        match CStr::from_ptr(ptr).to_str() {
            Ok(s) => Ok(s.to_owned()),
            Err(e) => Err(format!("{}", e)),
        }
    }
}

#[repr(C)]
struct CShallowStringsMap {
    size: size_t,
    keys: *const *const c_char,
    keys_lens: *const size_t,
    values: *const *const c_char,
    values_lens: *const size_t,
}

impl CShallowStringsMap {
    fn to_bytes_map(&self) -> Result<HashMap<Vec<u8>, Vec<u8>>, String> {
        let mut res = HashMap::new();
        unsafe {
            let keys = slice::from_raw_parts(self.keys, self.size);
            let keys_lens = slice::from_raw_parts(self.keys_lens, self.size);
            let values = slice::from_raw_parts(self.values, self.size);
            let values_lens = slice::from_raw_parts(self.values_lens, self.size);
            for ((k, klen), (v, vlen)) in keys.iter()
                .zip(keys_lens.iter())
                .zip(values.iter().zip(values_lens.iter())) {
                let k = slice::from_raw_parts(*k as *const u8, *klen);
                let v = slice::from_raw_parts(*v as *const u8, *vlen);
                res.insert(k.to_owned(), v.to_owned());
            }
        }
        Ok(res)
    }

    fn to_strings_map(&self) -> Result<HashMap<String, String>, String> {
        let mut res = HashMap::new();
        unsafe {
            let keys = slice::from_raw_parts(self.keys, self.size);
            let values = slice::from_raw_parts(self.values, self.size);
            for i in 0..self.size {
                let k = try!(ptr_to_string(keys[i]));
                let v = try!(ptr_to_string(values[i]));
                res.insert(k, v);
            }
        }
        Ok(res)
    }
}

#[repr(C)]
struct TablePropertiesContext {
    data_size: uint64_t,
    index_size: uint64_t,
    filter_size: uint64_t,
    raw_key_size: uint64_t,
    raw_value_size: uint64_t,
    num_data_blocks: uint64_t,
    num_entries: uint64_t,
    format_version: uint64_t,
    fixed_key_len: uint64_t,
    column_family_id: uint64_t,
    column_family_name: *const c_char,
    filter_policy_name: *const c_char,
    comparator_name: *const c_char,
    merge_operator_name: *const c_char,
    prefix_extractor_name: *const c_char,
    property_collectors_names: *const c_char,
    compression_name: *const c_char,
    user_collected_properties: CShallowStringsMap,
    readable_properties: CShallowStringsMap,
}

#[repr(C)]
pub struct TablePropertiesCollectionContext {
    inner: *mut c_void,
    size: size_t,
    keys: *const *const c_char,
    values: *const TablePropertiesContext,
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
        unsafe {
            let ctx = &*(self.inner as *mut TablePropertiesCollectionContext);
            let keys = slice::from_raw_parts(ctx.keys, ctx.size);
            let values = slice::from_raw_parts(ctx.values, ctx.size);
            for i in 0..ctx.size {
                let props = &values[i];
                let k = ptr_to_string(keys[i])?;
                let v = TableProperties {
                    data_size: props.data_size,
                    index_size: props.index_size,
                    filter_size: props.filter_size,
                    raw_key_size: props.raw_key_size,
                    raw_value_size: props.raw_value_size,
                    num_data_blocks: props.num_data_blocks,
                    num_entries: props.num_entries,
                    format_version: props.format_version,
                    fixed_key_len: props.fixed_key_len,
                    column_family_id: props.column_family_id,
                    column_family_name: try!(ptr_to_string(props.column_family_name)),
                    filter_policy_name: try!(ptr_to_string(props.filter_policy_name)),
                    comparator_name: try!(ptr_to_string(props.comparator_name)),
                    merge_operator_name: try!(ptr_to_string(props.merge_operator_name)),
                    prefix_extractor_name: try!(ptr_to_string(props.prefix_extractor_name)),
                    property_collectors_names: try!(ptr_to_string(props.property_collectors_names)),
                    compression_name: try!(ptr_to_string(props.compression_name)),
                    user_collected_properties: try!(props.user_collected_properties.to_bytes_map()),
                    readable_properties: try!(props.readable_properties.to_strings_map()),
                };
                collection.insert(k, v);
            }
        }
        Ok(collection)
    }
}
