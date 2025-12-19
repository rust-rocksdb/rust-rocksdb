use crate::{
    db::{DBInner, DB},
    db_options::Options,
    env::Env,
};
use librocksdb_sys as ffi;

/// Trait for accessing raw pointers to underlying RocksDB objects.
///
/// This trait is only available when the `raw-ptr` feature is enabled.
/// It provides access to the underlying C API pointers for advanced use cases.
///
/// # Examples
///
/// ```rust,ignore
/// use rocksdb::{DB, Options, AsRawPtr};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let db = DB::open_default("path")?;
/// let raw_db_ptr = unsafe { db.as_raw_ptr() }; // *mut rocksdb_t
///
/// let opts = Options::default();
/// let raw_opts_ptr = unsafe { opts.as_raw_ptr() }; // *mut rocksdb_options_t
///
/// // You can now use these pointers with the C API directly
/// // unsafe { rocksdb_some_c_function(raw_db_ptr, raw_opts_ptr); }
/// # Ok(())
/// # }
/// ```
///
/// # Safety
///
/// The returned pointers are only valid as long as the Rust objects are alive.
/// You must ensure proper lifetime management and avoid using the pointers
/// after the objects have been dropped.
pub trait AsRawPtr<T> {
    /// Returns a raw pointer to the underlying RocksDB object.
    ///
    /// # Safety
    ///
    /// The returned pointer is only valid as long as the object implementing
    /// this trait is alive. The caller must ensure proper lifetime management
    /// and avoid using the pointer after the object has been dropped.
    unsafe fn as_raw_ptr(&self) -> *mut T;
}

impl AsRawPtr<ffi::rocksdb_t> for DB {
    /// Returns a raw pointer to the underlying `rocksdb_t` object.
    ///
    /// This allows direct access to the RocksDB C API for advanced use cases.
    unsafe fn as_raw_ptr(&self) -> *mut ffi::rocksdb_t {
        self.inner.inner()
    }
}

impl AsRawPtr<ffi::rocksdb_options_t> for Options {
    /// Returns a raw pointer to the underlying `rocksdb_options_t` object.
    ///
    /// This allows direct access to the RocksDB options C API for advanced use cases.
    unsafe fn as_raw_ptr(&self) -> *mut ffi::rocksdb_options_t {
        self.inner
    }
}

impl AsRawPtr<ffi::rocksdb_env_t> for Env {
    /// Returns a raw pointer to the underlying `rocksdb_env_t` object.
    ///
    /// This allows direct access to the RocksDB environment C API for advanced use cases.
    unsafe fn as_raw_ptr(&self) -> *mut ffi::rocksdb_env_t {
        self.0.inner
    }
}
