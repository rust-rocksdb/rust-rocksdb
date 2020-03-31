// Copyright 2020 TiKV Project Authors. Licensed under Apache-2.0.

use crocksdb_ffi;
use libc::{c_char, c_void};
use librocksdb_sys::{DBEnv, DBInfoLogLevel as InfoLogLevel, DBLogger};
use std::ffi::{CString, VaList};

pub trait Logger: Send + Sync {
    fn logv(&self, log_level: InfoLogLevel, format: *const c_char, ap: VaList);
}

extern "C" fn destructor(ctx: *mut c_void) {
    unsafe {
        Box::from_raw(ctx as *mut Box<dyn Logger>);
    }
}

extern "C" fn logv(ctx: *mut c_void, log_level: InfoLogLevel, format: *const c_char, ap: VaList) {
    unsafe {
        let logger = &*(ctx as *mut Box<dyn Logger>);
        logger.logv(log_level, format, ap);
    }
}

pub fn new_logger<L: Logger>(l: L) -> *mut DBLogger {
    unsafe {
        let p: Box<dyn Logger> = Box::new(l);
        crocksdb_ffi::crocksdb_logger_create(
            Box::into_raw(Box::new(p)) as *mut c_void,
            destructor,
            logv,
        )
    }
}

pub fn create_env_logger(fname: &str, mut env: DBEnv) -> *mut DBLogger {
    let name = CString::new(fname.as_bytes()).unwrap();
    unsafe { crocksdb_ffi::crocksdb_create_env_logger(name.as_ptr(), &mut env) }
}
