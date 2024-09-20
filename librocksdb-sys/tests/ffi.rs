// Copyright 2016 Alex Regueiro
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

// This code is based on <https://github.com/facebook/rocksdb/blob/master/db/c_test.c>, revision a10e8a056d569acf6a52045124e6414ad33bdfcd.

#![allow(
    non_snake_case,
    non_upper_case_globals,
    unused_mut,
    unused_unsafe,
    unused_variables
)]

use libc::*;
use librocksdb_sys::*;
use std::borrow::Cow;
use std::env;
use std::ffi::{CStr, CString};
use std::io::Write;
use std::path::PathBuf;
use std::ptr;
use std::slice;
use std::str;
use uuid::Uuid;

macro_rules! err_println {
    ($($arg:tt)*) => (writeln!(&mut ::std::io::stderr(), $($arg)*).expect("failed printing to stderr"));
}

macro_rules! cstrp {
    ($s:expr) => {{
        static CSTR: &CStr =
            unsafe { CStr::from_bytes_with_nul_unchecked(concat!($s, "\0").as_bytes()) };
        CSTR.as_ptr()
    }};
}

static mut phase: &'static str = "";
// static mut dbname: *mut c_uchar = ptr::null_mut();
// static mut dbbackupname: *mut c_uchar = ptr::null_mut();

unsafe fn strndup(s: *const c_char, n: size_t) -> *mut c_char {
    let r: *mut c_char = malloc(n + 1) as *mut c_char;
    if r.is_null() {
        return r;
    }
    strncpy(r, s, n)
}

unsafe fn rstr<'a>(s: *const c_char) -> Cow<'a, str> {
    CStr::from_ptr(s).to_string_lossy()
}

fn GetTempDir() -> PathBuf {
    return match option_env!("TEST_TMPDIR") {
        Some("") | None => env::temp_dir(),
        Some(s) => s.into(),
    };
}

unsafe fn StartPhase(name: &'static str) {
    err_println!("=== Test {}\n", name);
    phase = name;
}

macro_rules! CheckNoError {
    ($err:ident) => {
        unsafe {
            assert!($err.is_null(), "{}: {}", phase, rstr($err));
        }
    };
}

macro_rules! CheckCondition {
    ($cond:expr) => {
        unsafe {
            assert!($cond, "{}: {}", phase, stringify!($cond));
        }
    };
}

unsafe fn CheckEqual(expected: *const c_char, actual: *const c_char, n: size_t) {
    let is_equal = if expected.is_null() && actual.is_null() {
        true
    } else if !expected.is_null()
        && !actual.is_null()
        && n == strlen(expected)
        && memcmp(expected as *const c_void, actual as *const c_void, n) == 0
    {
        true
    } else {
        false
    };

    if !is_equal {
        panic!(
            "{}: expected '{}', got '{}'",
            phase,
            rstr(strndup(expected, n)),
            rstr(strndup(actual, 5))
        );
    }
}

unsafe fn Free<T>(ptr: *mut *mut T) {
    if !(*ptr).is_null() {
        free(*ptr as *mut c_void);
        *ptr = ptr::null_mut();
    }
}

unsafe fn CheckGet(
    mut db: *mut rocksdb_t,
    options: *mut rocksdb_readoptions_t,
    key: *const c_char,
    expected: *const c_char,
) {
    let mut err: *mut c_char = ptr::null_mut();
    let mut val_len: size_t = 0;
    let mut val: *mut c_char = rocksdb_get(db, options, key, strlen(key), &mut val_len, &mut err);
    CheckNoError!(err);
    CheckEqual(expected, val, val_len);
    Free(&mut val);
}

unsafe fn CheckGetCF(
    db: *mut rocksdb_t,
    options: *const rocksdb_readoptions_t,
    handle: *mut rocksdb_column_family_handle_t,
    key: *const c_char,
    expected: *const c_char,
) {
    let mut err: *mut c_char = ptr::null_mut();
    let mut val_len: size_t = 0;
    let mut val: *mut c_char = rocksdb_get_cf(
        db,
        options,
        handle,
        key,
        strlen(key),
        &mut val_len,
        &mut err,
    );
    CheckNoError!(err);
    CheckEqual(expected, val, val_len);
    Free(&mut val);
}

unsafe fn CheckIter(iter: *mut rocksdb_iterator_t, key: *const c_char, val: *const c_char) {
    let mut len: size_t = 0;
    let mut str: *const c_char;
    str = rocksdb_iter_key(iter, &mut len);
    CheckEqual(key, str, len);
    str = rocksdb_iter_value(iter, &mut len);
    CheckEqual(val, str, len);
}

// Callback from rocksdb_writebatch_iterate()
unsafe extern "C" fn CheckPut(
    ptr: *mut c_void,
    k: *const c_char,
    klen: size_t,
    v: *const c_char,
    vlen: size_t,
) {
    let mut state: *mut c_int = ptr as *mut c_int;
    CheckCondition!(*state < 2);
    match *state {
        0 => {
            CheckEqual(cstrp!("bar"), k, klen);
            CheckEqual(cstrp!("b"), v, vlen);
        }
        1 => {
            CheckEqual(cstrp!("box"), k, klen);
            CheckEqual(cstrp!("c"), v, vlen);
        }
        _ => {}
    }
    *state += 1;
}

// Callback from rocksdb_writebatch_iterate()
unsafe extern "C" fn CheckDel(ptr: *mut c_void, k: *const c_char, klen: size_t) {
    let mut state: *mut c_int = ptr as *mut c_int;
    CheckCondition!(*state == 2);
    CheckEqual(cstrp!("bar"), k, klen);
    *state += 1;
}

unsafe extern "C" fn CmpDestroy(arg: *mut c_void) {}

