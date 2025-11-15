/// FFI (Foreign Function Interface) for C-compatible bindings
/// This module provides C-compatible interfaces for language bindings

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Opaque handle to a Kizuna instance
#[repr(C)]
pub struct KizunaHandle {
    _private: [u8; 0],
}

/// FFI result type
#[repr(C)]
pub struct FFIResult {
    /// Success flag
    pub success: bool,
    
    /// Error message (null if success)
    pub error: *mut c_char,
}

impl FFIResult {
    /// Creates a success result
    pub fn success() -> Self {
        Self {
            success: true,
            error: std::ptr::null_mut(),
        }
    }
    
    /// Creates an error result
    pub fn error(message: &str) -> Self {
        let error = CString::new(message).unwrap_or_default();
        Self {
            success: false,
            error: error.into_raw(),
        }
    }
}

/// Frees an FFI result
#[unsafe(no_mangle)]
pub extern "C" fn kizuna_free_result(result: FFIResult) {
    if !result.error.is_null() {
        unsafe {
            let _ = CString::from_raw(result.error);
        }
    }
}

/// Helper function to convert C string to Rust string
pub unsafe fn c_str_to_string(c_str: *const c_char) -> Result<String, std::str::Utf8Error> {
    if c_str.is_null() {
        return Ok(String::new());
    }
    
    let c_str = CStr::from_ptr(c_str);
    Ok(c_str.to_str()?.to_string())
}

/// Helper function to convert Rust string to C string
pub fn string_to_c_str(s: &str) -> *mut c_char {
    CString::new(s).unwrap_or_default().into_raw()
}
