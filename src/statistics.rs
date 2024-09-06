use crate::ffi;

#[derive(Debug, Clone)]
pub struct NameParseError;
impl core::fmt::Display for NameParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unrecognized name")
    }
}

impl std::error::Error for NameParseError {}

// Helper macro to generate iterable nums that translate into static strings mapped from the cpp
// land.
macro_rules! iterable_named_enum {
    (
    $(#[$m:meta])*
    $type_vis:vis enum $typename:ident {
        $(
            $(#[$variant_meta:meta])*
            $variant:ident($variant_str:literal) $(= $value:expr)?,
        )+
    }
    ) => {
        // Main Type
        #[allow(clippy::all)]
        $(#[$m])*
        $type_vis enum $typename {
            $(
            $(#[$variant_meta])*
            $variant$( = $value)?,
            )+
        }

        #[automatically_derived]
        impl $typename {
            #[doc = "The corresponding rocksdb string identifier for this variant"]
            pub const fn name(&self) -> &'static str {
                match self {
                    $(
                        $typename::$variant => $variant_str,
                    )+
                }
            }
            pub fn iter() -> ::core::slice::Iter<'static, $typename> {
                static VARIANTS: &'static [$typename] = &[
                    $(
                        $typename::$variant,
                    )+
                ];
                VARIANTS.iter()
            }
        }


        #[automatically_derived]
        impl ::core::str::FromStr for $typename {
            type Err = NameParseError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $(
                        $variant_str => Ok($typename::$variant),
                    )+
                    _ => Err(NameParseError),
                }
            }
        }

        #[automatically_derived]
        impl ::core::fmt::Display for $typename {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                self.name().fmt(f)
            }
        }
    };
}

/// StatsLevel can be used to reduce statistics overhead by skipping certain
/// types of stats in the stats collection process.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum StatsLevel {
    /// Disable all metrics
    DisableAll = 0,
    /// Disable timer stats, and skip histogram stats
    ExceptHistogramOrTimers = 2,
    /// Skip timer stats
    ExceptTimers,
    /// Collect all stats except time inside mutex lock AND time spent on
    /// compression.
    ExceptDetailedTimers,
    /// Collect all stats except the counters requiring to get time inside the
    /// mutex lock.
    ExceptTimeForMutex,
    /// Collect all stats, including measuring duration of mutex operations.
    /// If getting time is expensive on the platform to run, it can
    /// reduce scalability to more threads, especially for writes.
    All,
}

include!("statistics_enum_ticker.rs");
include!("statistics_enum_histogram.rs");

pub struct HistogramData {
    pub(crate) inner: *mut ffi::rocksdb_statistics_histogram_data_t,
}

impl HistogramData {
    pub fn new() -> HistogramData {
        HistogramData::default()
    }
    pub fn median(&self) -> f64 {
        unsafe { ffi::rocksdb_statistics_histogram_data_get_median(self.inner) }
    }
    pub fn average(&self) -> f64 {
        unsafe { ffi::rocksdb_statistics_histogram_data_get_average(self.inner) }
    }
    pub fn p95(&self) -> f64 {
        unsafe { ffi::rocksdb_statistics_histogram_data_get_p95(self.inner) }
    }
    pub fn p99(&self) -> f64 {
        unsafe { ffi::rocksdb_statistics_histogram_data_get_p99(self.inner) }
    }
    pub fn max(&self) -> f64 {
        unsafe { ffi::rocksdb_statistics_histogram_data_get_max(self.inner) }
    }
    pub fn min(&self) -> f64 {
        unsafe { ffi::rocksdb_statistics_histogram_data_get_min(self.inner) }
    }
    pub fn sum(&self) -> u64 {
        unsafe { ffi::rocksdb_statistics_histogram_data_get_sum(self.inner) }
    }
    pub fn count(&self) -> u64 {
        unsafe { ffi::rocksdb_statistics_histogram_data_get_count(self.inner) }
    }
    pub fn std_dev(&self) -> f64 {
        unsafe { ffi::rocksdb_statistics_histogram_data_get_std_dev(self.inner) }
    }
}

impl Default for HistogramData {
    fn default() -> Self {
        let histogram_data_inner = unsafe { ffi::rocksdb_statistics_histogram_data_create() };
        assert!(
            !histogram_data_inner.is_null(),
            "Could not create RocksDB histogram data"
        );

        Self {
            inner: histogram_data_inner,
        }
    }
}

impl Drop for HistogramData {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_statistics_histogram_data_destroy(self.inner);
        }
    }
}

#[test]
fn sanity_checks() {
    let want = "rocksdb.async.read.bytes";
    assert_eq!(want, Histogram::AsyncReadBytes.name());

    let want = "rocksdb.block.cache.index.miss";
    assert_eq!(want, Ticker::BlockCacheIndexMiss.to_string());

    // assert enum lengths
    assert_eq!(Ticker::iter().count(), 211 /* TICKER_ENUM_MAX */);
    assert_eq!(Histogram::iter().count(), 62 /* HISTOGRAM_ENUM_MAX */);
}
