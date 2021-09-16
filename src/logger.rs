// Copyright 2020 TiKV Project Authors. Licensed under Apache-2.0.

use crocksdb_ffi;
use libc::{c_char, c_void};
use librocksdb_sys::{DBEnv, DBInfoLogLevel as InfoLogLevel, DBLogger};
use std::ffi::{CStr, CString};
use std::str;

pub trait Logger: Send + Sync {
    fn logv(&self, log_level: InfoLogLevel, log: &str);
}

extern "C" fn destructor<L: Logger>(ctx: *mut c_void) {
    unsafe {
        Box::from_raw(ctx as *mut L);
    }
}

extern "C" fn logv<L: Logger>(ctx: *mut c_void, log_level: InfoLogLevel, log: *const c_char) {
    unsafe {
        let logger = &*(ctx as *mut L);
        let log = CStr::from_ptr(log);
        logger.logv(log_level, &log.to_string_lossy());
    }
}

pub fn new_logger<L: Logger>(l: L) -> *mut DBLogger {
    unsafe {
        let p: Box<dyn Logger> = Box::new(l);
        crocksdb_ffi::crocksdb_logger_create(
            Box::into_raw(p) as *mut c_void,
            destructor::<L>,
            logv::<L>,
        )
    }
}

pub fn create_env_logger(fname: &str, mut env: DBEnv) -> *mut DBLogger {
    let name = CString::new(fname.as_bytes()).unwrap();
    unsafe { crocksdb_ffi::crocksdb_create_env_logger(name.as_ptr(), &mut env) }
}
