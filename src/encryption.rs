// Copyright 2020 TiKV Project Authors. Licensed under Apache-2.0.

pub use crocksdb_ffi::{
    self, DBEncryptionKeyManagerInstance, DBEncryptionMethod, DBFileEncryptionInfo,
};

use libc::{c_char, c_void, size_t};
use std::ffi::{CStr, CString};
use std::fmt::{self, Debug, Formatter};
use std::io::Result;
use std::ptr;
use std::sync::Arc;

#[derive(Clone, PartialEq, Eq)]
pub struct FileEncryptionInfo {
    pub method: DBEncryptionMethod,
    pub key: Vec<u8>,
    pub iv: Vec<u8>,
}

impl Default for FileEncryptionInfo {
    fn default() -> Self {
        FileEncryptionInfo {
            method: DBEncryptionMethod::Unknown,
            key: vec![],
            iv: vec![],
        }
    }
}

impl Debug for FileEncryptionInfo {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "FileEncryptionInfo [method={}, key=...<{} bytes>, iv=...<{} bytes>]",
            self.method,
            self.key.len(),
            self.iv.len()
        )
    }
}

impl FileEncryptionInfo {
    pub unsafe fn copy_to(&self, file_info: *mut DBFileEncryptionInfo) {
        crocksdb_ffi::crocksdb_file_encryption_info_set_method(file_info, self.method);
        crocksdb_ffi::crocksdb_file_encryption_info_set_key(
            file_info,
            self.key.as_ptr() as *const c_char,
            self.key.len() as size_t,
        );
        crocksdb_ffi::crocksdb_file_encryption_info_set_iv(
            file_info,
            self.iv.as_ptr() as *const c_char,
            self.iv.len() as size_t,
        );
    }
}

pub trait EncryptionKeyManager: Sync + Send {
    fn get_file(&self, fname: &str) -> Result<FileEncryptionInfo>;
    fn new_file(&self, fname: &str) -> Result<FileEncryptionInfo>;
    fn delete_file(&self, fname: &str) -> Result<()>;
    fn link_file(&self, src_fname: &str, dst_fname: &str) -> Result<()>;
    fn rename_file(&self, src_fname: &str, dst_fname: &str) -> Result<()>;
}

// Copy rust-owned error message to C-owned string. Caller is responsible to delete the result.
fn copy_error<T: Into<Vec<u8>>>(err: T) -> *const c_char {
    let cstr = CString::new(err).unwrap();
    unsafe { libc::strdup(cstr.as_ptr()) }
}

extern "C" fn encryption_key_manager_destructor(ctx: *mut c_void) {
    unsafe {
        // Recover from raw pointer and implicitly drop.
        Box::from_raw(ctx as *mut Arc<dyn EncryptionKeyManager>);
    }
}

extern "C" fn encryption_key_manager_get_file(
    ctx: *mut c_void,
    fname: *const c_char,
    file_info: *mut DBFileEncryptionInfo,
) -> *const c_char {
    let key_manager = unsafe { &*(ctx as *mut Arc<dyn EncryptionKeyManager>) };
    let fname = match unsafe { CStr::from_ptr(fname).to_str() } {
        Ok(ret) => ret,
        Err(err) => {
            return copy_error(format!(
                "Encryption key manager encounter non-utf8 file name: {}",
                err
            ));
        }
    };
    match key_manager.get_file(fname) {
        Ok(ret) => {
            unsafe {
                ret.copy_to(file_info);
            }
            ptr::null()
        }
        Err(err) => copy_error(format!("Encryption key manager get file failure: {}", err)),
    }
}

extern "C" fn encryption_key_manager_new_file(
    ctx: *mut c_void,
    fname: *const c_char,
    file_info: *mut DBFileEncryptionInfo,
) -> *const c_char {
    let key_manager = unsafe { &*(ctx as *mut Arc<dyn EncryptionKeyManager>) };
    let fname = match unsafe { CStr::from_ptr(fname).to_str() } {
        Ok(ret) => ret,
        Err(err) => {
            return copy_error(format!(
                "Encryption key manager encounter non-utf8 file name: {}",
                err
            ));
        }
    };
    match key_manager.new_file(fname) {
        Ok(ret) => {
            unsafe {
                ret.copy_to(file_info);
            }
            ptr::null()
        }
        Err(err) => copy_error(format!("Encryption key manager new file failure: {}", err)),
    }
}

