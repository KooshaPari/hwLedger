// One-shot stub generator for pending-record journey manifests.
// Per scripting policy (Rust first): this replaces a shell loop.
// Compile: `rustc /tmp/gen_pending.rs -o /tmp/gen_pending && /tmp/gen_pending`
use std::fs;
use std::path::PathBuf;

fn main() {
    // (fr_id, kind) pairs that lack a matching verified manifest.
    let entries: &[(&str, &str)] = &[
        ("FR-PLAN-001", "cli"),
        ("FR-PLAN-004", "gui"),
        ("FR-PLAN-005", "gui"),
        ("FR-PLAN-006", "gui"),
        ("FR-PLAN-007", "cli"),
        ("FR-TEL-001", "cli"),
        ("FR-TEL-002", "cli"),
        ("FR-TEL-003", "gui"),
        ("FR-TEL-004", "cli"),
        ("FR-INF-005", "gui"),
        ("FR-FLEET-002", "cli"),
        ("FR-FLEET-003", "cli"),
        ("FR-FLEET-004", "cli"),
        ("FR-FLEET-005", "cli"),
        ("FR-FLEET-006", "cli"),
        ("FR-FLEET-007", "gui"),
        ("FR-FLEET-008", "cli"),
        ("FR-UI-002", "gui"),
        ("FR-UI-004", "gui"),
        ("FR-TRACE-001", "cli"),
        ("FR-TRACE-002", "cli"),
        ("FR-TRACE-003", "cli"),
        ("FR-TRACE-004", "cli"),
    ];
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
