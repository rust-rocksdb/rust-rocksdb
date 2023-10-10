/// A range which can be set as iterate bounds on [`crate::ReadOptions`].
///
/// See [`crate::ReadOptions::set_iterate_range`] for documentation and
/// examples.
pub trait IterateBounds {
    /// Converts object into lower and upper bounds pair.
    ///
    /// If this object represents range with one of the bounds unset,
    /// corresponding element is returned as `None`.  For example, `..upper`
    /// range would be converted into `(None, Some(upper))` pair.
    fn into_bounds(self) -> (Option<Vec<u8>>, Option<Vec<u8>>);
}

impl IterateBounds for std::ops::RangeFull {
    fn into_bounds(self) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
        (None, None)
    }
}

impl<K: Into<Vec<u8>>> IterateBounds for std::ops::Range<K> {
    fn into_bounds(self) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
        (Some(self.start.into()), Some(self.end.into()))
    }
}

impl<K: Into<Vec<u8>>> IterateBounds for std::ops::RangeFrom<K> {
    fn into_bounds(self) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
        (Some(self.start.into()), None)
    }
}

impl<K: Into<Vec<u8>>> IterateBounds for std::ops::RangeTo<K> {
    fn into_bounds(self) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
        (None, Some(self.end.into()))
    }
}

/// Representation of a range of keys starting with given prefix.
///
/// Can be used as argument of [`crate::ReadOptions::set_iterate_range`] method
/// to set iterate bounds.
#[derive(Clone, Copy)]
pub struct PrefixRange<K>(pub K);

impl<K: Into<Vec<u8>>> IterateBounds for PrefixRange<K> {
    /// Converts the prefix range representation into pair of bounds.
    ///
    /// The conversion assumes lexicographical sorting on `u8` values.  For
    /// example, `PrefixRange("a")` is equivalent to `"a".."b"` range.  Note
    /// that for some prefixes, either of the bounds may be `None`.  For
    /// example, an empty prefix is equivalent to a full range (i.e. both bounds
    /// being `None`).
    fn into_bounds(self) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
        let start = self.0.into();
        if start.is_empty() {
            (None, None)
        } else {
            let end = next_prefix(&start);
            (Some(start), end)
        }
    }
}

/// Returns lowest value following largest value with given prefix.
///
/// In other words, computes upper bound for a prefix scan over list of keys
/// sorted in lexicographical order.  This means that a prefix scan can be
/// expressed as range scan over a right-open `[prefix, next_prefix(prefix))`
/// range.
///
/// For example, for prefix `foo` the function returns `fop`.
///
/// Returns `None` if there is no value which can follow value with given
/// prefix.  This happens when prefix consists entirely of `'\xff'` bytes (or is
/// empty).
fn next_prefix(prefix: &[u8]) -> Option<Vec<u8>> {
    let ffs = prefix
        .iter()
        .rev()
        .take_while(|&&byte| byte == u8::MAX)
        .count();
    let next = &prefix[..(prefix.len() - ffs)];
    if next.is_empty() {
        // Prefix consisted of \xff bytes.  There is no prefix that
        // follows it.
        None
    } else {
        let mut next = next.to_vec();
        *next.last_mut().unwrap() += 1;
        Some(next)
    }
}

#[test]
fn test_prefix_range() {
    fn test(start: &[u8], end: Option<&[u8]>) {
        let got = PrefixRange(start).into_bounds();
        assert_eq!((Some(start), end), (got.0.as_deref(), got.1.as_deref()));
    }

    let empty: &[u8] = &[];
    assert_eq!((None, None), PrefixRange(empty).into_bounds());
    test(b"\xff", None);
    test(b"\xff\xff\xff\xff", None);
    test(b"a", Some(b"b"));
    test(b"a\xff\xff\xff", Some(b"b"));
}
