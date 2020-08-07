// Copyright 2020 Lucjan Suski
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

mod util;

use pretty_assertions::assert_eq;

use rocksdb::{Error, Options, SstFileWriter, DB};
use util::DBPath;

#[test]
fn sst_file_writer_works() {
    let db_path = DBPath::new("_rust_rocksdb_sstfilewritertest");
    let dir = tempfile::Builder::new()
        .prefix("_rust_rocksdb_sstfilewritertest")
        .tempdir()
        .expect("Failed to create temporary path for file writer.");
    let writer_path = dir.path().join("filewriter");
    {
        let opts = Options::default();
        let mut writer = SstFileWriter::create(&opts);
        writer.open(&writer_path).unwrap();
        writer.put(b"k1", b"v1").unwrap();

        writer.put(b"k2", b"v2").unwrap();

        writer.delete(b"k3").unwrap();
        writer.finish().unwrap();
        assert!(writer.file_size() > 0);
    }
    {
        let db = DB::open_default(&db_path).unwrap();
        db.put(b"k3", b"v3").unwrap();
        db.ingest_external_file(vec![&writer_path]).unwrap();
        let r: Result<Option<Vec<u8>>, Error> = db.get(b"k1");
        assert_eq!(r.unwrap().unwrap(), b"v1");
        let r: Result<Option<Vec<u8>>, Error> = db.get(b"k2");
        assert_eq!(r.unwrap().unwrap(), b"v2");
        assert!(db.get(b"k3").unwrap().is_none());
    }
}
