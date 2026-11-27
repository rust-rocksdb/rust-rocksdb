use rocksdb::Options;
use std::cell::Cell;
use std::rc::Rc;

fn main() {
    let state = Rc::new(Cell::new(0u32));
    let compare = move |a: &[u8], b: &[u8]| {
        state.set(state.get() + 1);
        a.cmp(b)
    };
    let mut opts = Options::default();
    opts.set_comparator("cmp", Box::new(compare));
}
