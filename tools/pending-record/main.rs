// One-shot stub generator for pending-record journey manifests.
// Per scripting policy (Rust first): this replaces a shell loop.
// Compile: `rustc /tmp/gen_pending.rs -o /tmp/gen_pending && /tmp/gen_pending`
use std::fs;
use std::path::PathBuf;

fn main() {
    // (fr_id, kind) pairs that lack a matching verified manifest.
    //
    // All 23 original pending-record stubs have been retired (commit: see
    // "close remaining hwLedger items in one bundled pass"):
    //   * FR-PLAN-001, FR-TEL-001/-002, FR-FLEET-002/-003, FR-TRACE-001..004
    //     are now covered by real verified tapes (first-plan, probe-list,
    //     fleet-register, fleet-audit, traceability-report,
    //     traceability-strict).
    //   * The remaining 14 FRs were retagged `[journey_kind: none]` in
    //     PRD.md with one-line justifications (GUI/TCC-deferred or
    //     server-internal primitives covered by unit/integration tests).
    //
    // Leave this list empty so regenerating stubs is a no-op until a new
    // tagged FR is introduced without a matching tape.
    let entries: &[(&str, &str)] = &[];
    let repo: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    for (fr, kind) in entries {
        let root = match *kind {
            "cli" => "docs-site/public/cli-journeys/manifests",
            "gui" => "docs-site/public/gui-journeys",
            "web" => "docs-site/public/streamlit-journeys/manifests",
            other => panic!("unknown kind: {other}"),
        };
        let slug = format!("pending-{}-{}", kind, fr.to_ascii_lowercase());
        let dir = repo.join(root).join(&slug);
        fs::create_dir_all(&dir).expect("mkdir");

        // manifest.verified.json — passed=false, score=0 → gate surfaces
        // the FR without falsely passing. `traces_to` bookkeeps the FR so
        // the scanner matches it to this kind.
        let manifest = format!(
            r#"{{
  "id": "{slug}",
  "intent": "PENDING RECORD — blackbox tape not yet captured for {fr} ({kind}).",
  "keyframe_count": 0,
  "passed": false,
  "recording": null,
  "steps": [],
  "traces_to": ["{fr}"],
  "verification": {{
    "overall_score": 0.0,
    "all_intents_passed": false,
    "describe_confidence": 0.0,
    "judge_confidence": 0.0,
    "mode": "pending",
    "timestamp": "1970-01-01T00:00:00Z",
    "pending_reason": "Tape recording pending — created by pending-record stub generator to keep {fr} visible in the traceability gate."
  }}
}}
"#
        );
        fs::write(dir.join("manifest.verified.json"), manifest).expect("write verified");

        // Companion manifest.json so the VitePress build doesn't complain
        // about a verified-only directory. Same structure without the
        // verification envelope.
        let plain = format!(
            r#"{{
  "id": "{slug}",
  "intent": "PENDING RECORD — blackbox tape not yet captured for {fr} ({kind}).",
  "keyframe_count": 0,
  "passed": false,
  "recording": null,
  "steps": [],
  "traces_to": ["{fr}"]
}}
"#
        );
        fs::write(dir.join("manifest.json"), plain).expect("write plain");

        println!("stubbed {}", dir.display());
    }
}
