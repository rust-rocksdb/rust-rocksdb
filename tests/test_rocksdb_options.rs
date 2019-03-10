// Copyright 2014 Tyler Neely
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
//
extern crate rocksdb;

use rocksdb::{prelude::*, TemporaryDBPath};

#[test]
fn test_set_num_levels() {
    let n = TemporaryDBPath::new();
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_num_levels(2);
        let _db = DB::open(&opts, &n).unwrap();
    }
}

#[test]
fn test_increase_parallelism() {
    let n = TemporaryDBPath::new();
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.increase_parallelism(4);
        let _db = DB::open(&opts, &n).unwrap();
    }
}
