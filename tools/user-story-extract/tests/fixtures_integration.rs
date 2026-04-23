//! Integration: harvesting the repo's `tests/fixtures/user-story/` yields
//! exactly 4 stories (one per language).

use std::collections::BTreeSet;
use std::path::PathBuf;
use user_story_extract::{check_coverage, check_duplicate_ids, extract_paths, parse_fr_list};

fn repo_root() -> PathBuf {
    // `tools/user-story-extract` -> up two = repo root.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().unwrap().parent().unwrap().to_path_buf()
}

#[test]
fn harvests_four_fixture_stories() {
    let root = repo_root().join("tests/fixtures/user-story");
    let (stories, errors) = extract_paths(&[root]);
    assert!(errors.is_empty(), "unexpected harvest errors: {errors:?}");
    assert_eq!(stories.len(), 4, "expected 4 fixture stories, got {}: {:#?}", stories.len(), stories.iter().map(|s| &s.journey_id).collect::<Vec<_>>());

    let ids: BTreeSet<_> = stories.iter().map(|s| s.journey_id.clone()).collect();
    assert!(ids.contains("fixture-rust-first-plan"));
    assert!(ids.contains("fixture-swift-export-gui"));
    assert!(ids.contains("fixture-playwright-planner"));
    assert!(ids.contains("fixture-k6-fleet-probe"));

    // Duplicate check passes.
    assert!(check_duplicate_ids(&stories).is_empty());

    // Coverage against a known FR set.
    let known_md = "FR-PLAN-001 FR-PLAN-002 FR-PLAN-003 FR-PLAN-004 FR-UI-001 FR-UI-002 FR-TEL-001 FR-FLEET-002";
    let known = parse_fr_list(known_md);
    let missing = check_coverage(&stories, &known);
    assert!(missing.is_empty(), "unexpected missing FRs: {missing:?}");
}

#[test]
fn coverage_flags_unknown_fr() {
    let root = repo_root().join("tests/fixtures/user-story");
    let (stories, _) = extract_paths(&[root]);
    let mut known = parse_fr_list("FR-PLAN-001");
    known.remove("FR-PLAN-002");
    let missing = check_coverage(&stories, &known);
    assert!(!missing.is_empty(), "expected unknown-FR errors when coverage is incomplete");
}
