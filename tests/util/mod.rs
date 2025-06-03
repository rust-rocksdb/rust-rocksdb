#![allow(dead_code)]

use std::{
    cmp::Ordering,
    convert::TryInto,
    path::{Path, PathBuf},
};

use rocksdb::{Error, Options, DB};

/// Temporary database path which calls DB::Destroy when DBPath is dropped.
pub struct DBPath {
    dir: tempfile::TempDir, // kept for cleaning up during drop
    path: PathBuf,
}

impl DBPath {
    /// Produces a fresh (non-existent) temporary path which will be DB::destroy'ed automatically.
    pub fn new(prefix: &str) -> DBPath {
        let dir = tempfile::Builder::new()
            .prefix(prefix)
            .tempdir()
            .expect("Failed to create temporary path for db.");
        let path = dir.path().join("db");

        DBPath { dir, path }
    }
}

impl Drop for DBPath {
    fn drop(&mut self) {
        let opts = Options::default();
        DB::destroy(&opts, &self.path).expect("Failed to destroy temporary DB");
    }
}

/// Convert a DBPath ref to a Path ref.
/// We don't implement this for DBPath values because we want them to
/// exist until the end of their scope, not get passed into functions and
/// dropped early.
impl AsRef<Path> for &DBPath {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

type Pair = (Box<[u8]>, Box<[u8]>);

pub fn pair(left: &[u8], right: &[u8]) -> Pair {
    (Box::from(left), Box::from(right))
}

#[track_caller]
pub fn assert_iter(iter: impl Iterator<Item = Result<Pair, Error>>, want: &[Pair]) {
    let got = iter.collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(got.as_slice(), want);
}

#[track_caller]
pub fn assert_iter_reversed(iter: impl Iterator<Item = Result<Pair, Error>>, want: &[Pair]) {
    let mut got = iter.collect::<Result<Vec<_>, _>>().unwrap();
    got.reverse();
    assert_eq!(got.as_slice(), want);
}

/// A timestamp type we use in testing [user-defined timestamp](https://github.com/facebook/rocksdb/wiki/User-defined-Timestamp).
/// This is a `u64` in little endian encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct U64Timestamp([u8; Self::SIZE]);

impl U64Timestamp {
    pub const SIZE: usize = 8;

    pub fn new(ts: u64) -> Self {
        Self(ts.to_le_bytes())
    }
}

impl From<&[u8]> for U64Timestamp {
    fn from(slice: &[u8]) -> Self {
        assert_eq!(
            slice.len(),
            Self::SIZE,
            "incorrect timestamp length: {}, should be {}",
            slice.len(),
            Self::SIZE
        );
        Self(slice.try_into().unwrap())
    }
}

impl From<U64Timestamp> for Vec<u8> {
    fn from(ts: U64Timestamp) -> Self {
        ts.0.to_vec()
    }
}

impl AsRef<[u8]> for U64Timestamp {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl PartialOrd for U64Timestamp {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for U64Timestamp {
    fn cmp(&self, other: &Self) -> Ordering {
        let lhs = u64::from_le_bytes(self.0);
        let rhs = u64::from_le_bytes(other.0);
        lhs.cmp(&rhs)
    }
}

/// A comparator for use in column families with [user-defined timestamp](https://github.com/facebook/rocksdb/wiki/User-defined-Timestamp)
/// enabled. This comparator assumes `u64` timestamp in little endian encoding.
/// This is the same behavior as RocksDB's built-in comparator.
///
/// Adapted from C++ and Golang implementations from:
/// - [rocksdb](https://github.com/facebook/rocksdb/blob/v9.4.0/test_util/testutil.cc#L112)
/// - [gorocksdb](https://github.com/linxGnu/grocksdb/blob/v1.9.2/db_ts_test.go#L167)
/// - [SeiDB](https://github.com/sei-protocol/sei-db/blob/v0.0.41/ss/rocksdb/comparator.go)
pub struct U64Comparator;

impl U64Comparator {
    pub const NAME: &'static str = "rust-rocksdb.U64Comparator";

    pub fn compare(a: &[u8], b: &[u8]) -> Ordering {
        // First, compare the keys without timestamps. If the keys are different,
        // then we don't have to consider the timestamps at all.
        let ord = Self::compare_without_ts(a, true, b, true);
        if ord != Ordering::Equal {
            return ord;
        }

        // The keys are the same, so now we compare the timestamps.
        // The larger (i.e. newer) key should come first, hence the `reverse`.
        Self::compare_ts(
            extract_timestamp_from_user_key(a),
            extract_timestamp_from_user_key(b),
        )
        .reverse()
    }

    pub fn compare_ts(bz1: &[u8], bz2: &[u8]) -> Ordering {
        let ts1 = U64Timestamp::from(bz1);
        let ts2 = U64Timestamp::from(bz2);
        ts1.cmp(&ts2)
    }

    pub fn compare_without_ts(
        mut a: &[u8],
        a_has_ts: bool,
        mut b: &[u8],
        b_has_ts: bool,
    ) -> Ordering {
        if a_has_ts {
            a = strip_timestamp_from_user_key(a);
        }
        if b_has_ts {
            b = strip_timestamp_from_user_key(b);
        }
        a.cmp(b)
    }
}

fn extract_timestamp_from_user_key(key: &[u8]) -> &[u8] {
    &key[(key.len() - U64Timestamp::SIZE)..]
}

fn strip_timestamp_from_user_key(key: &[u8]) -> &[u8] {
    &key[..(key.len() - U64Timestamp::SIZE)]
}
