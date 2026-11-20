use rocksdb::compaction_filter::{CompactionFilter, Decision};
use std::ffi::CStr;

pub struct Filter;

impl CompactionFilter for Filter {
    fn filter(&mut self, _level: u32, _key: &[u8], _value: &[u8]) -> Decision {
        Decision::Keep
    }

    fn name(&self) -> &CStr {
        c"Filter"
    }
}
