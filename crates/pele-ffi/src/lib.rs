//! Opaque, immutable owned results. Unsafe code is confined to the C adapter.
use pele::Error;
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    slice,
};

pub struct PeleResult {
    status: u32,
    data: Vec<u8>,
}
fn error(code: &str, message: &str) -> PeleResult {
    let e = Error {
        code: code.into(),
        message: message.into(),
        span: None,
    };
    PeleResult {
        status: e.status(),
        data: pele::error_envelope(&e).into_bytes(),
    }
}
#[no_mangle]
pub extern "C" fn pele_abi_version() -> u32 {
    1
}

/// # Safety
/// Each non-null pointer must name `len` readable bytes for this call.
#[no_mangle]
pub unsafe extern "C" fn pele_render(
    source: *const u8,
    source_len: usize,
    options: *const u8,
    options_len: usize,
) -> *mut PeleResult {
    let result = catch_unwind(AssertUnwindSafe(|| {
        if source_len > pele::MAX_SOURCE || options_len > pele::MAX_OPTIONS {
            return error("input_limit", "Declared input length exceeds limit");
        }
        if (source.is_null() && source_len != 0) || (options.is_null() && options_len != 0) {
            return error("invalid_utf8", "Null input with nonzero length");
        }
        let s = if source_len == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(source, source_len) }
        };
        let o = if options_len == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(options, options_len) }
        };
        let (Ok(s), Ok(o)) = (std::str::from_utf8(s), std::str::from_utf8(o)) else {
            return error("invalid_utf8", "Input must be UTF-8");
        };
        let (status, data) = pele::render_envelope(s, o);
        PeleResult {
            status,
            data: data.into_bytes(),
        }
    }))
    .unwrap_or_else(|_| error("internal", "Unexpected engine failure"));
    Box::into_raw(Box::new(result))
}
/// # Safety
/// Non-null result must be a live handle returned by pele_render.
#[no_mangle]
pub unsafe extern "C" fn pele_result_status(result: *const PeleResult) -> u32 {
    unsafe { result.as_ref() }.map_or(3, |r| r.status)
}
/// # Safety
/// Non-null result must be a live handle. Bytes are borrowed until free.
#[no_mangle]
pub unsafe extern "C" fn pele_result_data(result: *const PeleResult) -> *const u8 {
    unsafe { result.as_ref() }.map_or(std::ptr::null(), |r| r.data.as_ptr())
}
/// # Safety
/// Non-null result must be a live handle.
#[no_mangle]
pub unsafe extern "C" fn pele_result_len(result: *const PeleResult) -> usize {
    unsafe { result.as_ref() }.map_or(0, |r| r.data.len())
}
/// # Safety
/// Result must be null or a live result; free exactly once, after readers finish.
#[no_mangle]
pub unsafe extern "C" fn pele_result_free(result: *mut PeleResult) {
    if !result.is_null() {
        drop(unsafe { Box::from_raw(result) });
    }
}