unsafe extern "C" fn CmpCompare(
    arg: *mut c_void,
    a: *const c_char,
    alen: size_t,
    b: *const c_char,
    blen: size_t,
) -> c_int {
    let n = if alen < blen { alen } else { blen };
    let mut r = memcmp(a as *const c_void, b as *const c_void, n);
    if r == 0 {
        if alen < blen {
            r = -1;
        } else if alen > blen {
            r = 1;
        }
    }
    r
}

unsafe extern "C" fn CmpName(arg: *mut c_void) -> *const c_char {
    cstrp!("foo")
}

// Custom compaction filter

static mut fake_filter_result: c_uchar = 1;

unsafe extern "C" fn CFilterDestroy(arg: *mut c_void) {}

unsafe extern "C" fn CFilterName(arg: *mut c_void) -> *const c_char {
    cstrp!("foo")
}

unsafe extern "C" fn CFilterFilter(
    arg: *mut c_void,
    level: c_int,
    key: *const c_char,
    key_length: size_t,
    existing_value: *const c_char,
    value_length: size_t,
    new_value: *mut *mut c_char,
    new_value_length: *mut size_t,
    value_changed: *mut u8,
) -> c_uchar {
    if key_length == 3 {
        if memcmp(
            key.cast::<c_void>(),
            cstrp!("bar").cast::<c_void>(),
            key_length,
        ) == 0
        {
            return 1;
        } else if memcmp(
            key.cast::<c_void>(),
            cstrp!("baz").cast::<c_void>(),
            key_length,
        ) == 0
        {
            *value_changed = 1;
            *new_value = cstrp!("newbazvalue") as *mut c_char;
            *new_value_length = 11;
            return 0;
        }
    }
    0
}

unsafe extern "C" fn CFilterFactoryDestroy(arg: *mut c_void) {}

unsafe extern "C" fn CFilterFactoryName(arg: *mut c_void) -> *const c_char {
    cstrp!("foo")
}

unsafe extern "C" fn CFilterCreate(
    arg: *mut c_void,
    context: *mut rocksdb_compactionfiltercontext_t,
) -> *mut rocksdb_compactionfilter_t {
    rocksdb_compactionfilter_create(
        ptr::null_mut(),
        Some(CFilterDestroy),
        Some(CFilterFilter),
        Some(CFilterName),
    )
}

unsafe fn CheckCompaction(
    dbname: *const c_char,
    db: *mut rocksdb_t,
    options: *const rocksdb_options_t,
    roptions: *mut rocksdb_readoptions_t,
    woptions: *mut rocksdb_writeoptions_t,
) -> *mut rocksdb_t {
    let mut err: *mut c_char = ptr::null_mut();
    let db = rocksdb_open(options, dbname, &mut err);
    CheckNoError!(err);
    rocksdb_put(
        db,
        woptions,
        cstrp!("foo"),
        3,
        cstrp!("foovalue"),
        8,
        &mut err,
    );
    CheckNoError!(err);
    CheckGet(db, roptions, cstrp!("foo"), cstrp!("foovalue"));
    rocksdb_put(
        db,
        woptions,
        cstrp!("bar"),
        3,
        cstrp!("barvalue"),
        8,
        &mut err,
    );
    CheckNoError!(err);
    CheckGet(db, roptions, cstrp!("bar"), cstrp!("barvalue"));
    rocksdb_put(
        db,
        woptions,
        cstrp!("baz"),
        3,
        cstrp!("bazvalue"),
        8,
        &mut err,
    );
    CheckNoError!(err);
    CheckGet(db, roptions, cstrp!("baz"), cstrp!("bazvalue"));

    // Force compaction
    rocksdb_compact_range(db, ptr::null(), 0, ptr::null(), 0);
    // should have filtered bar, but not foo
    CheckGet(db, roptions, cstrp!("foo"), cstrp!("foovalue"));
    CheckGet(db, roptions, cstrp!("bar"), ptr::null());
    CheckGet(db, roptions, cstrp!("baz"), cstrp!("newbazvalue"));
    db
}

// Custom merge operator

unsafe extern "C" fn MergeOperatorDestroy(arg: *mut c_void) {}

unsafe extern "C" fn MergeOperatorName(arg: *mut c_void) -> *const c_char {
    cstrp!("foo")
}

unsafe extern "C" fn MergeOperatorFullMerge(
    arg: *mut c_void,
    key: *const c_char,
    key_length: size_t,
    existing_value: *const c_char,
    existing_value_length: size_t,
    operands_list: *const *const c_char,
    operands_list_length: *const size_t,
    num_operands: c_int,
    success: *mut u8,
    new_value_length: *mut size_t,
) -> *mut c_char {
    *new_value_length = 4;
    *success = 1;
    let result: *mut c_char = malloc(4) as *mut _;
    memcpy(result as *mut _, cstrp!("fake") as *mut _, 4);
    result
}

unsafe extern "C" fn MergeOperatorPartialMerge(
    arg: *mut c_void,
    key: *const c_char,
    key_length: size_t,
    operands_list: *const *const c_char,
    operands_list_length: *const size_t,
    num_operands: c_int,
    success: *mut u8,
    new_value_length: *mut size_t,
) -> *mut c_char {
    *new_value_length = 4;
    *success = 1;
    let result: *mut c_char = malloc(4) as *mut _;
    memcpy(result as *mut _, cstrp!("fake") as *const _, 4);
    result
}

