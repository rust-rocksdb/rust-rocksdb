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
mod columnfamily;
mod delete;
mod get;
mod get_pinned;
mod merge;
mod put;
mod writebatch;

mod open;

mod checkpoint;
mod compact;
mod iter;
mod property;
mod transaction;

pub use self::delete::{Delete, DeleteCF};
pub use self::get::{Get, GetCF};
pub use self::get_pinned::{GetPinned, GetPinnedCF};
pub use self::merge::{Merge, MergeCF};
pub use self::put::{Put, PutCF};
pub use self::writebatch::WriteOps;

pub use self::open::{Open, OpenCF};

/// Marker trait for operations that leave DB
/// state unchanged
pub trait Read {}

/// Marker trait for operations that mutate
/// DB state
pub trait Write {}

pub use self::checkpoint::CreateCheckpointObject;
pub use self::columnfamily::CreateCf;
pub use self::columnfamily::DropCf;
pub use self::columnfamily::GetColumnFamilys;
pub use self::compact::{CompactRange, CompactRangeCF};
pub use self::iter::{Iterate, IterateCF};
pub use self::property::{GetProperty, GetPropertyCF, SetOptions};
pub use self::transaction::TransactionBegin;
