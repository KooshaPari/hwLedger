//! Integration test: spawn real PlayCua, record a Finder window for 3s,
//! assert the MP4 exists and is non-empty. `#[ignore]` by default — run
//! manually with `cargo test -p hwledger-journey-record -- --ignored`.
//!
//! Traces to: ADR 0035 §Tests ("integration: actual 3s recording of a
//! Finder window on macOS via spawned PlayCua").

use std::path::PathBuf;

use hwledger_journey_record::{run_record, PlayCuaBinary, RecordTarget};

#[tokio::test]
#[ignore = "requires a Finder window and Screen Recording TCC grant; run with --ignored"]
async fn record_finder_window_3s() {
    let out = std::env::temp_dir().join("hwledger-journey-record-finder.mp4");
    if out.exists() {
        let _ = std::fs::remove_file(&out);
    }
    let target = RecordTarget::parse("window:Finder").unwrap();
    let bin = PlayCuaBinary::locate().expect("locate PlayCua");
    let outcome = run_record(bin, &target, &out, 3, &[]).await.expect("record");
    assert_eq!(outcome.duration_secs, 3);
    let meta = std::fs::metadata(&outcome.out_path)
        .unwrap_or_else(|e| panic!("MP4 missing at {}: {e}", outcome.out_path.display()));
    assert!(meta.len() > 0, "MP4 is zero bytes: {}", outcome.out_path.display());
    let _expected: PathBuf = out;
}
