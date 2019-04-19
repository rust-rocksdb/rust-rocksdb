use crate::{
    ffi_util::{opt_bytes_to_ptr, to_cpath},
    handle::Handle,
    open_raw::{OpenRaw, OpenRawFFI},
    ops,
    write_batch::WriteBatch,
    ColumnFamily, Error, Options, Transaction, WriteOptions,
};
use ffi;
use libc::{c_char, c_uchar, c_void, size_t};
use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::path::{Path, PathBuf};
use std::ptr;
use std::slice;
use std::str;

pub struct OptimisticTransactionDB {
    inner: *mut ffi::rocksdb_optimistictransactiondb_t,
    path: PathBuf,
    cfs: BTreeMap<String, ColumnFamily>,
    base_db: *mut ffi::rocksdb_t,
}

impl Handle<ffi::rocksdb_optimistictransactiondb_t> for OptimisticTransactionDB {
    fn handle(&self) -> *mut ffi::rocksdb_optimistictransactiondb_t {
        self.inner
    }
}

impl ops::Open for OptimisticTransactionDB {}
impl ops::OpenCF for OptimisticTransactionDB {}

impl OpenRaw for OptimisticTransactionDB {
    type Pointer = ffi::rocksdb_optimistictransactiondb_t;
    type Descriptor = ();

    fn open_ffi(input: OpenRawFFI<'_, Self::Descriptor>) -> Result<*mut Self::Pointer, Error> {
        let pointer = unsafe {
            if input.num_column_families <= 0 {
                ffi_try!(ffi::rocksdb_optimistictransactiondb_open(
                    input.options,
                    input.path,
                ))
            } else {
                ffi_try!(ffi::rocksdb_optimistictransactiondb_open_column_families(
                    input.options,
                    input.path,
                    input.num_column_families,
                    input.column_family_names,
                    input.column_family_options,
                    input.column_family_handles,
                ))
            }
        };

        Ok(pointer)
    }

    fn build<I>(
        path: PathBuf,
        _open_descriptor: Self::Descriptor,
        pointer: *mut Self::Pointer,
        column_families: I,
    ) -> Result<Self, Error>
    where
        I: IntoIterator<Item = (String, *mut ffi::rocksdb_column_family_handle_t)>,
    {
        let cfs: BTreeMap<_, _> = column_families
            .into_iter()
            .map(|(k, h)| (k, ColumnFamily::new(h)))
            .collect();
        let base_db = unsafe { ffi::rocksdb_optimistictransactiondb_get_base_db(pointer) };
        Ok(OptimisticTransactionDB {
            inner: pointer,
            cfs: cfs,
            path,
            base_db,
        })
    }
}

impl ops::Read for OptimisticTransactionDB {}
impl ops::Write for OptimisticTransactionDB {}

unsafe impl Send for OptimisticTransactionDB {}
unsafe impl Sync for OptimisticTransactionDB {}

impl OptimisticTransactionDB {
    pub fn list_cf<P: AsRef<Path>>(opts: &Options, path: P) -> Result<Vec<String>, Error> {
        let cpath = to_cpath(
            path,
            "Failed to convert path to CString when opening database.",
        )?;
        let mut length = 0;

        unsafe {
            let ptr = ffi_try!(ffi::rocksdb_list_column_families(
                opts.inner,
                cpath.as_ptr() as *const _,
                &mut length,
            ));

            let vec = slice::from_raw_parts(ptr, length)
                .iter()
                .map(|ptr| CStr::from_ptr(*ptr).to_string_lossy().into_owned())
                .collect();
            ffi::rocksdb_list_column_families_destroy(ptr, length);
            Ok(vec)
        }
    }

    pub fn destroy<P: AsRef<Path>>(opts: &Options, path: P) -> Result<(), Error> {
        let cpath = to_cpath(
            path,
            "Failed to convert path to CString when opening database.",
        )?;
        unsafe {
            ffi_try!(ffi::rocksdb_destroy_db(opts.inner, cpath.as_ptr(),));
        }
        Ok(())
    }

    pub fn repair<P: AsRef<Path>>(opts: Options, path: P) -> Result<(), Error> {
        let cpath = to_cpath(
            path,
            "Failed to convert path to CString when opening database.",
        )?;
        unsafe {
            ffi_try!(ffi::rocksdb_repair_db(opts.inner, cpath.as_ptr(),));
        }
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path.as_path()
    }

    pub fn write_opt(&self, batch: WriteBatch, writeopts: &WriteOptions) -> Result<(), Error> {
        unsafe {
            ffi_try!(ffi::rocksdb_write(
                self.base_db,
                writeopts.inner,
                batch.inner,
            ));
        }
        Ok(())
    }

    pub fn write(&self, batch: WriteBatch) -> Result<(), Error> {
        self.write_opt(batch, &WriteOptions::default())
    }

    pub fn write_without_wal(&self, batch: WriteBatch) -> Result<(), Error> {
        let mut wo = WriteOptions::new();
        wo.disable_wal(true);
        self.write_opt(batch, &wo)
    }

