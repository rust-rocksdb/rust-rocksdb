use crate::{ColumnFamily, DBIterator, DBRawIterator, Direction, Error, IteratorMode, ReadOptions};

pub trait Iterate {
    fn get_raw_iter(&self, readopts: &ReadOptions) -> DBRawIterator;

    fn get_iter(&self, readopts: &ReadOptions, mode: IteratorMode) -> DBIterator {
        let mut rv = DBIterator {
            raw: self.get_raw_iter(readopts),
            direction: Direction::Forward, // blown away by set_mode()
            just_seeked: false,
        };
        rv.set_mode(mode);
        rv
    }

    fn iterator_opt(&self, mode: IteratorMode, readopts: &ReadOptions) -> DBIterator {
        self.get_iter(readopts, mode)
    }

    fn iterator(&self, mode: IteratorMode) -> DBIterator {
        let readopts = ReadOptions::default();
        self.iterator_opt(mode, &readopts)
    }

    fn full_iterator(&self, mode: IteratorMode) -> DBIterator {
        let mut opts = ReadOptions::default();
        opts.set_total_order_seek(true);
        self.get_iter(&opts, mode)
    }

    fn prefix_iterator(&self, prefix: &[u8]) -> DBIterator {
        let mut opts = ReadOptions::default();
        opts.set_prefix_same_as_start(true);
        self.get_iter(&opts, IteratorMode::From(prefix, Direction::Forward))
    }

    fn raw_iterator(&self) -> DBRawIterator {
        let opts = ReadOptions::default();
        self.get_raw_iter(&opts)
    }
}

pub trait IterateCF {
    fn get_raw_iter_cf(
        &self,
        cf_handle: &ColumnFamily,
        readopts: &ReadOptions,
    ) -> Result<DBRawIterator, Error>;

    fn get_iter_cf(
        &self,
        cf_handle: &ColumnFamily,
        readopts: &ReadOptions,
        mode: IteratorMode,
    ) -> Result<DBIterator, Error> {
        let mut rv = DBIterator {
            raw: self.get_raw_iter_cf(cf_handle, readopts)?,
            direction: Direction::Forward, // blown away by set_mode()
            just_seeked: false,
        };
        rv.set_mode(mode);
        Ok(rv)
    }

    fn iterator_cf(
        &self,
        cf_handle: &ColumnFamily,
        mode: IteratorMode,
    ) -> Result<DBIterator, Error> {
        let opts = ReadOptions::default();
        self.get_iter_cf(cf_handle, &opts, mode)
    }

    fn full_iterator_cf(
        &self,
        cf_handle: &ColumnFamily,
        mode: IteratorMode,
    ) -> Result<DBIterator, Error> {
        let mut opts = ReadOptions::default();
        opts.set_total_order_seek(true);
        self.get_iter_cf(cf_handle, &opts, mode)
    }

    fn prefix_iterator_cf(
        &self,
        cf_handle: &ColumnFamily,
        prefix: &[u8],
    ) -> Result<DBIterator, Error> {
        let mut opts = ReadOptions::default();
        opts.set_prefix_same_as_start(true);
        self.get_iter_cf(
            cf_handle,
            &opts,
            IteratorMode::From(prefix, Direction::Forward),
        )
    }

    fn raw_iterator_cf(&self, cf_handle: &ColumnFamily) -> Result<DBRawIterator, Error> {
        let opts = ReadOptions::default();
        self.get_raw_iter_cf(cf_handle, &opts)
    }
}
