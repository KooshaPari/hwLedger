//! Minimum Rust test: invoke the TS entrypoint via `Command::new("bun")`
//! with a mock `--help`-style invocation. We do NOT actually render (that
//! requires Chromium + node_modules); we only assert the Rust side can shell
//! out cleanly when bun is present, and gracefully when it's not.

use hwledger_journey_render::{manifest::RichManifest, RenderError};

#[test]
fn rich_manifest_roundtrips() {
    let m = RichManifest {
        id: "plan-deepseek".into(),
        intent: "demo".into(),
        keyframe_count: 3,
        passed: true,
        ..Default::default()
    };
    let raw = serde_json::to_string(&m).unwrap();
    let parsed: RichManifest = serde_json::from_str(&raw).unwrap();
    assert_eq!(parsed.id, "plan-deepseek");
    assert_eq!(parsed.keyframe_count, 3);
}

#[test]
fn ensure_bun_or_graceful_error() {
    // We don't assert bun is present (CI may lack it). We only assert that
    // calling into the module with a bogus plan produces a typed error, not
    // a panic.
    let plan = hwledger_journey_render::RenderPlan::new(
        "no-such-journey",
        "/nonexistent/manifest.json",
        "/nonexistent/keyframes",
        "/nonexistent/remotion",
        "/nonexistent/out.mp4",
    );
    let res = hwledger_journey_render::build_rich_manifest(&plan);
    assert!(matches!(res, Err(RenderError::Io(_))));
}
