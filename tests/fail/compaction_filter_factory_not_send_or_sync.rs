#[path = "support/compaction_filter.rs"]
mod compaction_filter;

use compaction_filter::Filter;
use rocksdb::compaction_filter_factory::{CompactionFilterContext, CompactionFilterFactory};
use std::ffi::CStr;
use std::rc::Rc;

struct Factory(Rc<u32>);

impl CompactionFilterFactory for Factory {
    type Filter = Filter;

    fn create(&self, _context: CompactionFilterContext) -> Self::Filter {
        Filter
    }

    fn name(&self) -> &CStr {
        c"Factory"
    }
}

fn main() {}
