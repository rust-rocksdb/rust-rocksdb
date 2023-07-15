use crate::ffi_util::CStrLike;

use std::ffi::{CStr, CString};

/// A borrowed name of a RocksDB property.
///
/// The value is guaranteed to be a nul-terminated UTF-8 string. This means it
/// can be converted to [`CStr`] and [`str`] at zero cost.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct PropName(CStr);

impl PropName {
    /// Creates a new object from a nul-terminated string with no internal nul
    /// bytes.
    ///
    /// Panics if the `value` isn’t terminated by a nul byte or contains
    /// interior nul bytes.
    pub(crate) const fn new_unwrap(value: &str) -> &Self {
        let bytes = if let Some((&0, bytes)) = value.as_bytes().split_last() {
            bytes
        } else {
            panic!("input was not nul-terminated");
        };

        let mut idx = 0;
        while idx < bytes.len() {
            assert!(bytes[idx] != 0, "input contained interior nul byte");
            idx += 1;
        }

        // SAFETY: 1. We’ve just verified `value` is a nul-terminated with no
        // interior nul bytes and since its `str` it’s also valid UTF-8.
        // 2. Self and CStr have the same representation so casting is sound.
        unsafe {
            let value = CStr::from_bytes_with_nul_unchecked(value.as_bytes());
            &*(value as *const CStr as *const Self)
        }
    }

    /// Converts the value into a C string slice.
    #[inline]
    pub fn as_c_str(&self) -> &CStr {
        &self.0
    }

    /// Converts the value into a string slice.
    ///
    /// Nul byte terminating the underlying C string is not included in the
    /// returned slice.
    #[inline]
    pub fn as_str(&self) -> &str {
        // SAFETY: self.0 is guaranteed to be valid ASCII string.
        unsafe { std::str::from_utf8_unchecked(self.0.to_bytes()) }
    }
}

impl core::ops::Deref for PropName {
    type Target = CStr;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_c_str()
    }
}

impl core::convert::AsRef<CStr> for PropName {
    #[inline]
    fn as_ref(&self) -> &CStr {
        self.as_c_str()
    }
}

impl core::convert::AsRef<str> for PropName {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::borrow::ToOwned for PropName {
    type Owned = PropertyName;

    #[inline]
    fn to_owned(&self) -> Self::Owned {
        PropertyName(self.0.to_owned())
    }

    #[inline]
    fn clone_into(&self, target: &mut Self::Owned) {
        self.0.clone_into(&mut target.0);
    }
}

impl core::fmt::Display for PropName {
    #[inline]
    fn fmt(&self, fmtr: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.as_str().fmt(fmtr)
    }
}

impl core::fmt::Debug for PropName {
    #[inline]
    fn fmt(&self, fmtr: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.as_str().fmt(fmtr)
    }
}

impl core::cmp::PartialEq<CStr> for PropName {
    #[inline]
    fn eq(&self, other: &CStr) -> bool {
        self.as_c_str().eq(other)
    }
}

impl core::cmp::PartialEq<str> for PropName {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_str().eq(other)
    }
}

impl core::cmp::PartialEq<PropName> for CStr {
    #[inline]
    fn eq(&self, other: &PropName) -> bool {
        self.eq(other.as_c_str())
    }
}

impl core::cmp::PartialEq<PropName> for str {
    #[inline]
    fn eq(&self, other: &PropName) -> bool {
        self.eq(other.as_str())
    }
}

impl<'a> CStrLike for &'a PropName {
    type Baked = &'a CStr;
    type Error = std::convert::Infallible;

    #[inline]
    fn bake(self) -> Result<Self::Baked, Self::Error> {
        Ok(&self.0)
    }

    #[inline]
    fn into_c_string(self) -> Result<CString, Self::Error> {
        Ok(self.0.to_owned())
    }
}

/// An owned name of a RocksDB property.
///
/// The value is guaranteed to be a nul-terminated UTF-8 string. This means it
/// can be converted to [`CString`] and [`String`] at zero cost.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct PropertyName(CString);

impl PropertyName {
    /// Creates a new object from valid nul-terminated UTF-8 string. The string
    /// must not contain interior nul bytes.
    #[inline]
    unsafe fn from_vec_with_nul_unchecked(inner: Vec<u8>) -> Self {
        // SAFETY: Caller promises inner is nul-terminated and valid UTF-8.
        Self(CString::from_vec_with_nul_unchecked(inner))
    }

