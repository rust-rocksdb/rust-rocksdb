/// Provides access to underlying handles for 
/// database operations
pub trait Handle<T> {
    fn handle(&self) -> *mut T;
}