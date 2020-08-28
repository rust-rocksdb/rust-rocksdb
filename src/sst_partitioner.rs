// Copyright 2020 TiKV Project Authors. Licensed under Apache-2.0.

use super::SstPartitionerResult;
use crocksdb_ffi::{
    self, DBSstPartitioner, DBSstPartitionerContext, DBSstPartitionerFactory,
    DBSstPartitionerRequest,
};
use libc::{c_char, c_uchar, c_void, size_t};
use std::{ffi::CString, ptr, slice};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SstPartitionerRequest<'a> {
    pub prev_user_key: &'a [u8],
    pub current_user_key: &'a [u8],
    pub current_output_file_size: u64,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SstPartitionerContext<'a> {
    pub is_full_compaction: bool,
    pub is_manual_compaction: bool,
    pub output_level: i32,
    pub smallest_key: &'a [u8],
    pub largest_key: &'a [u8],
}

pub trait SstPartitioner {
    fn should_partition(&self, req: &SstPartitionerRequest) -> SstPartitionerResult;
    fn can_do_trivial_move(&self, smallest_user_key: &[u8], largest_user_key: &[u8]) -> bool;
}

extern "C" fn sst_partitioner_destructor<P: SstPartitioner>(ctx: *mut c_void) {
    unsafe {
        // Recover from raw pointer and implicitly drop.
        Box::from_raw(ctx as *mut P);
    }
}

extern "C" fn sst_partitioner_should_partition<P: SstPartitioner>(
    ctx: *mut c_void,
    request: *mut DBSstPartitionerRequest,
) -> SstPartitionerResult {
    let partitioner = unsafe { &*(ctx as *mut P) };
    let req = unsafe {
        let mut prev_key_len: usize = 0;
        let prev_key = crocksdb_ffi::crocksdb_sst_partitioner_request_prev_user_key(
            request,
            &mut prev_key_len,
        ) as *const u8;
        let mut current_key_len: usize = 0;
        let current_key = crocksdb_ffi::crocksdb_sst_partitioner_request_current_user_key(
            request,
            &mut current_key_len,
        ) as *const u8;
        SstPartitionerRequest {
            prev_user_key: slice::from_raw_parts(prev_key, prev_key_len),
            current_user_key: slice::from_raw_parts(current_key, current_key_len),
            current_output_file_size:
                crocksdb_ffi::crocksdb_sst_partitioner_request_current_output_file_size(request),
        }
    };
    partitioner.should_partition(&req) as _
}

extern "C" fn sst_partitioner_can_do_trivial_move<P: SstPartitioner>(
    ctx: *mut c_void,
    smallest_user_key: *const c_char,
    smallest_user_key_len: size_t,
    largest_user_key: *const c_char,
    largest_user_key_len: size_t,
) -> c_uchar {
    let partitioner = unsafe { &*(ctx as *mut P) };
    let smallest_key =
        unsafe { slice::from_raw_parts(smallest_user_key as *const u8, smallest_user_key_len) };
    let largest_key =
        unsafe { slice::from_raw_parts(largest_user_key as *const u8, largest_user_key_len) };
    partitioner.can_do_trivial_move(smallest_key, largest_key) as _
}

pub trait SstPartitionerFactory: Sync + Send {
    type Partitioner: SstPartitioner + 'static;

    fn name(&self) -> &CString;
    fn create_partitioner(&self, context: &SstPartitionerContext) -> Option<Self::Partitioner>;
}

extern "C" fn sst_partitioner_factory_destroy<F: SstPartitionerFactory>(ctx: *mut c_void) {
    unsafe {
        // Recover from raw pointer and implicitly drop.
        Box::from_raw(ctx as *mut F);
    }
}

extern "C" fn sst_partitioner_factory_name<F: SstPartitionerFactory>(
    ctx: *mut c_void,
) -> *const c_char {
    let factory = unsafe { &*(ctx as *mut F) };
    factory.name().as_ptr()
}