    pub fn create_cf<N: AsRef<str>>(&mut self, name: N, opts: &Options) -> Result<(), Error> {
        let cname = match CString::new(name.as_ref().as_bytes()) {
            Ok(c) => c,
            Err(_) => {
                return Err(Error::new(
                    "Failed to convert path to CString \
                     when opening rocksdb"
                        .to_owned(),
                ));
            }
        };
        unsafe {
            let cf_handle = ffi_try!(ffi::rocksdb_create_column_family(
                self.base_db,
                opts.inner,
                cname.as_ptr(),
            ));

            self.cfs
                .insert(name.as_ref().to_string(), ColumnFamily::new(cf_handle));
        };
        Ok(())
    }

    pub fn drop_cf(&mut self, name: &str) -> Result<(), Error> {
        if let Some(cf) = self.cfs.remove(name) {
            unsafe {
                ffi_try!(ffi::rocksdb_drop_column_family(self.base_db, cf.inner,));
            }
            Ok(())
        } else {
            Err(Error::new(
                format!("Invalid column family: {}", name).to_owned(),
            ))
        }
    }

    /// Return the underlying column family handle.
    pub fn cf_handle(&self, name: &str) -> Option<&ColumnFamily> {
        self.cfs.get(name)
    }

    pub fn merge_opt<K, V>(&self, key: K, value: V, writeopts: &WriteOptions) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let key = key.as_ref();
        let value = value.as_ref();

