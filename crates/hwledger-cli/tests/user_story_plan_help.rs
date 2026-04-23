//! Consumer-side example for `#[user_story_test]` — rewrites the
//! `plan --help` CLI smoke test as a user-story-sourced test.
//!
//! This replaces the `plan` subcommand branch of `test_help_output` in
//! `integration.rs` for journey id `cli-plan-help`. The other subcommands in
//! the original test remain covered there until Batch 2 follow-ups migrate
//! them individually.

use assert_cmd::Command;
use hwledger_user_story_macros::user_story_test;

// Mirror of the YAML below so the Batch 1 harvester
// (`tools/user-story-extract`) can inventory this macro-sourced story.
// Batch 5 will teach the harvester to read `#[user_story_test]` directly
// and drop this companion block.
//
// @user-story
// journey_id: cli-plan-help
// title: "CLI — plan --help shows the --seq context window flag"
// persona: operator exploring the hwLedger CLI
// given: a fresh hwLedger install with the hwledger-cli binary on PATH
// when:
//   - "run `hwledger-cli plan --help`"
// then:
//   - "exit 0"
//   - "stdout contains 'Usage'"
//   - "stdout contains '--seq' (the context-window flag)"
// traces_to: [FR-PLAN-003]
// record: true
// blind_judge: auto
// family: cli
// @end

#[user_story_test(yaml = r#"
journey_id: cli-plan-help
title: "CLI — plan --help shows the --seq context window flag"
persona: operator exploring the hwLedger CLI
given: a fresh hwLedger install with the hwledger-cli binary on PATH
when:
  - "run `hwledger-cli plan --help`"
then:
  - "exit 0"
  - "stdout contains 'Usage'"
  - "stdout contains '--seq' (the context-window flag)"
traces_to: [FR-PLAN-003]
record: true
blind_judge: auto
family: cli
"#)]
fn plan_help_shows_context_flag() {
    let output =
        Command::cargo_bin("hwledger-cli").unwrap().args(["plan", "--help"]).output().unwrap();
    assert!(output.status.success(), "plan --help should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage"), "plan --help should contain 'Usage'");
    assert!(
        stdout.contains("--seq"),
        "plan --help should expose the --seq context window flag (see FR-PLAN-003)"
    );
}