extern "C" fn encryption_key_manager_delete_file(
    ctx: *mut c_void,
    fname: *const c_char,
) -> *const c_char {
    let key_manager = unsafe { &*(ctx as *mut Arc<dyn EncryptionKeyManager>) };
    let fname = match unsafe { CStr::from_ptr(fname).to_str() } {
        Ok(ret) => ret,
        Err(err) => {
            return copy_error(format!(
                "Encryption key manager encounter non-utf8 file name: {}",
                err
            ));
        }
    };
    match key_manager.delete_file(fname) {
        Ok(()) => ptr::null(),
        Err(err) => copy_error(format!(
            "Encryption key manager delete file failure: {}",
            err
        )),
    }
}

extern "C" fn encryption_key_manager_link_file(
    ctx: *mut c_void,
    src_fname: *const c_char,
    dst_fname: *const c_char,
) -> *const c_char {
    let key_manager = unsafe { &*(ctx as *mut Arc<dyn EncryptionKeyManager>) };
    let src_fname = match unsafe { CStr::from_ptr(src_fname).to_str() } {
        Ok(ret) => ret,
        Err(err) => {
            return copy_error(format!(
                "Encryption key manager encounter non-utf8 file name: {}",
                err
            ));
        }
    };
    let dst_fname = match unsafe { CStr::from_ptr(dst_fname).to_str() } {
        Ok(ret) => ret,
        Err(err) => {
            return copy_error(format!(
                "Encryption key manager encounter non-utf8 file name: {}",
                err
            ));
        }
    };
    match key_manager.link_file(src_fname, dst_fname) {
        Ok(()) => ptr::null(),
        Err(err) => copy_error(format!(
            "Encryption key manager delete file failure: {}",
            err
        )),
    }
}

extern "C" fn encryption_key_manager_rename_file(
    ctx: *mut c_void,
    src_fname: *const c_char,
    dst_fname: *const c_char,
) -> *const c_char {
    let key_manager = unsafe { &*(ctx as *mut Arc<dyn EncryptionKeyManager>) };
    let src_fname = match unsafe { CStr::from_ptr(src_fname).to_str() } {
        Ok(ret) => ret,
        Err(err) => {
            return copy_error(format!(
                "Encryption key manager encounter non-utf8 file name: {}",
                err
            ));
        }
    };
    let dst_fname = match unsafe { CStr::from_ptr(dst_fname).to_str() } {
        Ok(ret) => ret,
        Err(err) => {
            return copy_error(format!(
                "Encryption key manager encounter non-utf8 file name: {}",
                err
            ));
        }
    };
    match key_manager.rename_file(src_fname, dst_fname) {
        Ok(()) => ptr::null(),
        Err(err) => copy_error(format!(
            "Encryption key manager delete file failure: {}",
            err
        )),
    }
}

pub struct DBEncryptionKeyManager {
    pub inner: *mut DBEncryptionKeyManagerInstance,
}

unsafe impl Send for DBEncryptionKeyManager {}
unsafe impl Sync for DBEncryptionKeyManager {}

impl DBEncryptionKeyManager {
    pub fn new(key_manager: Arc<dyn EncryptionKeyManager>) -> DBEncryptionKeyManager {
        // Size of Arc<dyn T>::into_raw is of 128-bits, which couldn't be used as C-style pointer.
        // Bixing it to make a 64-bits pointer.
        let ctx = Box::into_raw(Box::new(key_manager)) as *mut c_void;
        let instance = unsafe {
            crocksdb_ffi::crocksdb_encryption_key_manager_create(
                ctx,
                encryption_key_manager_destructor,
                encryption_key_manager_get_file,
                encryption_key_manager_new_file,
                encryption_key_manager_delete_file,
                encryption_key_manager_link_file,
                encryption_key_manager_rename_file,
            )
        };
        DBEncryptionKeyManager { inner: instance }
    }
}

impl Drop for DBEncryptionKeyManager {
    fn drop(&mut self) {
        unsafe {
            crocksdb_ffi::crocksdb_encryption_key_manager_destroy(self.inner);
        }
    }
}

