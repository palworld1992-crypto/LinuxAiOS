use super::error::LibvirtError;
use std::panic::{catch_unwind, UnwindSafe};

/// Run a closure that may call into FFI and convert a panic into a `LibvirtError::OperationFailed`.
/// This prevents Rust panics from unwinding across FFI boundaries as required by the design doc.
pub fn catch_ffi<T, F>(f: F) -> Result<T, LibvirtError>
where
    F: FnOnce() -> T + UnwindSafe,
{
    catch_unwind(f).map_err(|_| LibvirtError::OperationFailed("FFI call panicked".to_string()))
}
