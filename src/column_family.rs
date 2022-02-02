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

use crate::{db::MultiThreaded, ffi, Options};

use std::sync::Arc;

/// The name of the default column family.
///
/// The column family with this name is created implicitly whenever column
/// families are used.
pub const DEFAULT_COLUMN_FAMILY_NAME: &str = "default";

/// A descriptor for a RocksDB column family.
///
/// A description of the column family, containing the name and `Options`.
pub struct ColumnFamilyDescriptor {
    pub(crate) name: String,
    pub(crate) options: Options,
}

impl ColumnFamilyDescriptor {
    // Create a new column family descriptor with the specified name and options.
    pub fn new<S>(name: S, options: Options) -> Self
    where
        S: Into<String>,
    {
        Self {
            name: name.into(),
            options,
        }
    }
}

/// An opaque type used to represent a column family. Returned from some functions, and used
/// in others
pub struct ColumnFamily {
    pub(crate) inner: *mut ffi::rocksdb_column_family_handle_t,
}

/// A specialized opaque type used to represent a column family by the [`MultiThreaded`]
/// mode. Clone (and Copy) is derived to behave like `&ColumnFamily` (this is used for
/// single-threaded mode). `Clone`/`Copy` is safe because this lifetime is bound to DB like
/// iterators/snapshots. On top of it, this is as cheap and small as `&ColumnFamily` because
/// this only has a single pointer-wide field.
pub struct BoundColumnFamily<'a> {
    pub(crate) inner: *mut ffi::rocksdb_column_family_handle_t,
    pub(crate) multi_threaded_cfs: std::marker::PhantomData<&'a MultiThreaded>,
}

// internal struct which isn't exposed to public api.
// but its memory will be exposed after transmute()-ing to BoundColumnFamily.
// ColumnFamily's lifetime should be bound to DB. But, db holds cfs and cfs can't easily
// self-reference DB as its lifetime due to rust's type system
pub(crate) struct UnboundColumnFamily {
    pub(crate) inner: *mut ffi::rocksdb_column_family_handle_t,
}

impl UnboundColumnFamily {
    pub(crate) fn bound_column_family<'a>(self: Arc<Self>) -> Arc<BoundColumnFamily<'a>> {
        // SAFETY: the new BoundColumnFamily here just adding lifetime,
        // so that column family handle won't outlive db.
        unsafe { Arc::from_raw(Arc::into_raw(self).cast()) }
    }
}

fn destroy_handle(handle: *mut ffi::rocksdb_column_family_handle_t) {
    // SAFETY: This should be called only from various Drop::drop(), strictly keeping a 1-to-1
    // ownership to avoid double invocation to the rocksdb function with same handle.
    unsafe {
        ffi::rocksdb_column_family_handle_destroy(handle);
    }
}

impl Drop for ColumnFamily {
    fn drop(&mut self) {
        destroy_handle(self.inner);
    }
}

// these behaviors must be identical between BoundColumnFamily and UnboundColumnFamily
// due to the unsafe transmute() in bound_column_family()!
impl<'a> Drop for BoundColumnFamily<'a> {
    fn drop(&mut self) {
        destroy_handle(self.inner);
    }
}

impl Drop for UnboundColumnFamily {
    fn drop(&mut self) {
        destroy_handle(self.inner);
    }
}

/// Handy type alias to hide actual type difference to reference [`ColumnFamily`]
/// depending on the `multi-threaded-cf` crate feature.
#[cfg(not(feature = "multi-threaded-cf"))]
pub type ColumnFamilyRef<'a> = &'a ColumnFamily;

#[cfg(feature = "multi-threaded-cf")]
pub type ColumnFamilyRef<'a> = Arc<BoundColumnFamily<'a>>;

/// Utility trait to accept both supported references to `ColumnFamily`
/// (`&ColumnFamily` and `BoundColumnFamily`)
pub trait AsColumnFamilyRef {
    fn inner(&self) -> *mut ffi::rocksdb_column_family_handle_t;
}

impl AsColumnFamilyRef for ColumnFamily {
    fn inner(&self) -> *mut ffi::rocksdb_column_family_handle_t {
        self.inner
    }
}

impl<'a> AsColumnFamilyRef for &'a ColumnFamily {
    fn inner(&self) -> *mut ffi::rocksdb_column_family_handle_t {
        self.inner
    }
}

// Only implement for Arc-ed BoundColumnFamily as this tightly coupled and
// implementation detail, considering use of std::mem::transmute. BoundColumnFamily
// isn't expected to be used as naked.
// Also, ColumnFamilyRef might not be Arc<BoundColumnFamily<'a>> depending crate
// feature flags so, we can't use the type alias here.
impl<'a> AsColumnFamilyRef for Arc<BoundColumnFamily<'a>> {
    fn inner(&self) -> *mut ffi::rocksdb_column_family_handle_t {
        self.inner
    }
}

unsafe impl Send for ColumnFamily {}
unsafe impl Send for UnboundColumnFamily {}
unsafe impl Sync for UnboundColumnFamily {}
unsafe impl<'a> Send for BoundColumnFamily<'a> {}
