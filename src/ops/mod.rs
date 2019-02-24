mod get;

pub use self::get::{Get, GetCF};

/// Marker trait for operations that leave DB
/// state unchanged
pub trait Read {}

/// Marker trait for operations that mutate
/// DB state
pub trait Write {}