#[test]
fn ffi() {
    unsafe {
        let mut db: *mut rocksdb_t;
        let mut cmp: *mut rocksdb_comparator_t;
        let mut cache: *mut rocksdb_cache_t;
        let mut env: *mut rocksdb_env_t;
        let mut options: *mut rocksdb_options_t;
        let mut table_options: *mut rocksdb_block_based_table_options_t;
        let mut roptions: *mut rocksdb_readoptions_t;
        let mut woptions: *mut rocksdb_writeoptions_t;
        let mut err: *mut c_char = ptr::null_mut();
        let run: c_int = -1;

        let test_uuid = Uuid::new_v4().simple();

        let dbname = {
            let mut dir = GetTempDir();
            dir.push(format!("rocksdb_c_test-{}", test_uuid));
            let path = dir.to_str().unwrap();
            CString::new(path).unwrap()
        };
        let dbbackupname = {
            let mut dir = GetTempDir();
            dir.push(format!("rocksdb_c_test-{}-backup", test_uuid));
            let path = dir.to_str().unwrap();
            CString::new(path).unwrap()
        };

        let dbname = dbname.as_ptr();
        let dbbackupname = dbbackupname.as_ptr();

        StartPhase("create_objects");
        cmp = rocksdb_comparator_create(
            ptr::null_mut(),
            Some(CmpDestroy),
            Some(CmpCompare),
            Some(CmpName),
        );
        env = rocksdb_create_default_env();
        cache = rocksdb_cache_create_lru(100000);

        options = rocksdb_options_create();
        rocksdb_options_set_comparator(options, cmp);
        rocksdb_options_set_error_if_exists(options, 1);
        rocksdb_options_set_env(options, env);
        rocksdb_options_set_info_log(options, ptr::null_mut());
        rocksdb_options_set_write_buffer_size(options, 100000);
        rocksdb_options_set_paranoid_checks(options, 1);
        rocksdb_options_set_max_open_files(options, 10);
        table_options = rocksdb_block_based_options_create();
        rocksdb_block_based_options_set_block_cache(table_options, cache);
        rocksdb_options_set_block_based_table_factory(options, table_options);

        let no_compression = rocksdb_no_compression as c_int;
        rocksdb_options_set_compression(options, no_compression);
        rocksdb_options_set_compression_options(options, -14, -1, 0, 0);
        let mut compression_levels = vec![
            no_compression,
            no_compression,
            no_compression,
            no_compression,
        ];
        rocksdb_options_set_compression_per_level(
            options,
            compression_levels.as_mut_ptr(),
            compression_levels.len() as size_t,
        );

        roptions = rocksdb_readoptions_create();
        rocksdb_readoptions_set_verify_checksums(roptions, 1);
        rocksdb_readoptions_set_fill_cache(roptions, 0);

        woptions = rocksdb_writeoptions_create();
        rocksdb_writeoptions_set_sync(woptions, 1);

        StartPhase("destroy");
        rocksdb_destroy_db(options, dbname, &mut err);
        Free(&mut err);

        StartPhase("open_error");
        rocksdb_open(options, dbname, &mut err);
        CheckCondition!(!err.is_null());
        Free(&mut err);

        StartPhase("open");
        rocksdb_options_set_create_if_missing(options, 1);
        db = rocksdb_open(options, dbname, &mut err);
        CheckNoError!(err);
        CheckGet(db, roptions, cstrp!("foo") as *const _, ptr::null());

        StartPhase("put");
        rocksdb_put(db, woptions, cstrp!("foo"), 3, cstrp!("hello"), 5, &mut err);
        CheckNoError!(err);
        CheckGet(db, roptions, cstrp!("foo"), cstrp!("hello"));

        StartPhase("backup_and_restore");
        {
            rocksdb_destroy_db(options, dbbackupname, &mut err);
            CheckNoError!(err);

            let be = rocksdb_backup_engine_open(options, dbbackupname, &mut err);
            CheckNoError!(err);

            rocksdb_backup_engine_create_new_backup(be, db, &mut err);
            CheckNoError!(err);

            // need a change to trigger a new backup
            rocksdb_delete(db, woptions, cstrp!("does-not-exist"), 14, &mut err);
            CheckNoError!(err);

            rocksdb_backup_engine_create_new_backup(be, db, &mut err);
            CheckNoError!(err);

            let bei: *const rocksdb_backup_engine_info_t =
                rocksdb_backup_engine_get_backup_info(be);
            CheckCondition!(rocksdb_backup_engine_info_count(bei) > 1);
            rocksdb_backup_engine_info_destroy(bei);

            rocksdb_backup_engine_purge_old_backups(be, 1, &mut err);
            CheckNoError!(err);

            let bei: *const rocksdb_backup_engine_info_t =
                rocksdb_backup_engine_get_backup_info(be);
            CheckCondition!(rocksdb_backup_engine_info_count(bei) == 1);
            rocksdb_backup_engine_info_destroy(bei);

            rocksdb_delete(db, woptions, cstrp!("foo"), 3, &mut err);
            CheckNoError!(err);

            rocksdb_close(db);

            rocksdb_destroy_db(options, dbname, &mut err);
            CheckNoError!(err);

            let restore_options = rocksdb_restore_options_create();
            rocksdb_restore_options_set_keep_log_files(restore_options, 0);
            rocksdb_backup_engine_restore_db_from_latest_backup(
                be,
                dbname,
                dbname,
                restore_options,
                &mut err,
            );
            CheckNoError!(err);
            rocksdb_restore_options_destroy(restore_options);

            rocksdb_options_set_error_if_exists(options, 0);
            db = rocksdb_open(options, dbname, &mut err);
            CheckNoError!(err);
            rocksdb_options_set_error_if_exists(options, 1);

            CheckGet(db, roptions, cstrp!("foo"), cstrp!("hello"));

            rocksdb_backup_engine_close(be);
        }

        StartPhase("compactall");
        rocksdb_compact_range(db, ptr::null(), 0, ptr::null(), 0);
        CheckGet(db, roptions, cstrp!("foo"), cstrp!("hello"));

        StartPhase("compactrange");
        rocksdb_compact_range(db, cstrp!("a"), 1, cstrp!("z"), 1);
        CheckGet(db, roptions, cstrp!("foo"), cstrp!("hello"));

        StartPhase("writebatch");
        {
            let mut wb = rocksdb_writebatch_create();
            rocksdb_writebatch_put(wb, cstrp!("foo"), 3, cstrp!("a"), 1);
            rocksdb_writebatch_clear(wb);
            rocksdb_writebatch_put(wb, cstrp!("bar"), 3, cstrp!("b"), 1);
            rocksdb_writebatch_put(wb, cstrp!("box"), 3, cstrp!("c"), 1);
            rocksdb_writebatch_delete(wb, cstrp!("bar"), 3);
            rocksdb_write(db, woptions, wb, &mut err);
            CheckNoError!(err);
            CheckGet(db, roptions, cstrp!("foo"), cstrp!("hello"));
            CheckGet(db, roptions, cstrp!("bar"), ptr::null());
            CheckGet(db, roptions, cstrp!("box"), cstrp!("c"));
            let mut pos: c_int = 0;
            rocksdb_writebatch_iterate(
                wb,
                (&mut pos as *mut c_int).cast::<c_void>(),
                Some(CheckPut),
                Some(CheckDel),
            );
            CheckCondition!(pos == 3);
            rocksdb_writebatch_destroy(wb);
        }

        StartPhase("writebatch_vectors");
        {
            let wb = rocksdb_writebatch_create();
            let k_list: [*const c_char; 2] = [cstrp!("z"), cstrp!("ap")];
            let k_sizes: [size_t; 2] = [1, 2];
            let v_list: [*const c_char; 3] = [cstrp!("x"), cstrp!("y"), cstrp!("z")];
            let v_sizes: [size_t; 3] = [1, 1, 1];
            rocksdb_writebatch_putv(
                wb,
                k_list.len() as c_int,
                k_list.as_ptr(),
                k_sizes.as_ptr(),
                v_list.len() as c_int,
                v_list.as_ptr(),
                v_sizes.as_ptr(),
            );
            rocksdb_write(db, woptions, wb, &mut err);
            CheckNoError!(err);
            CheckGet(db, roptions, cstrp!("zap"), cstrp!("xyz"));
            rocksdb_writebatch_delete(wb, cstrp!("zap"), 3);
            rocksdb_write(db, woptions, wb, &mut err);
            CheckNoError!(err);
            CheckGet(db, roptions, cstrp!("zap"), ptr::null());
            rocksdb_writebatch_destroy(wb);
        }

        StartPhase("writebatch_rep");
        {
            let wb1: *mut rocksdb_writebatch_t = rocksdb_writebatch_create();
            rocksdb_writebatch_put(wb1, cstrp!("baz"), 3, cstrp!("d"), 1);
            rocksdb_writebatch_put(wb1, cstrp!("quux"), 4, cstrp!("e"), 1);
            rocksdb_writebatch_delete(wb1, cstrp!("quux"), 4);
            let mut repsize1: size_t = 0;
            let mut rep = rocksdb_writebatch_data(wb1, &mut repsize1) as *const c_void;
            let mut wb2 = rocksdb_writebatch_create_from(rep as *const c_char, repsize1);
            CheckCondition!(rocksdb_writebatch_count(wb1) == rocksdb_writebatch_count(wb2));
            let mut repsize2: size_t = 0;
            CheckCondition!(
                memcmp(
                    rep,
                    rocksdb_writebatch_data(wb2, &mut repsize2) as *const c_void,
                    repsize1
                ) == 0
            );
            rocksdb_writebatch_destroy(wb1);
            rocksdb_writebatch_destroy(wb2);
        }

        StartPhase("iter");
        {
            let mut iter = rocksdb_create_iterator(db, roptions);
            CheckCondition!(rocksdb_iter_valid(iter) == 0);
            rocksdb_iter_seek_to_first(iter);
            CheckCondition!(rocksdb_iter_valid(iter) != 0);
            CheckIter(iter, cstrp!("box"), cstrp!("c"));
            rocksdb_iter_next(iter);
            CheckIter(iter, cstrp!("foo"), cstrp!("hello"));
            rocksdb_iter_prev(iter);
            CheckIter(iter, cstrp!("box"), cstrp!("c"));
            rocksdb_iter_prev(iter);
            CheckCondition!(rocksdb_iter_valid(iter) == 0);
            rocksdb_iter_seek_to_last(iter);
            CheckIter(iter, cstrp!("foo"), cstrp!("hello"));
            rocksdb_iter_seek(iter, cstrp!("b"), 1);
            CheckIter(iter, cstrp!("box"), cstrp!("c"));
            rocksdb_iter_get_error(iter, &mut err);
            CheckNoError!(err);
            rocksdb_iter_destroy(iter);
        }

        StartPhase("multiget");
        {
            let keys: [*const c_char; 3] = [cstrp!("box"), cstrp!("foo"), cstrp!("notfound")];
            let keys_sizes: [size_t; 3] = [3, 3, 8];
            let mut vals: [*mut c_char; 3] = [ptr::null_mut(), ptr::null_mut(), ptr::null_mut()];
            let mut vals_sizes: [size_t; 3] = [0, 0, 0];
            let mut errs: [*mut c_char; 3] = [ptr::null_mut(), ptr::null_mut(), ptr::null_mut()];
            rocksdb_multi_get(
                db,
                roptions,
                3,
                keys.as_ptr(),
                keys_sizes.as_ptr(),
                vals.as_mut_ptr(),
                vals_sizes.as_mut_ptr(),
                errs.as_mut_ptr(),
            );

            for i in 0..3 {
                CheckEqual(ptr::null(), errs[i], 0);
                match i {
                    0 => CheckEqual(cstrp!("c"), vals[i], vals_sizes[i]),
                    1 => CheckEqual(cstrp!("hello"), vals[i], vals_sizes[i]),
                    2 => CheckEqual(ptr::null(), vals[i], vals_sizes[i]),
                    _ => {}
                }
                Free(&mut vals[i]);
            }
        }

        StartPhase("approximate_sizes");
        {
            let mut sizes: [u64; 2] = [0, 0];
            let start: [*const c_char; 2] = [cstrp!("a"), cstrp!("k00000000000000010000")];
            let start_len: [size_t; 2] = [1, 21];
            let limit: [*const c_char; 2] = [cstrp!("k00000000000000010000"), cstrp!("z")];
            let limit_len: [size_t; 2] = [21, 1];
            rocksdb_writeoptions_set_sync(woptions, 0);
            for i in 0..20000 {
                let keybuf = CString::new(format!("k{:020}", i)).unwrap();
                let key = keybuf.to_bytes_with_nul();
                let valbuf = CString::new(format!("v{:020}", i)).unwrap();
                let val = valbuf.to_bytes_with_nul();
                rocksdb_put(
                    db,
                    woptions,
                    key.as_ptr() as *const c_char,
                    key.len() as size_t,
                    val.as_ptr() as *const c_char,
                    val.len() as size_t,
                    &mut err,
                );
                CheckNoError!(err);
            }
            rocksdb_approximate_sizes(
                db,
                2,
                start.as_ptr(),
                start_len.as_ptr(),
                limit.as_ptr(),
                limit_len.as_ptr(),
                sizes.as_mut_ptr(),
                &mut err,
            );
            CheckNoError!(err);
            CheckCondition!(sizes[0] > 0);
            CheckCondition!(sizes[1] > 0);
        }

        StartPhase("property");
        {
            let mut prop: *mut c_char;
            prop = rocksdb_property_value(db, cstrp!("nosuchprop"));
            CheckCondition!(prop.is_null());
            prop = rocksdb_property_value(db, cstrp!("rocksdb.stats"));
            CheckCondition!(!prop.is_null());
            Free(&mut prop);
        }

        StartPhase("snapshot");
        {
            let snap: *const rocksdb_snapshot_t = rocksdb_create_snapshot(db);
            rocksdb_delete(db, woptions, cstrp!("foo"), 3, &mut err);
            CheckNoError!(err);
            rocksdb_readoptions_set_snapshot(roptions, snap);
            CheckGet(db, roptions, cstrp!("foo"), cstrp!("hello"));
            rocksdb_readoptions_set_snapshot(roptions, ptr::null());
            CheckGet(db, roptions, cstrp!("foo"), ptr::null());
            rocksdb_release_snapshot(db, snap);
        }

        StartPhase("repair");
        {
            // If we do not compact here, then the lazy deletion of files (https://reviews.facebook.net/D6123) would leave around deleted files and the repair process will find those files and put them back into the database.
            rocksdb_compact_range(db, ptr::null(), 0, ptr::null(), 0);
            rocksdb_close(db);
            rocksdb_options_set_create_if_missing(options, 0);
            rocksdb_options_set_error_if_exists(options, 0);
            // rocksdb_options_set_wal_recovery_mode(options, 2);
            rocksdb_repair_db(options, dbname, &mut err);
            CheckNoError!(err);
            db = rocksdb_open(options, dbname, &mut err);
            CheckNoError!(err);
            CheckGet(db, roptions, cstrp!("foo"), ptr::null());
            CheckGet(db, roptions, cstrp!("bar"), ptr::null());
            CheckGet(db, roptions, cstrp!("box"), cstrp!("c"));
            rocksdb_options_set_create_if_missing(options, 1);
            rocksdb_options_set_error_if_exists(options, 1);
        }

        StartPhase("filter");
        for run in 0..2 {
            CheckNoError!(err);
            let mut policy: *mut rocksdb_filterpolicy_t = rocksdb_filterpolicy_create_bloom(10.0);

            rocksdb_block_based_options_set_filter_policy(table_options, policy);

            // Create new database
            rocksdb_close(db);
            rocksdb_destroy_db(options, dbname, &mut err);
            rocksdb_options_set_block_based_table_factory(options, table_options);
            db = rocksdb_open(options, dbname, &mut err);
            CheckNoError!(err);
            rocksdb_put(
                db,
                woptions,
                cstrp!("foo"),
                3,
                cstrp!("foovalue"),
                8,
                &mut err,
            );
            CheckNoError!(err);
            rocksdb_put(
                db,
                woptions,
                cstrp!("bar"),
                3,
                cstrp!("barvalue"),
                8,
                &mut err,
            );
            CheckNoError!(err);
            rocksdb_compact_range(db, ptr::null(), 0, ptr::null(), 0);

            fake_filter_result = 1;
            CheckGet(db, roptions, cstrp!("foo"), cstrp!("foovalue"));
            CheckGet(db, roptions, cstrp!("bar"), cstrp!("barvalue"));
            if phase == "" {
                // Must not find value when custom filter returns false
                fake_filter_result = 0;
                CheckGet(db, roptions, cstrp!("foo"), ptr::null());
                CheckGet(db, roptions, cstrp!("bar"), ptr::null());
                fake_filter_result = 1;

                CheckGet(db, roptions, cstrp!("foo"), cstrp!("foovalue"));
                CheckGet(db, roptions, cstrp!("bar"), cstrp!("barvalue"));
            }
            // Reset the policy
            rocksdb_block_based_options_set_filter_policy(table_options, ptr::null_mut());
            rocksdb_options_set_block_based_table_factory(options, table_options);
        }

        StartPhase("compaction_filter");
        {
            let options_with_filter = rocksdb_options_create();
            rocksdb_options_set_create_if_missing(options_with_filter, 1);
            let cfilter = rocksdb_compactionfilter_create(
                ptr::null_mut(),
                Some(CFilterDestroy),
                Some(CFilterFilter),
                Some(CFilterName),
            );
            // Create new database
            rocksdb_close(db);
            rocksdb_destroy_db(options_with_filter, dbname, &mut err);
            rocksdb_options_set_compaction_filter(options_with_filter, cfilter);
            db = CheckCompaction(dbname, db, options_with_filter, roptions, woptions);

            rocksdb_options_set_compaction_filter(options_with_filter, ptr::null_mut());
            rocksdb_compactionfilter_destroy(cfilter);
            rocksdb_options_destroy(options_with_filter);
        }

        StartPhase("compaction_filter_factory");
        {
            let mut options_with_filter_factory = rocksdb_options_create();
            rocksdb_options_set_create_if_missing(options_with_filter_factory, 1);
            let mut factory = rocksdb_compactionfilterfactory_create(
                ptr::null_mut(),
                Some(CFilterFactoryDestroy),
                Some(CFilterCreate),
                Some(CFilterFactoryName),
            );
            // Create new database
            rocksdb_close(db);
            rocksdb_destroy_db(options_with_filter_factory, dbname, &mut err);
            rocksdb_options_set_compaction_filter_factory(options_with_filter_factory, factory);
            db = CheckCompaction(dbname, db, options_with_filter_factory, roptions, woptions);

            rocksdb_options_set_compaction_filter_factory(
                options_with_filter_factory,
                ptr::null_mut(),
            );
            rocksdb_options_destroy(options_with_filter_factory);
        }

        StartPhase("merge_operator");
        {
            let mut merge_operator = rocksdb_mergeoperator_create(
                ptr::null_mut(),
                Some(MergeOperatorDestroy),
                Some(MergeOperatorFullMerge),
                Some(MergeOperatorPartialMerge),
                None,
                Some(MergeOperatorName),
            );
            // Create new database
            rocksdb_close(db);
            rocksdb_destroy_db(options, dbname, &mut err);
            rocksdb_options_set_merge_operator(options, merge_operator);
            db = rocksdb_open(options, dbname, &mut err);
            CheckNoError!(err);
            rocksdb_put(
                db,
                woptions,
                cstrp!("foo"),
                3,
                cstrp!("foovalue"),
                8,
                &mut err,
            );
            CheckNoError!(err);
            CheckGet(db, roptions, cstrp!("foo"), cstrp!("foovalue"));
            rocksdb_merge(
                db,
                woptions,
                cstrp!("foo"),
                3,
                cstrp!("barvalue"),
                8,
                &mut err,
            );
            CheckNoError!(err);
            CheckGet(db, roptions, cstrp!("foo"), cstrp!("fake"));

            // Merge of a non-existing value
            rocksdb_merge(
                db,
                woptions,
                cstrp!("bar"),
                3,
                cstrp!("barvalue"),
                8,
                &mut err,
            );
            CheckNoError!(err);
            CheckGet(db, roptions, cstrp!("bar"), cstrp!("fake"));
        }

        StartPhase("columnfamilies");
        {
            rocksdb_close(db);
            rocksdb_destroy_db(options, dbname, &mut err);
            CheckNoError!(err);

            let mut db_options = rocksdb_options_create();
            rocksdb_options_set_create_if_missing(db_options, 1);
            db = rocksdb_open(db_options, dbname, &mut err);
            CheckNoError!(err);
            let mut cfh = rocksdb_create_column_family(db, db_options, cstrp!("cf1"), &mut err);
            rocksdb_column_family_handle_destroy(cfh);
            CheckNoError!(err);
            rocksdb_close(db);

            let mut cflen: size_t = 0;
            let column_fams_raw =
                rocksdb_list_column_families(db_options, dbname, &mut cflen, &mut err);
            let column_fams = slice::from_raw_parts(column_fams_raw, cflen as usize);
            CheckEqual(cstrp!("default"), column_fams[0], 7);
            CheckEqual(cstrp!("cf1"), column_fams[1], 3);
            CheckCondition!(cflen == 2);
            rocksdb_list_column_families_destroy(column_fams_raw, cflen);

            let mut cf_options = rocksdb_options_create();

            let mut cf_names: [*const c_char; 2] = [cstrp!("default"), cstrp!("cf1")];
            let mut cf_opts: [*const rocksdb_options_t; 2] = [cf_options, cf_options];
            let mut handles: [*mut rocksdb_column_family_handle_t; 2] =
                [ptr::null_mut(), ptr::null_mut()];
            db = rocksdb_open_column_families(
                db_options,
                dbname,
                2,
                cf_names.as_mut_ptr(),
                cf_opts.as_mut_ptr(),
                handles.as_mut_ptr(),
                &mut err,
            );
            CheckNoError!(err);

            rocksdb_put_cf(
                db,
                woptions,
                handles[1],
                cstrp!("foo"),
                3,
                cstrp!("hello"),
                5,
                &mut err,
            );
            CheckNoError!(err);

            CheckGetCF(db, roptions, handles[1], cstrp!("foo"), cstrp!("hello"));

            rocksdb_delete_cf(db, woptions, handles[1], cstrp!("foo"), 3, &mut err);
            CheckNoError!(err);

            CheckGetCF(db, roptions, handles[1], cstrp!("foo"), ptr::null());

            let mut wb = rocksdb_writebatch_create();
            rocksdb_writebatch_put_cf(wb, handles[1], cstrp!("baz"), 3, cstrp!("a"), 1);
            rocksdb_writebatch_clear(wb);
            rocksdb_writebatch_put_cf(wb, handles[1], cstrp!("bar"), 3, cstrp!("b"), 1);
            rocksdb_writebatch_put_cf(wb, handles[1], cstrp!("box"), 3, cstrp!("c"), 1);
            rocksdb_writebatch_delete_cf(wb, handles[1], cstrp!("bar"), 3);
            rocksdb_write(db, woptions, wb, &mut err);
            CheckNoError!(err);
            CheckGetCF(db, roptions, handles[1], cstrp!("baz"), ptr::null());
            CheckGetCF(db, roptions, handles[1], cstrp!("bar"), ptr::null());
            CheckGetCF(db, roptions, handles[1], cstrp!("box"), cstrp!("c"));
            rocksdb_writebatch_destroy(wb);

            let keys: [*const c_char; 3] = [cstrp!("box"), cstrp!("box"), cstrp!("barfooxx")];
            let get_handles: [*const rocksdb_column_family_handle_t; 3] =
                [handles[0], handles[1], handles[1]];
            let keys_sizes: [size_t; 3] = [3, 3, 8];
            let mut vals: [*mut c_char; 3] = [ptr::null_mut(), ptr::null_mut(), ptr::null_mut()];
            let mut vals_sizes: [size_t; 3] = [0, 0, 0];
            let mut errs: [*mut c_char; 3] = [ptr::null_mut(), ptr::null_mut(), ptr::null_mut()];
            rocksdb_multi_get_cf(
                db,
                roptions,
                get_handles.as_ptr(),
                3,
                keys.as_ptr(),
                keys_sizes.as_ptr(),
                vals.as_mut_ptr(),
                vals_sizes.as_mut_ptr(),
                errs.as_mut_ptr(),
            );

            for i in 0..3 {
                CheckEqual(ptr::null(), errs[i], 0);
                match i {
                    0 => CheckEqual(ptr::null(), vals[i], vals_sizes[i]), // wrong cf
                    1 => CheckEqual(cstrp!("c"), vals[i], vals_sizes[i]), // bingo
                    2 => CheckEqual(ptr::null(), vals[i], vals_sizes[i]), // normal not found
                    _ => {}
                }
                Free(&mut vals[i]);
            }

            let mut iter = rocksdb_create_iterator_cf(db, roptions, handles[1]);
            CheckCondition!(rocksdb_iter_valid(iter) == 0);
            rocksdb_iter_seek_to_first(iter);
            CheckCondition!(rocksdb_iter_valid(iter) != 0);

            let mut i: u32 = 0;
            while rocksdb_iter_valid(iter) != 0 {
                rocksdb_iter_next(iter);
                i += 1;
            }
            CheckCondition!(i == 1);
            rocksdb_iter_get_error(iter, &mut err);
            CheckNoError!(err);
            rocksdb_iter_destroy(iter);

            let mut iters_cf_handles: [*mut rocksdb_column_family_handle_t; 2] =
                [handles[0], handles[1]];
            let mut iters_handles: [*mut rocksdb_iterator_t; 2] =
                [ptr::null_mut(), ptr::null_mut()];
            rocksdb_create_iterators(
                db,
                roptions,
                iters_cf_handles.as_mut_ptr(),
                iters_handles.as_mut_ptr(),
                2,
                &mut err,
            );
            CheckNoError!(err);

            iter = iters_handles[0];
            CheckCondition!(rocksdb_iter_valid(iter) == 0);
            rocksdb_iter_seek_to_first(iter);
            CheckCondition!(rocksdb_iter_valid(iter) == 0);
            rocksdb_iter_destroy(iter);

            iter = iters_handles[1];
            CheckCondition!(rocksdb_iter_valid(iter) == 0);
            rocksdb_iter_seek_to_first(iter);
            CheckCondition!(rocksdb_iter_valid(iter) != 0);

            let mut i: u32 = 0;
            while rocksdb_iter_valid(iter) != 0 {
                rocksdb_iter_next(iter);
                i += 1;
            }
            CheckCondition!(i == 1);
            rocksdb_iter_get_error(iter, &mut err);
            CheckNoError!(err);
            rocksdb_iter_destroy(iter);

            rocksdb_drop_column_family(db, handles[1], &mut err);
            CheckNoError!(err);
            for i in 0..2 {
                rocksdb_column_family_handle_destroy(handles[i]);
            }
            rocksdb_close(db);
            rocksdb_destroy_db(options, dbname, &mut err);
            rocksdb_options_destroy(db_options);
            rocksdb_options_destroy(cf_options);
        }

        StartPhase("prefix");
        {
            // Create new database
            rocksdb_options_set_allow_mmap_reads(options, 1);
            rocksdb_options_set_prefix_extractor(
                options,
                rocksdb_slicetransform_create_fixed_prefix(3),
            );
            rocksdb_options_set_hash_skip_list_rep(options, 5000, 4, 4);
            rocksdb_options_set_plain_table_factory(options, 4, 10, 0.75, 16, 0, 0, 0, 0);
            rocksdb_options_set_allow_concurrent_memtable_write(options, 0);

            db = rocksdb_open(options, dbname, &mut err);
            CheckNoError!(err);

            rocksdb_put(db, woptions, cstrp!("foo1"), 4, cstrp!("foo"), 3, &mut err);
            CheckNoError!(err);
            rocksdb_put(db, woptions, cstrp!("foo2"), 4, cstrp!("foo"), 3, &mut err);
            CheckNoError!(err);
            rocksdb_put(db, woptions, cstrp!("foo3"), 4, cstrp!("foo"), 3, &mut err);
            CheckNoError!(err);
            rocksdb_put(db, woptions, cstrp!("bar1"), 4, cstrp!("bar"), 3, &mut err);
            CheckNoError!(err);
            rocksdb_put(db, woptions, cstrp!("bar2"), 4, cstrp!("bar"), 3, &mut err);
            CheckNoError!(err);
            rocksdb_put(db, woptions, cstrp!("bar3"), 4, cstrp!("bar"), 3, &mut err);
            CheckNoError!(err);

            let mut iter = rocksdb_create_iterator(db, roptions);
            CheckCondition!(rocksdb_iter_valid(iter) == 0);

            rocksdb_iter_seek(iter, cstrp!("bar"), 3);
            rocksdb_iter_get_error(iter, &mut err);
            CheckNoError!(err);
            CheckCondition!(rocksdb_iter_valid(iter) != 0);

            CheckIter(iter, cstrp!("bar1"), cstrp!("bar"));
            rocksdb_iter_next(iter);
            CheckIter(iter, cstrp!("bar2"), cstrp!("bar"));
            rocksdb_iter_next(iter);
            CheckIter(iter, cstrp!("bar3"), cstrp!("bar"));
            rocksdb_iter_get_error(iter, &mut err);
            CheckNoError!(err);
            rocksdb_iter_destroy(iter);

            rocksdb_close(db);
            rocksdb_destroy_db(options, dbname, &mut err);
        }

        StartPhase("cuckoo_options");
        {
            let mut cuckoo_options = rocksdb_cuckoo_options_create();
            rocksdb_cuckoo_options_set_hash_ratio(cuckoo_options, 0.5);
            rocksdb_cuckoo_options_set_max_search_depth(cuckoo_options, 200);
            rocksdb_cuckoo_options_set_cuckoo_block_size(cuckoo_options, 10);
            rocksdb_cuckoo_options_set_identity_as_first_hash(cuckoo_options, 1);
            rocksdb_cuckoo_options_set_use_module_hash(cuckoo_options, 0);
            rocksdb_options_set_cuckoo_table_factory(options, cuckoo_options);

            db = rocksdb_open(options, dbname, &mut err);
            CheckNoError!(err);

            rocksdb_cuckoo_options_destroy(cuckoo_options);
        }

        StartPhase("iterate_upper_bound");
        {
            // Create new empty database
            rocksdb_close(db);
            rocksdb_destroy_db(options, dbname, &mut err);
            CheckNoError!(err);

            rocksdb_options_set_prefix_extractor(options, ptr::null_mut());
            db = rocksdb_open(options, dbname, &mut err);
            CheckNoError!(err);

            rocksdb_put(db, woptions, cstrp!("a"), 1, cstrp!("0"), 1, &mut err);
            CheckNoError!(err);
            rocksdb_put(db, woptions, cstrp!("foo"), 3, cstrp!("bar"), 3, &mut err);
            CheckNoError!(err);
            rocksdb_put(db, woptions, cstrp!("foo1"), 4, cstrp!("bar1"), 4, &mut err);
            CheckNoError!(err);
            rocksdb_put(db, woptions, cstrp!("g1"), 2, cstrp!("0"), 1, &mut err);
            CheckNoError!(err);

            // testing basic case with no iterate_upper_bound and no prefix_extractor
            {
                rocksdb_readoptions_set_iterate_upper_bound(roptions, ptr::null(), 0);
                let mut iter = rocksdb_create_iterator(db, roptions);

                rocksdb_iter_seek(iter, cstrp!("foo"), 3);
                CheckCondition!(rocksdb_iter_valid(iter) != 0);
                CheckIter(iter, cstrp!("foo"), cstrp!("bar"));

                rocksdb_iter_next(iter);
                CheckCondition!(rocksdb_iter_valid(iter) != 0);
                CheckIter(iter, cstrp!("foo1"), cstrp!("bar1"));

                rocksdb_iter_next(iter);
                CheckCondition!(rocksdb_iter_valid(iter) != 0);
                CheckIter(iter, cstrp!("g1"), cstrp!("0"));

                rocksdb_iter_destroy(iter);
            }

            // testing iterate_upper_bound and forward iterator
            // to make sure it stops at bound
            {
                // iterate_upper_bound points beyond the last expected entry
                rocksdb_readoptions_set_iterate_upper_bound(roptions, cstrp!("foo2"), 4);

                let mut iter = rocksdb_create_iterator(db, roptions);

                rocksdb_iter_seek(iter, cstrp!("foo"), 3);
                CheckCondition!(rocksdb_iter_valid(iter) != 0);
                CheckIter(iter, cstrp!("foo"), cstrp!("bar"));

                rocksdb_iter_next(iter);
                CheckCondition!(rocksdb_iter_valid(iter) != 0);
                CheckIter(iter, cstrp!("foo1"), cstrp!("bar1"));

                rocksdb_iter_next(iter);
                // should stop here...
                CheckCondition!(rocksdb_iter_valid(iter) == 0);

                rocksdb_iter_destroy(iter);
            }
        }

        // Simple sanity check that setting memtable rep works.
        StartPhase("memtable_reps");
        {
            // Create database with vector memtable.
            rocksdb_close(db);
            rocksdb_destroy_db(options, dbname, &mut err);
            CheckNoError!(err);

            rocksdb_options_set_memtable_vector_rep(options);
            db = rocksdb_open(options, dbname, &mut err);
            CheckNoError!(err);

            // // Create database with hash skiplist memtable.
            // rocksdb_close(db);
            // rocksdb_destroy_db(options, dbname, &mut err);
            // CheckNoError!(err);
            //
            // rocksdb_options_set_hash_skip_list_rep(options, 5000, 4, 4);
            // db = rocksdb_open(options, dbname, &mut err);
            // CheckNoError!(err);
        }

        StartPhase("cleanup");
        rocksdb_close(db);
        rocksdb_options_destroy(options);
        rocksdb_block_based_options_destroy(table_options);
        rocksdb_readoptions_destroy(roptions);
        rocksdb_writeoptions_destroy(woptions);
        rocksdb_cache_destroy(cache);
        rocksdb_comparator_destroy(cmp);
        rocksdb_env_destroy(env);

        err_println!("PASS");
    }
}
