use crate::{ColumnFamilyDescriptor, Error, Options, Transaction, WriteOptions, DB};
use ffi;
use libc::{c_int, c_uchar};
use std::collections::BTreeMap;
use std::ffi::CString;
use std::fs;
use std::path::Path;
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
        OptimisticTransactionDB::open_cf(opts, path, &[])
    }

    /// Open a database with the given database options and column family names.
    ///
    /// Column families opened using this function will be created with default `Options`.
    pub fn open_cf<P: AsRef<Path>>(
        opts: &Options,
        path: P,
        cfs: &[&str],
    ) -> Result<OptimisticTransactionDB, Error> {
        let cfs_v = cfs
            .to_vec()
            .iter()
            .map(|name| ColumnFamilyDescriptor::new(*name, Options::default()))
            .collect();

        OptimisticTransactionDB::open_cf_descriptors(opts, path, cfs_v)
    }

    /// Open a database with the given database options and column family names/options.
    pub fn open_cf_descriptors<P: AsRef<Path>>(
        opts: &Options,
        path: P,
        cfs: Vec<ColumnFamilyDescriptor>,
    ) -> Result<OptimisticTransactionDB, Error> {
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
        if cfs.len() == 0 {
            unsafe {
                db = ffi_try!(ffi::rocksdb_optimistictransactiondb_open(
                    opts.inner,
                    cpath.as_ptr() as *const _,
                ));
            }
        } else {
            let mut cfs_v = cfs;
            // Always open the default column family.
            if !cfs_v.iter().any(|cf| cf.name == "default") {
                cfs_v.push(ColumnFamilyDescriptor {
                    name: String::from("default"),
                    options: Options::default(),
                });
            }
            // We need to store our CStrings in an intermediate vector
            // so that their pointers remain valid.
            let c_cfs: Vec<CString> = cfs_v
                .iter()
                .map(|cf| CString::new(cf.name.as_bytes()).unwrap())
                .collect();

            let mut cfnames: Vec<_> = c_cfs.iter().map(|cf| cf.as_ptr()).collect();

            // These handles will be populated by DB.
            let mut cfhandles: Vec<_> = cfs_v.iter().map(|_| ptr::null_mut()).collect();

            let mut cfopts: Vec<_> = cfs_v
                .iter()
                .map(|cf| cf.options.inner as *const _)
                .collect();

            unsafe {
                db = ffi_try!(ffi::rocksdb_optimistictransactiondb_open_column_families(
                    opts.inner,
                    cpath.as_ptr(),
                    cfs_v.len() as c_int,
                    cfnames.as_mut_ptr(),
                    cfopts.as_mut_ptr(),
                    cfhandles.as_mut_ptr(),
                ));
            }

            for handle in &cfhandles {
                if handle.is_null() {
                    return Err(Error::new(
                        "Received null column family \
                         handle from DB."
                            .to_owned(),
                    ));
                }
            }

            for (n, h) in cfs_v.iter().zip(cfhandles) {
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

    pub fn transaction(
        &self,
        writeopts: &WriteOptions,
        otxnoptions: &OptimistictransactionOptions,
    ) -> Transaction {
        unsafe {
            let inner = ffi::rocksdb_optimistictransaction_begin(
                self.inner,
                writeopts.inner,
                otxnoptions.inner,
                ptr::null_mut(),
            );
            Transaction::new(inner)
        }
    }

    pub fn transaction_default(&self) -> Transaction {
        let write_options = WriteOptions::default();
        let optimistictransaction_options = OptimistictransactionOptions::new();
        self.transaction(&write_options, &optimistictransaction_options)
    }

    pub fn path(&self) -> &Path {
        &self.path.as_path()
    }

    pub fn get_base_db(&self) -> &DB {
        &self.base_db
    }
}

impl Drop for OptimisticTransactionDB {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_optimistictransactiondb_close(self.inner);
        }
    }
}

pub struct OptimistictransactionOptions {
    inner: *mut ffi::rocksdb_optimistictransaction_options_t,
}

impl OptimistictransactionOptions {
    pub fn new() -> OptimistictransactionOptions {
        unsafe {
            let inner = ffi::rocksdb_optimistictransaction_options_create();
            OptimistictransactionOptions { inner }
        }
    }
    pub fn set_snapshot(&self, set_snapshot: bool) {
        unsafe {
            ffi::rocksdb_optimistictransaction_options_set_set_snapshot(
                self.inner,
                set_snapshot as c_uchar,
            );
        }
    }
}

impl Drop for OptimistictransactionOptions {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_optimistictransaction_options_destroy(self.inner);
        }
    }
}
