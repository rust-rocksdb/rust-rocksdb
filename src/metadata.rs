// Copyright 2020 Tyler Neely
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
use crate::{
    ffi,
    ffi_util::{from_cstr, raw_data},
};
use libc::c_void;

/// The metadata that describes a column family.
pub struct ColumnFamilyMetaData {
    /// Name of the column family
    pub name: String,
    /// The size of all sst files under this column family
    pub size: u64,
    /// The number of files under this column family
    pub file_count: usize,
    /// metadata of each level under this column family
    pub levels: Vec<LevelMetaData>,
}

impl ColumnFamilyMetaData {
    pub(crate) unsafe fn from_c(ptr: *mut ffi::rocksdb_column_family_metadata_t) -> Self {
        let name = from_cstr(ffi::rocksdb_column_family_metadata_get_name(ptr));
        let size = ffi::rocksdb_column_family_metadata_get_size(ptr);
        let file_count = ffi::rocksdb_column_family_metadata_get_file_count(ptr);
        let level_count = ffi::rocksdb_column_family_metadata_get_level_count(ptr);

        let mut levels: Vec<LevelMetaData> = Vec::new();
        for i in 0..level_count {
            levels.push(LevelMetaData::from_c(
                ffi::rocksdb_column_family_metadata_get_level_metadata(ptr, i),
            ));
        }
        ffi::rocksdb_column_family_metadata_destroy(ptr);

        Self {
            name,
            size,
            file_count,
            levels,
        }
    }
}

/// The metadata that describes a level.
pub struct LevelMetaData {
    /// The level of the current LevelMetaData.
    pub level: i32,
    /// The sst files under the current LevelMetaData.
    pub files: Vec<SstFileMetaData>,
}

impl LevelMetaData {
    unsafe fn from_c(ptr: *mut ffi::rocksdb_level_metadata_t) -> Self {
        let level = ffi::rocksdb_level_metadata_get_level(ptr);
        let file_count = ffi::rocksdb_level_metadata_get_file_count(ptr);
        let mut files: Vec<SstFileMetaData> = Vec::new();
        for i in 0..file_count {
            files.push(SstFileMetaData::from_c(
                ffi::rocksdb_level_metadata_get_sst_file_metadata(ptr, i),
            ));
        }
        ffi::rocksdb_level_metadata_destroy(ptr);

        Self { level, files }
    }
}

/// The metadata that describes a sst file.
pub struct SstFileMetaData {
    /// The relative file name
    pub name: String,
    /// Size of the file
    pub size: u64,
    /// Smallest user defined key in the file
    pub smallest_key: Option<Vec<u8>>,
    /// Largest user defined key in the file
    pub largest_key: Option<Vec<u8>>,
}

impl SstFileMetaData {
    unsafe fn from_c(ptr: *mut ffi::rocksdb_sst_file_metadata_t) -> Self {
        let name = from_cstr(ffi::rocksdb_sst_file_metadata_get_relative_filename(ptr));
        let size = ffi::rocksdb_sst_file_metadata_get_size(ptr);

        let mut key_size: usize = 0;

        let smallest_key_raw = ffi::rocksdb_sst_file_metadata_get_smallestkey(ptr, &mut key_size);
        let smallest_key = raw_data(smallest_key_raw, key_size);
        ffi::rocksdb_free(smallest_key_raw as *mut c_void);

        let largest_key_raw = ffi::rocksdb_sst_file_metadata_get_largestkey(ptr, &mut key_size);
        let largest_key = raw_data(largest_key_raw, key_size);
        ffi::rocksdb_free(largest_key_raw as *mut c_void);

        ffi::rocksdb_sst_file_metadata_destroy(ptr);
        Self {
            name,
            size,
            smallest_key,
            largest_key,
        }
    }
}
