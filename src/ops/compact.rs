use super::columnfamily::GetColumnFamilys;
use crate::{ffi_util::opt_bytes_to_ptr, handle::Handle, ColumnFamily};
use libc::size_t;

pub trait CompactRange {
    fn compact_range<S: AsRef<[u8]>, E: AsRef<[u8]>>(&self, start: Option<S>, end: Option<E>);
}

pub trait CompactRangeCF {
    fn compact_range_cf(&self, cf: &ColumnFamily, start: Option<&[u8]>, end: Option<&[u8]>);
}

impl<T> CompactRange for T
where
    T: Handle<ffi::rocksdb_t> + super::Write,
{
    fn compact_range<S: AsRef<[u8]>, E: AsRef<[u8]>>(&self, start: Option<S>, end: Option<E>) {
        unsafe {
            let start = start.as_ref().map(AsRef::as_ref);
            let end = end.as_ref().map(AsRef::as_ref);

            ffi::rocksdb_compact_range(
                self.handle(),
                opt_bytes_to_ptr(start),
                start.map_or(0, |s| s.len()) as size_t,
                opt_bytes_to_ptr(end),
                end.map_or(0, |e| e.len()) as size_t,
            );
        }
    }
}

impl<T> CompactRangeCF for T
where
    T: Handle<ffi::rocksdb_t> + super::Write + GetColumnFamilys,
{
    fn compact_range_cf(&self, cf: &ColumnFamily, start: Option<&[u8]>, end: Option<&[u8]>) {
        unsafe {
            ffi::rocksdb_compact_range_cf(
                self.handle(),
                cf.inner,
                opt_bytes_to_ptr(start),
                start.map_or(0, |s| s.len()) as size_t,
                opt_bytes_to_ptr(end),
                end.map_or(0, |e| e.len()) as size_t,
            );
        }
    }
}