extern "C" fn sst_partitioner_factory_create_partitioner<F: SstPartitionerFactory>(
    ctx: *mut c_void,
    context: *mut DBSstPartitionerContext,
) -> *mut DBSstPartitioner {
    let factory = unsafe { &*(ctx as *mut F) };
    let context = unsafe {
        let mut smallest_key_len: usize = 0;
        let smallest_key = crocksdb_ffi::crocksdb_sst_partitioner_context_smallest_key(
            context,
            &mut smallest_key_len,
        ) as *const u8;
        let mut largest_key_len: usize = 0;
        let largest_key = crocksdb_ffi::crocksdb_sst_partitioner_context_largest_key(
            context,
            &mut largest_key_len,
        ) as *const u8;
        SstPartitionerContext {
            is_full_compaction: crocksdb_ffi::crocksdb_sst_partitioner_context_is_full_compaction(
                context,
            ) != 0,
            is_manual_compaction:
                crocksdb_ffi::crocksdb_sst_partitioner_context_is_manual_compaction(context) != 0,
            output_level: crocksdb_ffi::crocksdb_sst_partitioner_context_output_level(context),
            smallest_key: slice::from_raw_parts(smallest_key, smallest_key_len),
            largest_key: slice::from_raw_parts(largest_key, largest_key_len),
        }
    };
    match factory.create_partitioner(&context) {
        None => ptr::null_mut(),
        Some(partitioner) => {
            let ctx = Box::into_raw(Box::new(partitioner)) as *mut c_void;
            unsafe {
                crocksdb_ffi::crocksdb_sst_partitioner_create(
                    ctx,
                    sst_partitioner_destructor::<F::Partitioner>,
                    sst_partitioner_should_partition::<F::Partitioner>,
                    sst_partitioner_can_do_trivial_move::<F::Partitioner>,
                )
            }
        }
    }
}

pub fn new_sst_partitioner_factory<F: SstPartitionerFactory>(
    factory: F,
) -> *mut DBSstPartitionerFactory {
    unsafe {
        crocksdb_ffi::crocksdb_sst_partitioner_factory_create(
            Box::into_raw(Box::new(factory)) as *mut c_void,
            sst_partitioner_factory_destroy::<F>,
            sst_partitioner_factory_name::<F>,
            sst_partitioner_factory_create_partitioner::<F>,
        )
    }
}

#[cfg(test)]
mod test {
    use std::{
        ffi::{CStr, CString},
        sync::{Arc, Mutex},
    };

    use super::*;

    struct TestState {
        pub call_create_partitioner: usize,
        pub call_should_partition: usize,
        pub call_can_do_trivial_move: usize,
        pub drop_partitioner: usize,
        pub drop_factory: usize,
        pub should_partition_result: SstPartitionerResult,
        pub can_do_trivial_move_result: bool,
        pub no_partitioner: bool,

        // SstPartitionerRequest fields
        pub prev_user_key: Option<Vec<u8>>,
        pub current_user_key: Option<Vec<u8>>,
        pub current_output_file_size: Option<u64>,

        // can_do_trivial_move params
        pub trivial_move_smallest_key: Option<Vec<u8>>,
        pub trivial_move_largest_key: Option<Vec<u8>>,

        // SstPartitionerContext fields
        pub is_full_compaction: Option<bool>,
        pub is_manual_compaction: Option<bool>,
        pub output_level: Option<i32>,
        pub smallest_key: Option<Vec<u8>>,
        pub largest_key: Option<Vec<u8>>,
    }

    impl Default for TestState {
        fn default() -> Self {
            TestState {
                call_create_partitioner: 0,
                call_should_partition: 0,
                call_can_do_trivial_move: 0,
                drop_partitioner: 0,
                drop_factory: 0,
                should_partition_result: SstPartitionerResult::NotRequired,
                can_do_trivial_move_result: false,
                no_partitioner: false,
                prev_user_key: None,
                current_user_key: None,
                current_output_file_size: None,
                trivial_move_smallest_key: None,
                trivial_move_largest_key: None,
                is_full_compaction: None,
                is_manual_compaction: None,
                output_level: None,
                smallest_key: None,
                largest_key: None,
            }
        }
    }

