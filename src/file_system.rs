// Copyright 2020 TiKV Project Authors. Licensed under Apache-2.0.

pub use crocksdb_ffi::{self, DBFileSystemInspectorInstance};

use libc::{c_char, c_void, size_t, strdup};
use std::sync::Arc;

// Inspect global IO flow. No per-file inspection for now.
pub trait FileSystemInspector: Sync + Send {
    fn read(&self, len: usize) -> Result<usize, String>;
    fn write(&self, len: usize) -> Result<usize, String>;
}

extern "C" fn file_system_inspector_destructor(ctx: *mut c_void) {
    unsafe {
        // Recover from raw pointer and implicitly drop.
        Box::from_raw(ctx as *mut Arc<dyn FileSystemInspector>);
    }
}

extern "C" fn file_system_inspector_read(
    ctx: *mut c_void,
    len: size_t,
    errptr: *mut *mut c_char,
) -> size_t {
    let file_system_inspector = unsafe { &*(ctx as *mut Arc<dyn FileSystemInspector>) };
    match file_system_inspector.read(len) {
        Ok(ret) => ret,
        Err(e) => {
            unsafe {
                *errptr = strdup(e.as_ptr() as *const c_char);
            }
            0
        }
    }
}

extern "C" fn file_system_inspector_write(
    ctx: *mut c_void,
    len: size_t,
    errptr: *mut *mut c_char,
) -> size_t {
    let file_system_inspector = unsafe { &*(ctx as *mut Arc<dyn FileSystemInspector>) };
    match file_system_inspector.write(len) {
        Ok(ret) => ret,
        Err(e) => {
            unsafe {
                *errptr = strdup(e.as_ptr() as *const c_char);
            }
            0
        }
    }
}

pub struct DBFileSystemInspector {
    pub inner: *mut DBFileSystemInspectorInstance,
}

unsafe impl Send for DBFileSystemInspector {}
unsafe impl Sync for DBFileSystemInspector {}

impl DBFileSystemInspector {
    pub fn new(file_system_inspector: Arc<dyn FileSystemInspector>) -> DBFileSystemInspector {
        // Size of Arc<dyn T>::into_raw is of 128-bits, which couldn't be used as C-style pointer.
        // Boxing it to make a 64-bits pointer.
        let ctx = Box::into_raw(Box::new(file_system_inspector)) as *mut c_void;
        let instance = unsafe {
            crocksdb_ffi::crocksdb_file_system_inspector_create(
                ctx,
                file_system_inspector_destructor,
                file_system_inspector_read,
                file_system_inspector_write,
            )
        };
        DBFileSystemInspector { inner: instance }
    }
}

impl Drop for DBFileSystemInspector {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_file_system_inspector_destroy(self.inner);
        }
    }
}

#[cfg(test)]
impl FileSystemInspector for DBFileSystemInspector {
    fn read(&self, len: usize) -> Result<usize, String> {
        let ret = unsafe { ffi_try!(crocksdb_file_system_inspector_read(self.inner, len)) };
        Ok(ret)
    }
    fn write(&self, len: usize) -> Result<usize, String> {
        let ret = unsafe { ffi_try!(crocksdb_file_system_inspector_write(self.inner, len)) };
        Ok(ret)
    }
}

#[cfg(test)]
mod test {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use super::*;

    struct TestDrop {
        called: Arc<AtomicUsize>,
    }

    impl Drop for TestDrop {
        fn drop(&mut self) {
            self.called.fetch_add(1, Ordering::SeqCst);
        }
    }

    struct TestFileSystemInspector {
        pub refill_bytes: usize,
        pub read_called: usize,
        pub write_called: usize,
        pub drop: Option<TestDrop>,
    }

    impl Default for TestFileSystemInspector {
        fn default() -> Self {
            TestFileSystemInspector {
                refill_bytes: 0,
                read_called: 0,
                write_called: 0,
                drop: None,
            }
        }
    }

    impl FileSystemInspector for Mutex<TestFileSystemInspector> {
        fn read(&self, len: usize) -> Result<usize, String> {
            let mut inner = self.lock().unwrap();
            inner.read_called += 1;
            if len <= inner.refill_bytes {
                Ok(len)
            } else {
                Err("request exceeds refill bytes".into())
            }
        }
        fn write(&self, len: usize) -> Result<usize, String> {
            let mut inner = self.lock().unwrap();
            inner.write_called += 1;
            if len <= inner.refill_bytes {
                Ok(len)
            } else {
                Err("request exceeds refill bytes".into())
            }
        }
    }

    #[test]
    fn test_create_and_destroy_inspector() {
        let drop_called = Arc::new(AtomicUsize::new(0));
        let fs_inspector = Arc::new(Mutex::new(TestFileSystemInspector {
            drop: Some(TestDrop {
                called: drop_called.clone(),
            }),
            ..Default::default()
        }));
        let db_fs_inspector = DBFileSystemInspector::new(fs_inspector.clone());
        drop(fs_inspector);
        assert_eq!(0, drop_called.load(Ordering::SeqCst));
        drop(db_fs_inspector);
        assert_eq!(1, drop_called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_inspected_operation() {
        let fs_inspector = Arc::new(Mutex::new(TestFileSystemInspector {
            refill_bytes: 4,
            ..Default::default()
        }));
        let db_fs_inspector = DBFileSystemInspector::new(fs_inspector.clone());
        assert_eq!(2, db_fs_inspector.read(2).unwrap());
        assert!(db_fs_inspector.read(8).is_err());
        assert_eq!(2, db_fs_inspector.write(2).unwrap());
        assert!(db_fs_inspector.write(8).is_err());
        let record = fs_inspector.lock().unwrap();
        assert_eq!(2, record.read_called);
        assert_eq!(2, record.write_called);
    }
}
