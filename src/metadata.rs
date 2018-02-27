// Copyright 2018 PingCAP, Inc.
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

use crocksdb_ffi::{self, DBColumnFamilyMetaData, DBLevelMetaData, DBSstFileMetaData};
use std::ffi::CStr;

pub struct ColumnFamilyMetaData {
    inner: *mut DBColumnFamilyMetaData,
}

impl ColumnFamilyMetaData {
    pub fn from_ptr(inner: *mut DBColumnFamilyMetaData) -> ColumnFamilyMetaData {
        ColumnFamilyMetaData { inner: inner }
    }

    pub fn get_levels(&self) -> Vec<LevelMetaData> {
        let mut levels = Vec::new();
        unsafe {
            let n = crocksdb_ffi::crocksdb_column_family_meta_data_level_count(self.inner);
            for i in 0..n {
                let data = crocksdb_ffi::crocksdb_column_family_meta_data_level_data(self.inner, i);
                levels.push(LevelMetaData { inner: data });
            }
        }
        levels
    }
}

impl Drop for ColumnFamilyMetaData {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_column_family_meta_data_destroy(self.inner);
        }
    }
}

pub struct LevelMetaData {
    inner: *const DBLevelMetaData,
}

impl LevelMetaData {
    pub fn get_files(&self) -> Vec<SstFileMetaData> {
        let mut files = Vec::new();
        unsafe {
            let n = crocksdb_ffi::crocksdb_level_meta_data_file_count(self.inner);
            for i in 0..n {
                let data = crocksdb_ffi::crocksdb_level_meta_data_file_data(self.inner, i);
                files.push(SstFileMetaData { inner: data });
            }
        }
        files
    }
}

pub struct SstFileMetaData {
    inner: *const DBSstFileMetaData,
}

impl SstFileMetaData {
    pub fn get_size(&self) -> usize {
        unsafe { crocksdb_ffi::crocksdb_sst_file_meta_data_size(self.inner) }
    }

    pub fn get_name(&self) -> String {
        unsafe {
            let ptr = crocksdb_ffi::crocksdb_sst_file_meta_data_name(self.inner);
            CStr::from_ptr(ptr).to_string_lossy().into_owned()
        }
    }
}
