use crate::{ColumnFamilyDescriptor, Error, Options, Transaction, WriteOptions, DB};
use ffi;
use libc::{c_int, c_uchar};
use std::collections::BTreeMap;
use std::ffi::CString;
use std::fs;
use std::path::{Path, PathBuf};
use std::path::PathBuf;
use std::ptr;
use std::str;
use std::sync::{Arc, RwLock};

pub struct OptimisticTransactionDB {
    inner: *mut ffi::rocksdb_optimistictransactiondb_t,
    path: PathBuf,
    base_db: DB,
}

impl OptimisticTransactionDB {
    /// Open a database with default options.
    pub fn open_default<P: AsRef<Path>>(path: P) -> Result<OptimisticTransactionDB, Error> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        OptimisticTransactionDB::open(&opts, path)
    }

    /// Open the database with the specified options.
    pub fn open<P: AsRef<Path>>(opts: &Options, path: P) -> Result<OptimisticTransactionDB, Error> {
        OptimisticTransactionDB::open_cf(opts, path, None::<&str>)
    }

    /// Open a database with the given database options and column family names.
    ///
    /// Column families opened using this function will be created with default `Options`.
    pub fn open_cf<P, I, N>(
        opts: &Options,
        path: P,
        cfs: I,
    ) -> Result<OptimisticTransactionDB, Error>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = N>,
        N: AsRef<str>,
    {
        let cfs = cfs
            .into_iter()
            .map(|name| ColumnFamilyDescriptor::new(name.as_ref(), Options::default()));
        OptimisticTransactionDB::open_cf_descriptors(opts, path, cfs)
    }

    /// Open a database with the given database options and column family names/options.
    pub fn open_cf_descriptors<P, I>(
        opts: &Options,
        path: P,
        cfs: I,
    ) -> Result<OptimisticTransactionDB, Error>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = ColumnFamilyDescriptor>,
    {
        let path = path.as_ref();
        let cpath = match CString::new(path.to_string_lossy().as_bytes()) {
            Ok(c) => c,
            Err(_) => {
                return Err(Error::new(
                    "Failed to convert path to CString \
                     when opening DB."
                        .to_owned(),
                ));
            }
        };

        if let Err(e) = fs::create_dir_all(&path) {
            return Err(Error::new(format!(
                "Failed to create RocksDB\
                 directory: `{:?}`.",
                e
            )));
        }

        let db: *mut ffi::rocksdb_optimistictransactiondb_t;
        let cf_map = Arc::new(RwLock::new(BTreeMap::new()));
        let mut cfs: Vec<_> = cfs.into_iter().collect();
        if cfs.is_empty() {
            unsafe {
                db = ffi_try!(ffi::rocksdb_optimistictransactiondb_open(
                    opts.inner,
                    cpath.as_ptr() as *const _,
                ));
            }
        } else {
            // Always open the default column family.
            if !cfs
                .iter()
                .any(|cf: &ColumnFamilyDescriptor| cf.name == "default")
            {
                cfs.push(ColumnFamilyDescriptor {
                    name: String::from("default"),
                    options: Options::default(),
                });
            }
            // We need to store our CStrings in an intermediate vector
            // so that their pointers remain valid.
            let c_cfs: Vec<CString> = cfs
                .iter()
                .map(|cf| CString::new(cf.name.as_bytes()).unwrap())
                .collect();

            let mut cf_names: Vec<_> = c_cfs.iter().map(|cf| cf.as_ptr()).collect();

            // These handles will be populated by DB.
            let mut cf_handles: Vec<_> = cfs.iter().map(|_| ptr::null_mut()).collect();

            let mut cf_opts: Vec<_> = cfs.iter().map(|cf| cf.options.inner as *const _).collect();

            unsafe {
                db = ffi_try!(ffi::rocksdb_optimistictransactiondb_open_column_families(
                    opts.inner,
                    cpath.as_ptr(),
                    cfs.len() as c_int,
                    cf_names.as_mut_ptr(),
                    cf_opts.as_mut_ptr(),
                    cf_handles.as_mut_ptr(),
                ));
            }

            for handle in &cf_handles {
                if handle.is_null() {
                    return Err(Error::new(
                        "Received null column family \
                         handle from DB."
                            .to_owned(),
                    ));
                }
            }

            for (n, h) in cfs.iter().zip(cf_handles) {
                cf_map
                    .write()
                    .map_err(|e| Error::new(e.to_string()))?
                    .insert(n.name.clone(), h);
            }
        }

        if db.is_null() {
            return Err(Error::new("Could not initialize database.".to_owned()));
        };

        let path_buf = path.to_path_buf();

        let base_db = unsafe {
            let inner = ffi::rocksdb_optimistictransactiondb_get_base_db(db);
            DB {
                inner,
                cfs: cf_map,
                path: path_buf.clone(),
                is_base_db: true,
            }
        };

        Ok(OptimisticTransactionDB {
            inner: db,
            path: path_buf,
            base_db,
        })
    }

    /// Begins a new optimistic transaction.
    pub fn transaction(
        &self,
        write_options: &WriteOptions,
        optimistic_tx_options: &OptimisticTransactionOptions,
    ) -> Transaction {
        unsafe {
            let inner = ffi::rocksdb_optimistictransaction_begin(
                self.inner,
                write_options.inner,
                optimistic_tx_options.inner,
                ptr::null_mut(),
            );
            let snapshot = if optimistic_tx_options.set_snapshot {
                Some(ffi::rocksdb_transaction_get_snapshot(inner))
            } else {
                None
            };
            Transaction::new(inner, snapshot)
        }
    }

    /// Begins a new optimistic transaction with default options.
    pub fn transaction_default(&self) -> Transaction {
        let write_options = WriteOptions::default();
        let optimistic_transaction_options = OptimisticTransactionOptions::new();
        self.transaction(&write_options, &optimistic_transaction_options)
    }

    // Get database path
    pub fn path(&self) -> &Path {
        &self.path.as_path()
    }

    /// Get base DB
    pub fn get_base_db(&self) -> &DB {
        &self.base_db
    }
}

impl Drop for OptimisticTransactionDB {
    fn drop(&mut self) {
        self.base_db.drop_base_db();
        unsafe {
            ffi::rocksdb_optimistictransactiondb_close(self.inner);
        }
    }
}

pub struct OptimisticTransactionOptions {
    inner: *mut ffi::rocksdb_optimistictransaction_options_t,
    set_snapshot: bool,
}

impl OptimisticTransactionOptions {
    /// Create new optimistic transaction options
    pub fn new() -> OptimisticTransactionOptions {
        unsafe {
            let inner = ffi::rocksdb_optimistictransaction_options_create();
            OptimisticTransactionOptions {
                inner,
                set_snapshot: false,
            }
        }
    }

    /// mode snapshot switch.
    /// Default: false
    pub fn set_snapshot(&mut self, set_snapshot: bool) {
        unsafe {
            ffi::rocksdb_optimistictransaction_options_set_set_snapshot(
                self.inner,
                set_snapshot as c_uchar,
            );
        }
        self.set_snapshot = set_snapshot;
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