// The implementation of EncryptionKeyManager is used to test calling the methods through FFI.
#[cfg(test)]
impl EncryptionKeyManager for DBEncryptionKeyManager {
    fn get_file(&self, fname: &str) -> Result<FileEncryptionInfo> {
        use std::io::{Error, ErrorKind};
        use std::mem;
        use std::slice;
        let ret: Result<FileEncryptionInfo>;
        unsafe {
            let file_info = crocksdb_ffi::crocksdb_file_encryption_info_create();
            let err = crocksdb_ffi::crocksdb_encryption_key_manager_get_file(
                self.inner,
                CString::new(fname).unwrap().as_ptr(),
                file_info,
            );
            if err == ptr::null() {
                let mut key_len: size_t = 0;
                let mut iv_len: size_t = 0;
                let key: *const u8 = mem::transmute(
                    crocksdb_ffi::crocksdb_file_encryption_info_key(file_info, &mut key_len),
                );
                let iv: *const u8 = mem::transmute(crocksdb_ffi::crocksdb_file_encryption_info_iv(
                    file_info,
                    &mut iv_len,
                ));
                ret = Ok(FileEncryptionInfo {
                    method: crocksdb_ffi::crocksdb_file_encryption_info_method(file_info),
                    key: slice::from_raw_parts(key, key_len).to_vec(),
                    iv: slice::from_raw_parts(iv, iv_len).to_vec(),
                });
            } else {
                ret = Err(Error::new(
                    ErrorKind::Other,
                    format!("{}", CStr::from_ptr(err).to_str().unwrap()),
                ));
                libc::free(err as _);
            }
            crocksdb_ffi::crocksdb_file_encryption_info_destroy(file_info);
        }
        ret
    }

    fn new_file(&self, fname: &str) -> Result<FileEncryptionInfo> {
        use std::io::{Error, ErrorKind};
        use std::mem;
        use std::slice;
        let ret: Result<FileEncryptionInfo>;
        unsafe {
            let file_info = crocksdb_ffi::crocksdb_file_encryption_info_create();
            let err = crocksdb_ffi::crocksdb_encryption_key_manager_new_file(
                self.inner,
                CString::new(fname).unwrap().as_ptr(),
                file_info,
            );
            if err == ptr::null() {
                let mut key_len: size_t = 0;
                let mut iv_len: size_t = 0;
                let key: *const u8 = mem::transmute(
                    crocksdb_ffi::crocksdb_file_encryption_info_key(file_info, &mut key_len),
                );
                let iv: *const u8 = mem::transmute(crocksdb_ffi::crocksdb_file_encryption_info_iv(
                    file_info,
                    &mut iv_len,
                ));
                ret = Ok(FileEncryptionInfo {
                    method: crocksdb_ffi::crocksdb_file_encryption_info_method(file_info),
                    key: slice::from_raw_parts(key, key_len).to_vec(),
                    iv: slice::from_raw_parts(iv, iv_len).to_vec(),
                });
            } else {
                ret = Err(Error::new(
                    ErrorKind::Other,
                    format!("{}", CStr::from_ptr(err).to_str().unwrap()),
                ));
                libc::free(err as _);
            }
            crocksdb_ffi::crocksdb_file_encryption_info_destroy(file_info);
        }
        ret
    }

    fn delete_file(&self, fname: &str) -> Result<()> {
        use std::io::{Error, ErrorKind};
        let ret: Result<()>;
        unsafe {
            let err = crocksdb_ffi::crocksdb_encryption_key_manager_delete_file(
                self.inner,
                CString::new(fname).unwrap().as_ptr(),
            );
            if err == ptr::null() {
                ret = Ok(());
            } else {
                ret = Err(Error::new(
                    ErrorKind::Other,
                    format!("{}", CStr::from_ptr(err).to_str().unwrap()),
                ));
                libc::free(err as _);
            }
        }
        ret
    }

    fn link_file(&self, src_fname: &str, dst_fname: &str) -> Result<()> {
        use std::io::{Error, ErrorKind};
        let ret: Result<()>;
        unsafe {
            let err = crocksdb_ffi::crocksdb_encryption_key_manager_link_file(
                self.inner,
                CString::new(src_fname).unwrap().as_ptr(),
                CString::new(dst_fname).unwrap().as_ptr(),
            );
            if err == ptr::null() {
                ret = Ok(());
            } else {
                ret = Err(Error::new(
                    ErrorKind::Other,
                    format!("{}", CStr::from_ptr(err).to_str().unwrap()),
                ));
                libc::free(err as _);
            }
        }
        ret
    }