    struct TestSstPartitioner {
        state: Arc<Mutex<TestState>>,
    }

    impl SstPartitioner for TestSstPartitioner {
        fn should_partition(&self, req: &SstPartitionerRequest) -> SstPartitionerResult {
            let mut s = self.state.lock().unwrap();
            s.call_should_partition += 1;
            s.prev_user_key = Some(req.prev_user_key.to_vec());
            s.current_user_key = Some(req.current_user_key.to_vec());
            s.current_output_file_size = Some(req.current_output_file_size);

            s.should_partition_result
        }

        fn can_do_trivial_move(&self, smallest_key: &[u8], largest_key: &[u8]) -> bool {
            let mut s = self.state.lock().unwrap();
            s.call_can_do_trivial_move += 1;
            s.trivial_move_smallest_key = Some(smallest_key.to_vec());
            s.trivial_move_largest_key = Some(largest_key.to_vec());

            s.can_do_trivial_move_result
        }
    }

    impl Drop for TestSstPartitioner {
        fn drop(&mut self) {
            self.state.lock().unwrap().drop_partitioner += 1;
        }
    }

    lazy_static! {
        static ref FACTORY_NAME: CString =
            CString::new(b"TestSstPartitionerFactory".to_vec()).unwrap();
    }

    struct TestSstPartitionerFactory {
        state: Arc<Mutex<TestState>>,
    }

    impl SstPartitionerFactory for TestSstPartitionerFactory {
        type Partitioner = TestSstPartitioner;

        fn name(&self) -> &CString {
            &FACTORY_NAME
        }

        fn create_partitioner(&self, context: &SstPartitionerContext) -> Option<Self::Partitioner> {
            let mut s = self.state.lock().unwrap();
            s.call_create_partitioner += 1;
            if s.no_partitioner {
                return None;
            }
            s.is_full_compaction = Some(context.is_full_compaction);
            s.is_manual_compaction = Some(context.is_manual_compaction);
            s.output_level = Some(context.output_level);
            s.smallest_key = Some(context.smallest_key.to_vec());
            s.largest_key = Some(context.largest_key.to_vec());

            Some(TestSstPartitioner {
                state: self.state.clone(),
            })
        }
    }

    impl Drop for TestSstPartitionerFactory {
        fn drop(&mut self) {
            self.state.lock().unwrap().drop_factory += 1;
        }
    }

    #[test]
    fn factory_name() {
        let s = Arc::new(Mutex::new(TestState::default()));
        let factory = new_sst_partitioner_factory(TestSstPartitionerFactory { state: s });
        let factory_name =
            unsafe { CStr::from_ptr(crocksdb_ffi::crocksdb_sst_partitioner_factory_name(factory)) };
        assert_eq!(*FACTORY_NAME.as_c_str(), *factory_name);
        unsafe {
            crocksdb_ffi::crocksdb_sst_partitioner_factory_destroy(factory);
        }
    }