        unsafe {
            ffi_try!(ffi::rocksdb_merge(
                self.base_db,
                writeopts.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
                value.as_ptr() as *const c_char,
                value.len() as size_t,
            ));
            Ok(())
        }
    }

    pub fn merge_cf_opt<K, V>(
        &self,
        cf: ColumnFamily,
        key: K,
        value: V,
        writeopts: &WriteOptions,
    ) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let key = key.as_ref();
        let value = value.as_ref();

        unsafe {
            ffi_try!(ffi::rocksdb_merge_cf(
                self.base_db,
                writeopts.inner,
                cf.inner,
                key.as_ptr() as *const c_char,
                key.len() as size_t,
                value.as_ptr() as *const c_char,
                value.len() as size_t,
            ));
            Ok(())
        }
    }

    pub fn merge<K, V>(&self, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.merge_opt(key.as_ref(), value.as_ref(), &WriteOptions::default())
    }

    pub fn merge_cf<K, V>(&self, cf: ColumnFamily, key: K, value: V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.merge_cf_opt(cf, key.as_ref(), value.as_ref(), &WriteOptions::default())
    }

    pub fn compact_range<S: AsRef<[u8]>, E: AsRef<[u8]>>(&self, start: Option<S>, end: Option<E>) {
        unsafe {
            let start = start.as_ref().map(|s| s.as_ref());
            let end = end.as_ref().map(|e| e.as_ref());

            ffi::rocksdb_compact_range(
                self.base_db,
                opt_bytes_to_ptr(start),
                start.map_or(0, |s| s.len()) as size_t,
                opt_bytes_to_ptr(end),
                end.map_or(0, |e| e.len()) as size_t,
            );
        }
    }

    pub fn compact_range_cf(&self, cf: ColumnFamily, start: Option<&[u8]>, end: Option<&[u8]>) {
        unsafe {
            ffi::rocksdb_compact_range_cf(
                self.base_db,
                cf.inner,
                opt_bytes_to_ptr(start),
                start.map_or(0, |s| s.len()) as size_t,
                opt_bytes_to_ptr(end),
                end.map_or(0, |e| e.len()) as size_t,
            );
        }
    }

    pub fn set_options(&self, opts: &[(&str, &str)]) -> Result<(), Error> {
        let copts = opts
            .iter()
            .map(|(name, value)| {
                let cname = match CString::new(name.as_bytes()) {
                    Ok(cname) => cname,
                    Err(e) => return Err(Error::new(format!("Invalid option name `{}`", e))),
                };
                let cvalue = match CString::new(value.as_bytes()) {
                    Ok(cvalue) => cvalue,
                    Err(e) => return Err(Error::new(format!("Invalid option value: `{}`", e))),
                };
                Ok((cname, cvalue))
            })
            .collect::<Result<Vec<(CString, CString)>, Error>>()?;

        let cnames: Vec<*const c_char> = copts.iter().map(|opt| opt.0.as_ptr()).collect();
        let cvalues: Vec<*const c_char> = copts.iter().map(|opt| opt.1.as_ptr()).collect();
        let count = opts.len() as i32;
        unsafe {
            ffi_try!(ffi::rocksdb_set_options(
                self.base_db,
                count,
                cnames.as_ptr(),
                cvalues.as_ptr(),
            ));
        }
        Ok(())
    }

    /// Retrieves a RocksDB property by name.
    ///
    /// For a full list of properties, see
    /// https://github.com/facebook/rocksdb/blob/08809f5e6cd9cc4bc3958dd4d59457ae78c76660/include/rocksdb/db.h#L428-L634
    pub fn property_value(&self, name: &str) -> Result<Option<String>, Error> {
        let prop_name = match CString::new(name) {
            Ok(c) => c,
            Err(e) => {
                return Err(Error::new(format!(
                    "Failed to convert property name to CString: {}",
                    e
                )));
            }
        };

        unsafe {
            let value = ffi::rocksdb_property_value(self.base_db, prop_name.as_ptr());
            if value.is_null() {
                return Ok(None);
            }

            let str_value = match CStr::from_ptr(value).to_str() {
                Ok(s) => s.to_owned(),
                Err(e) => {
                    return Err(Error::new(format!(
                        "Failed to convert property value to string: {}",
                        e
                    )));
                }
            };

            ffi::rocksdb_free(value as *mut c_void);
            Ok(Some(str_value))
        }
    }

    /// Retrieves a RocksDB property by name, for a specific column family.
    ///
    /// For a full list of properties, see
    /// https://github.com/facebook/rocksdb/blob/08809f5e6cd9cc4bc3958dd4d59457ae78c76660/include/rocksdb/db.h#L428-L634
    pub fn property_value_cf(&self, cf: ColumnFamily, name: &str) -> Result<Option<String>, Error> {
        let prop_name = match CString::new(name) {
            Ok(c) => c,
            Err(e) => {
                return Err(Error::new(format!(
                    "Failed to convert property name to CString: {}",
                    e
                )));
            }
        };

        unsafe {
            let value = ffi::rocksdb_property_value_cf(self.base_db, cf.inner, prop_name.as_ptr());
            if value.is_null() {
                return Ok(None);
            }

            let str_value = match CStr::from_ptr(value).to_str() {
                Ok(s) => s.to_owned(),
                Err(e) => {
                    return Err(Error::new(format!(
                        "Failed to convert property value to string: {}",
                        e
                    )));
                }
            };

            libc::free(value as *mut c_void);
            Ok(Some(str_value))
        }
    }

    /// Retrieves a RocksDB property and casts it to an integer.
    ///
    /// For a full list of properties that return int values, see
    /// https://github.com/facebook/rocksdb/blob/08809f5e6cd9cc4bc3958dd4d59457ae78c76660/include/rocksdb/db.h#L654-L689
    pub fn property_int_value(&self, name: &str) -> Result<Option<u64>, Error> {
        match self.property_value(name) {
            Ok(Some(value)) => match value.parse::<u64>() {
                Ok(int_value) => Ok(Some(int_value)),
                Err(e) => Err(Error::new(format!(
                    "Failed to convert property value to int: {}",
                    e
                ))),
            },
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Retrieves a RocksDB property for a specific column family and casts it to an integer.
    ///
    /// For a full list of properties that return int values, see
    /// https://github.com/facebook/rocksdb/blob/08809f5e6cd9cc4bc3958dd4d59457ae78c76660/include/rocksdb/db.h#L654-L689
    pub fn property_int_value_cf(
        &self,
        cf: ColumnFamily,
        name: &str,
    ) -> Result<Option<u64>, Error> {
        match self.property_value_cf(cf, name) {
            Ok(Some(value)) => match value.parse::<u64>() {
                Ok(int_value) => Ok(Some(int_value)),
                Err(e) => Err(Error::new(format!(
                    "Failed to convert property value to int: {}",
                    e
                ))),
            },
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

impl Drop for OptimisticTransactionDB {
    fn drop(&mut self) {
        unsafe {
            for cf in self.cfs.values() {
                ffi::rocksdb_column_family_handle_destroy(cf.inner);
            }
            ffi::rocksdb_optimistictransactiondb_close_base_db(self.base_db);
            ffi::rocksdb_optimistictransactiondb_close(self.inner);
        }
    }
}

impl ops::TransactionBegin for OptimisticTransactionDB {
    type WriteOptions = WriteOptions;
    type TransactionOptions = OptimisticTransactionOptions;
    fn transaction(
        &self,
        write_options: &WriteOptions,
        tx_options: &OptimisticTransactionOptions,
    ) -> Transaction<OptimisticTransactionDB> {
        unsafe {
            let inner = ffi::rocksdb_optimistictransaction_begin(
                self.inner,
                write_options.inner,
                tx_options.inner,
                ptr::null_mut(),
            );
            Transaction::new(inner)
        }
    }
}

pub struct OptimisticTransactionOptions {
    inner: *mut ffi::rocksdb_optimistictransaction_options_t,
}

impl OptimisticTransactionOptions {
    /// Create new optimistic transaction options
    pub fn new() -> OptimisticTransactionOptions {
        unsafe {
            let inner = ffi::rocksdb_optimistictransaction_options_create();
            OptimisticTransactionOptions { inner }
        }
    }

    /// Set a snapshot at start of transaction by setting set_snapshot=true
    /// Default: false
    pub fn set_snapshot(&mut self, set_snapshot: bool) {
        unsafe {
            ffi::rocksdb_optimistictransaction_options_set_set_snapshot(
                self.inner,
                set_snapshot as c_uchar,
            );
        }
    }
}

impl Drop for OptimisticTransactionOptions {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_optimistictransaction_options_destroy(self.inner);
        }
    }
}

impl Default for OptimisticTransactionOptions {
    fn default() -> OptimisticTransactionOptions {
        OptimisticTransactionOptions::new()
    }
}