    fn rename_file(&self, src_fname: &str, dst_fname: &str) -> Result<()> {
        use std::io::{Error, ErrorKind};
        let ret: Result<()>;
        unsafe {
            let err = crocksdb_ffi::crocksdb_encryption_key_manager_rename_file(
                self.inner,
                CString::new(src_fname).unwrap().as_ptr(),
                CString::new(dst_fname).unwrap().as_ptr(),
            );
            if err == ptr::null() {
                ret = Ok(());
            } else {
                ret = Err(Error::new(
                    ErrorKind::Other,
                    format!("{}", CStr::from_ptr(err).to_str().unwrap()),
                ));
                libc::free(err as _);
            }
        }
        ret
    }
}

#[cfg(test)]
mod test {
    use std::io::{Error, ErrorKind};
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

    struct TestEncryptionKeyManager {
        pub get_file_called: AtomicUsize,
        pub new_file_called: AtomicUsize,
        pub delete_file_called: AtomicUsize,
        pub link_file_called: AtomicUsize,
        pub rename_file_called: AtomicUsize,
        pub fname: Mutex<String>,
        pub dst_fname: Mutex<String>,
        pub return_value: Option<FileEncryptionInfo>,
        pub drop: Option<TestDrop>,
    }

    impl Default for TestEncryptionKeyManager {
        fn default() -> Self {
            TestEncryptionKeyManager {
                get_file_called: AtomicUsize::new(0),
                new_file_called: AtomicUsize::new(0),
                delete_file_called: AtomicUsize::new(0),
                link_file_called: AtomicUsize::new(0),
                rename_file_called: AtomicUsize::new(0),
                fname: Mutex::new("".to_string()),
                dst_fname: Mutex::new("".to_string()),
                return_value: None,
                drop: None,
            }
        }
    }

    impl EncryptionKeyManager for Mutex<TestEncryptionKeyManager> {
        fn get_file(&self, fname: &str) -> Result<FileEncryptionInfo> {
            let key_manager = self.lock().unwrap();
            key_manager.get_file_called.fetch_add(1, Ordering::SeqCst);
            key_manager.fname.lock().unwrap().insert_str(0, fname);
            match &key_manager.return_value {
                Some(file_info) => Ok(file_info.clone()),
                None => Err(Error::new(ErrorKind::Other, "")),
            }
        }

        fn new_file(&self, fname: &str) -> Result<FileEncryptionInfo> {
            let key_manager = self.lock().unwrap();
            key_manager.new_file_called.fetch_add(1, Ordering::SeqCst);
            key_manager.fname.lock().unwrap().insert_str(0, fname);
            match &key_manager.return_value {
                Some(file_info) => Ok(file_info.clone()),
                None => Err(Error::new(ErrorKind::Other, "")),
            }
        }

        fn delete_file(&self, fname: &str) -> Result<()> {
            let key_manager = self.lock().unwrap();
            key_manager
                .delete_file_called
                .fetch_add(1, Ordering::SeqCst);
            key_manager.fname.lock().unwrap().insert_str(0, fname);
            match &key_manager.return_value {
                Some(_) => Ok(()),
                None => Err(Error::new(ErrorKind::Other, "")),
            }
        }

        fn link_file(&self, src_fname: &str, dst_fname: &str) -> Result<()> {
            let key_manager = self.lock().unwrap();
            key_manager.link_file_called.fetch_add(1, Ordering::SeqCst);
            key_manager.fname.lock().unwrap().insert_str(0, src_fname);
            key_manager
                .dst_fname
                .lock()
                .unwrap()
                .insert_str(0, dst_fname);
            match &key_manager.return_value {
                Some(_) => Ok(()),
                None => Err(Error::new(ErrorKind::Other, "")),
            }
        }

        fn rename_file(&self, src_fname: &str, dst_fname: &str) -> Result<()> {
            let key_manager = self.lock().unwrap();
            key_manager
                .rename_file_called
                .fetch_add(1, Ordering::SeqCst);
            key_manager.fname.lock().unwrap().insert_str(0, src_fname);
            key_manager
                .dst_fname
                .lock()
                .unwrap()
                .insert_str(0, dst_fname);
            match &key_manager.return_value {
                Some(_) => Ok(()),
                None => Err(Error::new(ErrorKind::Other, "")),
            }
        }
    }

