//! FFI bridge to Swift ScreenCaptureKit implementation.
//!
//! Provides C-compatible exports from a Swift static library,
//! enabling Rust code to start/stop ScreenCaptureKit recordings.
//!
//! The Swift companion library (swift-sck/) must be compiled and linked
//! into the final binary or framework.

use crate::error::{RecorderError, RecorderResult};
use std::ffi::CString;
use std::os::raw::c_int;

extern "C" {
    /// Start a screen recording for the specified app.
    ///
    /// # Arguments
    ///
    /// * `app_bundle_id_cstr` — C string bundle ID (e.g., "com.kooshapari.hwLedger")
    /// * `output_path_cstr` — C string path to output MP4
    /// * `width` — output width in pixels
    /// * `height` — output height in pixels
    /// * `fps` — frames per second
    ///
    /// # Returns
    ///
    /// 0 on success, non-zero error code on failure.
    fn hwledger_sck_start_recording(
        app_bundle_id_cstr: *const u8,
        output_path_cstr: *const u8,
        width: u32,
        height: u32,
        fps: u32,
    ) -> c_int;

    /// Stop the active screen recording.
    ///
    /// # Returns
    ///
    /// 0 on success, non-zero error code on failure.
    fn hwledger_sck_stop_recording() -> c_int;

    /// Check if Screen Recording permission is granted.
    ///
    /// Returns 1 if permission is granted, 0 if denied.
    fn hwledger_sck_has_permission() -> c_int;
}

/// Check if Screen Recording permission is granted (TCC).
///
/// # Errors
///
/// Returns an error if the permission check fails (e.g., system error).
///
/// # Safety
///
/// This calls a C function exported from the Swift ScreenCaptureKit bridge.
/// The function is thread-safe and idempotent.
pub fn check_screen_capture_permission() -> RecorderResult<bool> {
    // SAFETY: hwledger_sck_has_permission() is a pure C function from the Swift bridge.
    // It has no preconditions and returns a stable result.
    unsafe {
        let result = hwledger_sck_has_permission();
        match result {
            1 => Ok(true),
            0 => Ok(false),
            _ => Err(RecorderError::StreamConfigurationError(format!(
                "permission check failed with code: {}",
                result
            ))),
        }
    }
}

/// Start recording via ScreenCaptureKit.
///
/// # Safety
///
/// This calls a C function with valid C strings. The string pointers must remain valid
/// for the duration of the function call, which is guaranteed by the scope.
pub fn start_recording(
    app_bundle_id: &str,
    output_path: &str,
    width: u32,
    height: u32,
    fps: u32,
) -> RecorderResult<()> {
    let bundle_id_cstring = CString::new(app_bundle_id)
        .map_err(|_| RecorderError::InvalidOutputPath("invalid bundle ID".to_string()))?;

    let output_path_cstring = CString::new(output_path)
        .map_err(|_| RecorderError::InvalidOutputPath("invalid output path".to_string()))?;

    // SAFETY: Both CString objects are valid for the duration of this call.
    // The FFI function is idempotent and thread-safe per the Swift implementation.
    unsafe {
        let result = hwledger_sck_start_recording(
            bundle_id_cstring.as_ptr() as *const u8,
            output_path_cstring.as_ptr() as *const u8,
            width,
            height,
            fps,
        );

        if result == 0 {
            Ok(())
        } else {
            Err(RecorderError::StreamConfigurationError(format!(
                "SCK start failed with code: {}",
                result
            )))
        }
    }
}

/// Stop recording via ScreenCaptureKit.
///
/// # Safety
///
/// This calls a C function with no parameters. It is idempotent and thread-safe.
pub fn stop_recording() -> RecorderResult<()> {
    // SAFETY: hwledger_sck_stop_recording() is a pure C function with no preconditions.
    unsafe {
        let result = hwledger_sck_stop_recording();

        if result == 0 {
            Ok(())
        } else {
            Err(RecorderError::StreamConfigurationError(format!(
                "SCK stop failed with code: {}",
                result
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cstring_conversion() {
        let bundle_id = "com.kooshapari.hwLedger";
        let cstring = CString::new(bundle_id).unwrap();
        assert!(!cstring.as_ptr().cast::<u8>().is_null());
    }
}
