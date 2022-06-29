use std::path::{Path, PathBuf};

use rocksdb::{Options, DB};

/// Temporary database path which calls DB::Destroy when DBPath is dropped.
pub struct DBPath {
    #[allow(dead_code)]
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
/// exist until the end of their scope, not get passed in to functions and
/// dropped early.
impl AsRef<Path> for &DBPath {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

#[allow(dead_code)]
pub fn pair(left: &[u8], right: &[u8]) -> (Box<[u8]>, Box<[u8]>) {
    (Box::from(left), Box::from(right))
}

// Use macro rather than functions so that on failure we get line number where
// the macro was instantiated rather than line number inside of a function.
#[allow(unused_macros)]
macro_rules! assert_iter {
    ($iter:expr, $want:expr) => {
        let got = $iter.collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(got.as_slice(), &$want);
    };
    (reversed $iter:expr, $want:expr) => {
        let mut got = $iter.collect::<Result<Vec<_>, _>>().unwrap();
        got.reverse();
        assert_eq!(got.as_slice(), &$want);
    };
}

#[allow(unused_imports)]
pub(crate) use assert_iter;