    #[test]
    fn create_and_destroy() {
        let drop_called = Arc::new(AtomicUsize::new(0));
        let key_manager = Arc::new(Mutex::new(TestEncryptionKeyManager {
            drop: Some(TestDrop {
                called: drop_called.clone(),
            }),
            ..Default::default()
        }));
        let db_key_manager = DBEncryptionKeyManager::new(key_manager.clone());
        drop(key_manager);
        assert_eq!(0, drop_called.load(Ordering::SeqCst));
        drop(db_key_manager);
        assert_eq!(1, drop_called.load(Ordering::SeqCst));
    }

    #[test]
    fn get_file() {
        let key_manager = Arc::new(Mutex::new(TestEncryptionKeyManager {
            return_value: Some(FileEncryptionInfo {
                method: DBEncryptionMethod::Aes128Ctr,
                key: b"test_key_get_file".to_vec(),
                iv: b"test_iv_get_file".to_vec(),
            }),
            ..Default::default()
        }));
        let db_key_manager = DBEncryptionKeyManager::new(key_manager.clone());
        let file_info = db_key_manager.get_file("get_file_path").unwrap();
        assert_eq!(DBEncryptionMethod::Aes128Ctr, file_info.method);
        assert_eq!(b"test_key_get_file", file_info.key.as_slice());
        assert_eq!(b"test_iv_get_file", file_info.iv.as_slice());
        let record = key_manager.lock().unwrap();
        assert_eq!(1, record.get_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.new_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.delete_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.link_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.rename_file_called.load(Ordering::SeqCst));
        assert_eq!("get_file_path", record.fname.lock().unwrap().as_str());
    }

    #[test]
    fn get_file_error() {
        let key_manager = Arc::new(Mutex::new(TestEncryptionKeyManager::default()));
        let db_key_manager = DBEncryptionKeyManager::new(key_manager.clone());
        assert!(db_key_manager.get_file("get_file_path").is_err());
        let record = key_manager.lock().unwrap();
        assert_eq!(1, record.get_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.new_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.delete_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.link_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.rename_file_called.load(Ordering::SeqCst));
        assert_eq!("get_file_path", record.fname.lock().unwrap().as_str());
    }

    #[test]
    fn new_file() {
        let key_manager = Arc::new(Mutex::new(TestEncryptionKeyManager {
            return_value: Some(FileEncryptionInfo {
                method: DBEncryptionMethod::Aes256Ctr,
                key: b"test_key_new_file".to_vec(),
                iv: b"test_iv_new_file".to_vec(),
            }),
            ..Default::default()
        }));
        let db_key_manager = DBEncryptionKeyManager::new(key_manager.clone());
        let file_info = db_key_manager.new_file("new_file_path").unwrap();
        assert_eq!(DBEncryptionMethod::Aes256Ctr, file_info.method);
        assert_eq!(b"test_key_new_file", file_info.key.as_slice());
        assert_eq!(b"test_iv_new_file", file_info.iv.as_slice());
        let record = key_manager.lock().unwrap();
        assert_eq!(0, record.get_file_called.load(Ordering::SeqCst));
        assert_eq!(1, record.new_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.delete_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.link_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.rename_file_called.load(Ordering::SeqCst));
        assert_eq!("new_file_path", record.fname.lock().unwrap().as_str());
    }

    #[test]
    fn new_file_error() {
        let key_manager = Arc::new(Mutex::new(TestEncryptionKeyManager::default()));
        let db_key_manager = DBEncryptionKeyManager::new(key_manager.clone());
        assert!(db_key_manager.new_file("new_file_path").is_err());
        let record = key_manager.lock().unwrap();
        assert_eq!(0, record.get_file_called.load(Ordering::SeqCst));
        assert_eq!(1, record.new_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.delete_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.link_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.rename_file_called.load(Ordering::SeqCst));
        assert_eq!("new_file_path", record.fname.lock().unwrap().as_str());
    }

    #[test]
    fn delete_file() {
        let key_manager = Arc::new(Mutex::new(TestEncryptionKeyManager {
            return_value: Some(FileEncryptionInfo::default()),
            ..Default::default()
        }));
        let db_key_manager = DBEncryptionKeyManager::new(key_manager.clone());
        assert!(db_key_manager.delete_file("delete_file_path").is_ok());
        let record = key_manager.lock().unwrap();
        assert_eq!(0, record.get_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.new_file_called.load(Ordering::SeqCst));
        assert_eq!(1, record.delete_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.link_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.rename_file_called.load(Ordering::SeqCst));
        assert_eq!("delete_file_path", record.fname.lock().unwrap().as_str());
    }

