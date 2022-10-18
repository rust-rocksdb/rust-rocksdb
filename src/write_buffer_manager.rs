/// copy from https://github.com/bh1xuw/rust-rocks/blob/master/src/write_buffer_manager.rs
///! `WriteBufferManager` is for managing memory allocation for one or more
///! MemTables.
use crate::{ffi, Cache};

/// `WriteBufferManager` is for managing memory allocation for one or more
/// MemTables.
pub struct WriteBufferManager {
    pub(crate) inner: *mut ffi::rocksdb_write_buffer_manager_t,
}
unsafe impl Send for WriteBufferManager {}
unsafe impl Sync for WriteBufferManager {}

impl Drop for WriteBufferManager {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_write_buffer_manager_destroy(self.inner);
        }
    }
}

impl WriteBufferManager {
    /// _buffer_size = 0 indicates no limit. Memory won't be tracked,
    /// memory_usage() won't be valid and ShouldFlush() will always return true.
    pub fn new(buffer_size: usize) -> WriteBufferManager {
        WriteBufferManager {
            inner: unsafe { ffi::rocksdb_write_buffer_manager_create(buffer_size) },
        }
    }
    // Parameters:
    // _buffer_size: _buffer_size = 0 indicates no limit. Memory won't be capped.
    // memory_usage() won't be valid and ShouldFlush() will always return true.
    //
    // cache_: if `cache` is provided, we'll put dummy entries in the cache and
    // cost the memory allocated to the cache. It can be used even if _buffer_size
    // = 0.
    //
    // allow_stall: if set true, it will enable stalling of writes when
    // memory_usage() exceeds buffer_size. It will wait for flush to complete and
    // memory usage to drop down.
    pub fn new_with_cache(
        buffer_size: usize,
        cache: &Cache,
        allow_stall: bool,
    ) -> WriteBufferManager {
        WriteBufferManager {
            inner: unsafe {
                ffi::rocksdb_write_buffer_manager_create_with_cache(
                    buffer_size,
                    cache.0.inner,
                    allow_stall,
                )
            },
        }
    }

    // Returns true if buffer_limit is passed to limit the total memory usage and
    // is greater than 0.
    pub fn enabled(&self) -> bool {
        unsafe { ffi::rocksdb_write_buffer_manager_enabled(self.inner) != 0 }
    }

    // Returns the total memory used by memtables.
    // Only valid if enabled()
    pub fn memory_usage(&self) -> usize {
        unsafe { ffi::rocksdb_write_buffer_manager_memory_usage(self.inner) }
    }

    // Returns the buffer_size.
    pub fn buffer_size(&self) -> usize {
        unsafe { ffi::rocksdb_write_buffer_manager_buffer_size(self.inner) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Options, DB};
    use std::iter;
    use tempfile::TempDir;

    #[test]
    #[ignore]
    fn write_buffer_manager_of_2db() {
        let tmp_dir1 = TempDir::new().unwrap();
        let tmp_dir2 = TempDir::new().unwrap();
        let cache = Cache::new_lru_cache(10240).unwrap();
        let manager = WriteBufferManager::new_with_cache(102400, &cache, false);
        let mut op1 = Options::default();
        op1.create_if_missing(true);
        op1.set_write_buffer_manager(&manager);
        let mut op2 = Options::default();
        op2.create_if_missing(true);
        op2.set_write_buffer_manager(&manager);
        assert_eq!(manager.memory_usage(), 0);
        let db1 = DB::open(&op1, &tmp_dir1).unwrap();

        let mem1 = manager.memory_usage();

        let db2 = DB::open(&op2, &tmp_dir2).unwrap();

        assert_eq!(manager.enabled(), true);
        let mem2 = manager.memory_usage();
        assert!(mem2 > mem1);
        // println!("mem1:{}, mem2:{}", mem1, mem2);

        for i in 0..100 {
            let key = format!("k{}", i);
            let val = format!("v{}", i * i);
            let value: String = iter::repeat(val).take(i * i).collect::<Vec<_>>().concat();

            db1.put(key.as_bytes(), value.as_bytes()).unwrap();
        }

        let mem3 = manager.memory_usage();
        // println!("mem2:{}, mem3:{}", mem2, mem3);
        assert!(mem3 > mem2);

        for i in 0..100 {
            let key = format!("k{}", i);
            let val = format!("v{}", i * i);
            let value: String = iter::repeat(val).take(i * i).collect::<Vec<_>>().concat();

            db2.put(key.as_bytes(), value.as_bytes()).unwrap();
        }

        let mem4 = manager.memory_usage();
        // println!(
        //     "mem3:{}, mem4:{}, manager.buffer_size():{}",
        //     mem3,
        //     mem4,
        //     manager.buffer_size()
        // );
        assert!(mem4 > mem3);

        assert!(db2.flush().is_ok());
        let mem5 = manager.memory_usage();
        assert!(mem5 < mem4);
        // println!("mem5:{}", mem5);
        drop(db1);
        drop(db2);
        assert_eq!(manager.memory_usage(), 0);
    }
}
