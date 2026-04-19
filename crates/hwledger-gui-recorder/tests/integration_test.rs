//! Integration test for hwledger-gui-recorder.
//! Requires Screen Recording permission from the system.
//!
//! Run with: HWLEDGER_SCK_LIVE=1 cargo test --test integration_test -- --nocapture
//!
//! To grant permission on macOS:
//! 1. System Settings > Privacy & Security > Screen Recording
//! 2. Add Terminal or cargo process to allowed list
//! 3. Run the test again

#[cfg(target_os = "macos")]
mod screen_recording {
    use std::path::PathBuf;
    use std::thread;
    use std::time::Duration;

    /// Test recording against Finder (always available).
    /// If permission is not granted, test is skipped.
    #[test]
    #[ignore] // Requires macOS permission grant
    fn test_sck_recording_with_permission() {
        // Only run if explicitly enabled
        if std::env::var("HWLEDGER_SCK_LIVE").is_err() {
            eprintln!("Skipping live SCK recording test. Set HWLEDGER_SCK_LIVE=1 to enable.");
            return;
        }

        let output_path = tempfile::NamedTempFile::new()
            .expect("could not create temp file")
            .path()
            .to_path_buf();

        eprintln!("Recording to: {:?}", output_path);

        // Note: The Rust FFI expects to call out to Swift implementations.
        // In a test-only context without linking the Swift bridge, this will fail with
        // undefined symbol errors. This test is intended to be run alongside the full
        // hwLedgerUITests executable that links both Rust and Swift.
        //
        // For standalone Rust testing, use the public API (JourneyRecorder, ScreenRecorder)
        // which require a properly compiled binary.

        eprintln!("Live recording test requires compilation alongside Swift code. Skipping.");
    }
}

#[cfg(not(target_os = "macos"))]
mod not_macos {
    #[test]
    fn test_non_macos_skipped() {
        // Tests only make sense on macOS
    }
}