    #[test]
    fn factory_create_partitioner() {
        const IS_FULL_COMPACTION: bool = false;
        const IS_MANUAL_COMPACTION: bool = true;
        const OUTPUT_LEVEL: i32 = 3;
        const SMALLEST_KEY: &[u8] = b"aaaa";
        const LARGEST_KEY: &[u8] = b"bbbb";

        let s = Arc::new(Mutex::new(TestState::default()));
        let factory = new_sst_partitioner_factory(TestSstPartitionerFactory { state: s.clone() });
        let context = unsafe { crocksdb_ffi::crocksdb_sst_partitioner_context_create() };
        unsafe {
            crocksdb_ffi::crocksdb_sst_partitioner_context_set_is_full_compaction(
                context,
                IS_FULL_COMPACTION as _,
            );
            crocksdb_ffi::crocksdb_sst_partitioner_context_set_is_manual_compaction(
                context,
                IS_MANUAL_COMPACTION as _,
            );
            crocksdb_ffi::crocksdb_sst_partitioner_context_set_output_level(context, OUTPUT_LEVEL);
            crocksdb_ffi::crocksdb_sst_partitioner_context_set_smallest_key(
                context,
                SMALLEST_KEY.as_ptr() as *const c_char,
                SMALLEST_KEY.len(),
            );
            crocksdb_ffi::crocksdb_sst_partitioner_context_set_largest_key(
                context,
                LARGEST_KEY.as_ptr() as *const c_char,
                LARGEST_KEY.len(),
            );
        }
        let partitioner = unsafe {
            crocksdb_ffi::crocksdb_sst_partitioner_factory_create_partitioner(factory, context)
        };
        {
            let sl = s.lock().unwrap();
            assert_eq!(1, sl.call_create_partitioner);
            assert_eq!(IS_FULL_COMPACTION, sl.is_full_compaction.unwrap());
            assert_eq!(IS_MANUAL_COMPACTION, sl.is_manual_compaction.unwrap());
            assert_eq!(OUTPUT_LEVEL, sl.output_level.unwrap());
            assert_eq!(SMALLEST_KEY, sl.smallest_key.as_ref().unwrap().as_slice());
            assert_eq!(LARGEST_KEY, sl.largest_key.as_ref().unwrap().as_slice());
        }
        unsafe {
            crocksdb_ffi::crocksdb_sst_partitioner_destroy(partitioner);
            crocksdb_ffi::crocksdb_sst_partitioner_factory_destroy(factory);
        }
    }

    #[test]
    fn factory_create_no_partitioner() {
        let s = Arc::new(Mutex::new(TestState::default()));
        s.lock().unwrap().no_partitioner = true;
        let factory = new_sst_partitioner_factory(TestSstPartitionerFactory { state: s.clone() });
        let context = unsafe { crocksdb_ffi::crocksdb_sst_partitioner_context_create() };
        let partitioner = unsafe {
            crocksdb_ffi::crocksdb_sst_partitioner_factory_create_partitioner(factory, context)
        };
        assert_eq!(1, s.lock().unwrap().call_create_partitioner);
        assert_eq!(ptr::null_mut(), partitioner);
        unsafe {
            crocksdb_ffi::crocksdb_sst_partitioner_factory_destroy(factory);
        }
    }

    #[test]
    fn partitioner_should_partition() {
        const SHOULD_PARTITION: SstPartitionerResult = SstPartitionerResult::Required;
        const PREV_KEY: &[u8] = b"test_key_abc";
        const CURRENT_KEY: &[u8] = b"test_key_def";
        const CURRENT_OUTPUT_FILE_SIZE: u64 = 1234567;

        let s = Arc::new(Mutex::new(TestState::default()));
        s.lock().unwrap().should_partition_result = SHOULD_PARTITION;
        let factory = new_sst_partitioner_factory(TestSstPartitionerFactory { state: s.clone() });
        let context = unsafe { crocksdb_ffi::crocksdb_sst_partitioner_context_create() };
        let partitioner = unsafe {
            crocksdb_ffi::crocksdb_sst_partitioner_factory_create_partitioner(factory, context)
        };
        let req = unsafe { crocksdb_ffi::crocksdb_sst_partitioner_request_create() };
        unsafe {
            crocksdb_ffi::crocksdb_sst_partitioner_request_set_prev_user_key(
                req,
                PREV_KEY.as_ptr() as *const c_char,
                PREV_KEY.len(),
            );
            crocksdb_ffi::crocksdb_sst_partitioner_request_set_current_user_key(
                req,
                CURRENT_KEY.as_ptr() as *const c_char,
                CURRENT_KEY.len(),
            );
            crocksdb_ffi::crocksdb_sst_partitioner_request_set_current_output_file_size(
                req,
                CURRENT_OUTPUT_FILE_SIZE,
            );
        }
        let should_partition =
            unsafe { crocksdb_ffi::crocksdb_sst_partitioner_should_partition(partitioner, req) };
        assert_eq!(SHOULD_PARTITION, should_partition);
        {
            let sl = s.lock().unwrap();
            assert_eq!(1, sl.call_create_partitioner);
            assert_eq!(1, sl.call_should_partition);
            assert_eq!(0, sl.call_can_do_trivial_move);
            assert_eq!(PREV_KEY, sl.prev_user_key.as_ref().unwrap().as_slice());
            assert_eq!(
                CURRENT_KEY,
                sl.current_user_key.as_ref().unwrap().as_slice()
            );
            assert_eq!(
                CURRENT_OUTPUT_FILE_SIZE,
                sl.current_output_file_size.unwrap()
            );
        }
        unsafe {
            crocksdb_ffi::crocksdb_sst_partitioner_destroy(partitioner);
            crocksdb_ffi::crocksdb_sst_partitioner_factory_destroy(factory);
        }
    }

