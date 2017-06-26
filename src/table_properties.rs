use crocksdb_ffi::{self, DBTablePropertiesCollection, ptr_to_string};
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
    pub readable_properties: HashMap<String, String>,
}

pub type TablePropertiesCollection = HashMap<String, TableProperties>;

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
            let ctx = &*(self.inner as *mut DBTablePropertiesCollection);
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
