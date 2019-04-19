use super::{Get, GetCF, Iterate, IterateCF, Read};
use crate::{
    db_iterator::{DBIterator, DBRawIterator, IteratorMode},
    db_vector::DBVector,
    handle::ConstHandle,
    ColumnFamily, Direction, Error, ReadOptions, ReadOptionsFactory, Snapshot,
};

pub trait SnapshotIterate {
    fn get_raw_iter(&self, readopts: &ReadOptionsFactory) -> DBRawIterator;

    fn get_iter(&self, readopts: &ReadOptionsFactory, mode: IteratorMode) -> DBIterator {
        let mut rv = DBIterator {
            raw: self.get_raw_iter(readopts),
            direction: Direction::Forward, // blown away by set_mode()
            just_seeked: false,
        };
        rv.set_mode(mode);
        rv
    }

    fn iterator_opt(&self, mode: IteratorMode, readopts: &ReadOptionsFactory) -> DBIterator {
        self.get_iter(readopts, mode)
    }

    fn iterator(&self, mode: IteratorMode) -> DBIterator {
        let readopts = ReadOptionsFactory::default();
        self.iterator_opt(mode, &readopts)
    }

    fn full_iterator(&self, mode: IteratorMode) -> DBIterator {
        let opts = ReadOptionsFactory::default().set_total_order_seek(true);
        self.get_iter(&opts, mode)
    }

    fn prefix_iterator(&self, prefix: &[u8]) -> DBIterator {
        let opts = ReadOptionsFactory::default().set_prefix_same_as_start(true);
        self.get_iter(&opts, IteratorMode::From(prefix, Direction::Forward))
    }

    fn raw_iterator(&self) -> DBRawIterator {
        let opts = ReadOptionsFactory::default();
        self.get_raw_iter(&opts)
    }
}

pub trait SnapshotIterateCF {
    fn get_raw_iter_cf(
        &self,
        cf_handle: &ColumnFamily,
        readopts: &ReadOptionsFactory,
    ) -> Result<DBRawIterator, Error>;

    fn get_iter_cf(
        &self,
        cf_handle: &ColumnFamily,
        readopts: &ReadOptionsFactory,
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
        let opts = ReadOptionsFactory::default();
        self.get_iter_cf(cf_handle, &opts, mode)
    }

    fn full_iterator_cf(
        &self,
        cf_handle: &ColumnFamily,
        mode: IteratorMode,
    ) -> Result<DBIterator, Error> {
        let opts = ReadOptionsFactory::default().set_total_order_seek(true);
        self.get_iter_cf(cf_handle, &opts, mode)
    }

    fn prefix_iterator_cf(
        &self,
        cf_handle: &ColumnFamily,
        prefix: &[u8],
    ) -> Result<DBIterator, Error> {
        let opts = ReadOptionsFactory::default().set_prefix_same_as_start(true);
        self.get_iter_cf(
            cf_handle,
            &opts,
            IteratorMode::From(prefix, Direction::Forward),
        )
    }

    fn raw_iterator_cf(&self, cf_handle: &ColumnFamily) -> Result<DBRawIterator, Error> {
        let opts = ReadOptionsFactory::default();
        self.get_raw_iter_cf(cf_handle, &opts)
    }
}
