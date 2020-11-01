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
//

// PIGMED operations (Put, Iterate, Get, Merge, Delete)
#[macro_use]
pub(crate) mod backup;
#[macro_use]
pub(crate) mod checkpoint;
#[macro_use]
pub(crate) mod column_family;
#[macro_use]
pub(crate) mod compact_range;
#[macro_use]
pub(crate) mod delete;
#[macro_use]
pub(crate) mod delete_file_in_range;
#[macro_use]
pub(crate) mod delete_range;
#[macro_use]
pub(crate) mod flush;
#[macro_use]
pub(crate) mod get;
#[macro_use]
pub(crate) mod get_pinned;
#[macro_use]
pub(crate) mod ingest_external_file;
#[macro_use]
pub(crate) mod iterate;
#[macro_use]
pub(crate) mod merge;
#[macro_use]
pub(crate) mod perf;
#[macro_use]
pub(crate) mod property;
#[macro_use]
pub(crate) mod put;
#[macro_use]
pub(crate) mod set_options;
#[macro_use]
pub(crate) mod snapshot;
#[macro_use]
pub(crate) mod write_batch;

pub(crate) use self::backup::BackupInternal;
pub(crate) use self::checkpoint::CheckpointInternal;
pub(crate) use self::column_family::GetColumnFamilies;
pub use self::column_family::{CreateColumnFamily, DropColumnFamily, GetColumnFamily};
pub use self::compact_range::{CompactRange, CompactRangeCF, CompactRangeCFOpt, CompactRangeOpt};
pub use self::delete::{Delete, DeleteCF, DeleteCFOpt, DeleteOpt};
pub use self::delete_file_in_range::{DeleteFileInRange, DeleteFileInRangeCF};
pub use self::delete_range::{DeleteRangeCF, DeleteRangeCFOpt};
pub use self::flush::{Flush, FlushCF, FlushCFOpt, FlushOpt};
pub use self::get::{Get, GetCF, GetCFOpt, GetOpt};
pub use self::get_pinned::{GetPinned, GetPinnedCF, GetPinnedCFOpt, GetPinnedOpt};
pub use self::ingest_external_file::{
    IngestExternalFile, IngestExternalFileCF, IngestExternalFileCFOpt, IngestExternalFileOpt,
};
pub use self::iterate::{Iterate, IterateCF};
pub use self::merge::{Merge, MergeCF, MergeCFOpt, MergeOpt};
pub(crate) use self::perf::PerfInternal;
pub use self::property::{GetProperty, GetPropertyCF};
pub use self::put::{Put, PutCF, PutCFOpt, PutOpt};
pub use self::set_options::SetOptions;
pub(crate) use self::snapshot::SnapshotInternal;
pub use self::write_batch::{WriteBatchWrite, WriteBatchWriteOpt};