    #[test]
    fn delete_file_error() {
        let key_manager = Arc::new(Mutex::new(TestEncryptionKeyManager::default()));
        let db_key_manager = DBEncryptionKeyManager::new(key_manager.clone());
        assert!(db_key_manager.delete_file("delete_file_path").is_err());
        let record = key_manager.lock().unwrap();
        assert_eq!(0, record.get_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.new_file_called.load(Ordering::SeqCst));
        assert_eq!(1, record.delete_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.link_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.rename_file_called.load(Ordering::SeqCst));
        assert_eq!("delete_file_path", record.fname.lock().unwrap().as_str());
    }

    #[test]
    fn link_file() {
        let key_manager = Arc::new(Mutex::new(TestEncryptionKeyManager {
            return_value: Some(FileEncryptionInfo::default()),
            ..Default::default()
        }));
        let db_key_manager = DBEncryptionKeyManager::new(key_manager.clone());
        assert!(db_key_manager
            .link_file("src_link_file_path", "dst_link_file_path")
            .is_ok());
        let record = key_manager.lock().unwrap();
        assert_eq!(0, record.get_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.new_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.delete_file_called.load(Ordering::SeqCst));
        assert_eq!(1, record.link_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.rename_file_called.load(Ordering::SeqCst));
        assert_eq!("src_link_file_path", record.fname.lock().unwrap().as_str());
        assert_eq!(
            "dst_link_file_path",
            record.dst_fname.lock().unwrap().as_str()
        );
    }

    #[test]
    fn link_file_error() {
        let key_manager = Arc::new(Mutex::new(TestEncryptionKeyManager::default()));
        let db_key_manager = DBEncryptionKeyManager::new(key_manager.clone());
        assert!(db_key_manager
            .link_file("src_link_file_path", "dst_link_file_path")
            .is_err());
        let record = key_manager.lock().unwrap();
        assert_eq!(0, record.get_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.new_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.delete_file_called.load(Ordering::SeqCst));
        assert_eq!(1, record.link_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.rename_file_called.load(Ordering::SeqCst));
        assert_eq!("src_link_file_path", record.fname.lock().unwrap().as_str());
        assert_eq!(
            "dst_link_file_path",
            record.dst_fname.lock().unwrap().as_str()
        );
    }

    #[test]
    fn rename_file() {
        let key_manager = Arc::new(Mutex::new(TestEncryptionKeyManager {
            return_value: Some(FileEncryptionInfo::default()),
            ..Default::default()
        }));
        let db_key_manager = DBEncryptionKeyManager::new(key_manager.clone());
        assert!(db_key_manager
            .rename_file("src_rename_file_path", "dst_rename_file_path")
            .is_ok());
        let record = key_manager.lock().unwrap();
        assert_eq!(0, record.get_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.new_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.delete_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.link_file_called.load(Ordering::SeqCst));
        assert_eq!(1, record.rename_file_called.load(Ordering::SeqCst));
        assert_eq!(
            "src_rename_file_path",
            record.fname.lock().unwrap().as_str()
        );
        assert_eq!(
            "dst_rename_file_path",
            record.dst_fname.lock().unwrap().as_str()
        );
    }

    #[test]
    fn rename_file_error() {
        let key_manager = Arc::new(Mutex::new(TestEncryptionKeyManager::default()));
        let db_key_manager = DBEncryptionKeyManager::new(key_manager.clone());
        assert!(db_key_manager
            .rename_file("src_rename_file_path", "dst_rename_file_path")
            .is_err());
        let record = key_manager.lock().unwrap();
        assert_eq!(0, record.get_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.new_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.delete_file_called.load(Ordering::SeqCst));
        assert_eq!(0, record.link_file_called.load(Ordering::SeqCst));
        assert_eq!(1, record.rename_file_called.load(Ordering::SeqCst));
        assert_eq!(
            "src_rename_file_path",
            record.fname.lock().unwrap().as_str()
        );
        assert_eq!(
            "dst_rename_file_path",
            record.dst_fname.lock().unwrap().as_str()
        );
    }
}