    #[test]
    fn partitioner_can_do_trivial_move() {
        const SMALLEST_KEY: &[u8] = b"test_key_abc";
        const LARGEST_KEY: &[u8] = b"test_key_def";
        const RESULT: bool = true;

        let s = Arc::new(Mutex::new(TestState::default()));
        s.lock().unwrap().can_do_trivial_move_result = RESULT;
        let factory = new_sst_partitioner_factory(TestSstPartitionerFactory { state: s.clone() });
        let context = unsafe { crocksdb_ffi::crocksdb_sst_partitioner_context_create() };
        let partitioner = unsafe {
            crocksdb_ffi::crocksdb_sst_partitioner_factory_create_partitioner(factory, context)
        };
        let result = unsafe {
            crocksdb_ffi::crocksdb_sst_partitioner_can_do_trivial_move(
                partitioner,
                SMALLEST_KEY.as_ptr() as *const c_char,
                SMALLEST_KEY.len(),
                LARGEST_KEY.as_ptr() as *const c_char,
                LARGEST_KEY.len(),
            )
        };
        {
            let sl = s.lock().unwrap();
            assert_eq!(1, sl.call_create_partitioner);
            assert_eq!(0, sl.call_should_partition);
            assert_eq!(1, sl.call_can_do_trivial_move);
            assert_eq!(
                SMALLEST_KEY,
                sl.trivial_move_smallest_key.as_ref().unwrap().as_slice()
            );
            assert_eq!(
                LARGEST_KEY,
                sl.trivial_move_largest_key.as_ref().unwrap().as_slice()
            );
            assert_eq!(RESULT, result);
        }
        unsafe {
            crocksdb_ffi::crocksdb_sst_partitioner_destroy(partitioner);
            crocksdb_ffi::crocksdb_sst_partitioner_factory_destroy(factory);
        }
    }

    #[test]
    fn drop() {
        let s = Arc::new(Mutex::new(TestState::default()));
        let factory = new_sst_partitioner_factory(TestSstPartitionerFactory { state: s.clone() });
        let context = unsafe { crocksdb_ffi::crocksdb_sst_partitioner_context_create() };
        let partitioner = unsafe {
            crocksdb_ffi::crocksdb_sst_partitioner_factory_create_partitioner(factory, context)
        };
        {
            let sl = s.lock().unwrap();
            assert_eq!(0, sl.drop_partitioner);
            assert_eq!(0, sl.drop_factory);
        }
        unsafe {
            crocksdb_ffi::crocksdb_sst_partitioner_destroy(partitioner);
        }
        {
            let sl = s.lock().unwrap();
            assert_eq!(1, sl.drop_partitioner);
            assert_eq!(0, sl.drop_factory);
        }
        unsafe {
            crocksdb_ffi::crocksdb_sst_partitioner_factory_destroy(factory);
        }
        {
            let sl = s.lock().unwrap();
            assert_eq!(1, sl.drop_partitioner);
            assert_eq!(1, sl.drop_factory);
        }
    }
}
