// Copyright 2019 Tyler Neely
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

use rocksdb::WriteBatch;

#[test]
fn test_write_batch_clear() {
    let mut batch = WriteBatch::default();
    let _ = batch.put(b"1", b"2");
    assert_eq!(batch.len(), 1);
    let _ = batch.clear();
    assert_eq!(batch.len(), 0);
    assert!(batch.is_empty());
}
