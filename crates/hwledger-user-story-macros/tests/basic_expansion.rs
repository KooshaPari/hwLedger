//! Positive-path expansion tests for `#[user_story_test]`.
//!
//! These verify the macro compiles valid frontmatter into a working `#[test]`
//! function whose body still runs the user's assertions.

use hwledger_user_story_macros::user_story_test;

#[user_story_test(yaml = r#"
journey_id: macro-smoke
title: "macro smoke test"
persona: framework maintainer
given: the proc-macro expands cleanly
when:
  - run the expanded test
then:
  - the body executes
  - the assertion passes
traces_to: [FR-USER-STORY-001]
record: false
blind_judge: skip
family: cli
"#)]
fn macro_smoke_expands_and_runs() {
    let sum: i32 = (1..=4).sum();
    assert_eq!(sum, 10);
}

#[user_story_test(yaml = r#"
journey_id: macro-minimal
title: "minimal required fields only"
persona: author
given: only the required fields are present
when: ["do nothing"]
then: ["pass"]
traces_to: [FR-USER-STORY-002]
"#)]
fn macro_minimal_required_fields() {
    // The macro must default record/blind_judge/family sensibly.
}