    /// Converts the value into a C string.
    #[inline]
    pub fn into_c_string(self) -> CString {
        self.0
    }

    /// Converts the property name into a string.
    ///
    /// Nul byte terminating the underlying C string is not included in the
    /// returned value.
    #[inline]
    pub fn into_string(self) -> String {
        // SAFETY: self.0 is guaranteed to be valid UTF-8.
        unsafe { String::from_utf8_unchecked(self.0.into_bytes()) }
    }
}

impl std::ops::Deref for PropertyName {
    type Target = PropName;

    #[inline]
    fn deref(&self) -> &Self::Target {
        // SAFETY: 1. PropName and CStr have the same representation so casting
        // is safe. 2. self.0 is guaranteed to be valid nul-terminated UTF-8
        // string.
        unsafe { &*(self.0.as_c_str() as *const CStr as *const PropName) }
    }
}

impl core::convert::AsRef<CStr> for PropertyName {
    #[inline]
    fn as_ref(&self) -> &CStr {
        self.as_c_str()
    }
}

impl core::convert::AsRef<str> for PropertyName {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::borrow::Borrow<PropName> for PropertyName {
    #[inline]
    fn borrow(&self) -> &PropName {
        self
    }
}

impl core::fmt::Display for PropertyName {
    #[inline]
    fn fmt(&self, fmtr: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.as_str().fmt(fmtr)
    }
}

impl core::fmt::Debug for PropertyName {
    #[inline]
    fn fmt(&self, fmtr: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.as_str().fmt(fmtr)
    }
}

impl core::cmp::PartialEq<CString> for PropertyName {
    #[inline]
    fn eq(&self, other: &CString) -> bool {
        self.as_c_str().eq(other.as_c_str())
    }
}

impl core::cmp::PartialEq<String> for PropertyName {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl core::cmp::PartialEq<PropertyName> for CString {
    #[inline]
    fn eq(&self, other: &PropertyName) -> bool {
        self.as_c_str().eq(other.as_c_str())
    }
}

impl core::cmp::PartialEq<PropertyName> for String {
    #[inline]
    fn eq(&self, other: &PropertyName) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl CStrLike for PropertyName {
    type Baked = CString;
    type Error = std::convert::Infallible;

    #[inline]
    fn bake(self) -> Result<Self::Baked, Self::Error> {
        Ok(self.0)
    }

    #[inline]
    fn into_c_string(self) -> Result<CString, Self::Error> {
        Ok(self.0)
    }
}

impl<'a> CStrLike for &'a PropertyName {
    type Baked = &'a CStr;
    type Error = std::convert::Infallible;

    #[inline]
    fn bake(self) -> Result<Self::Baked, Self::Error> {
        Ok(self.as_c_str())
    }

    #[inline]
    fn into_c_string(self) -> Result<CString, Self::Error> {
        Ok(self.0.clone())
    }
}

/// Constructs a property name for an ‘at level’ property.
///
/// `name` is the infix of the property name (e.g. `"num-files-at-level"`) and
/// `level` is level to get statistics of. The property name is constructed as
/// `"rocksdb.<name><level>"`.
///
/// Expects `name` not to contain any interior nul bytes.
pub(crate) unsafe fn level_property(name: &str, level: usize) -> PropertyName {
    let bytes = format!("rocksdb.{name}{level}\0").into_bytes();
    // SAFETY: We’re appending terminating nul and caller promises `name` has no
    // interior nul bytes.
    PropertyName::from_vec_with_nul_unchecked(bytes)
}

#[test]
fn sanity_checks() {
    let want = "rocksdb.cfstats-no-file-histogram";
    assert_eq!(want, crate::properties::CFSTATS_NO_FILE_HISTOGRAM);

    let want = "rocksdb.num-files-at-level5";
    assert_eq!(want, &*crate::properties::num_files_at_level(5));
}

#[test]
#[should_panic]
fn test_interior_nul() {
    PropName::new_unwrap("interior nul\0\0");
}

#[test]
#[should_panic]
fn test_non_nul_terminated() {
    PropName::new_unwrap("no nul terminator");
}
