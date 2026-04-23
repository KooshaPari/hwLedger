use hwledger_user_story_macros::user_story_test;

#[user_story_test(yaml = r#"
title: "missing journey id"
persona: x
given: y
when: ["a"]
then: ["b"]
traces_to: [FR-X-001]
"#)]
fn t() {}

fn main() {}
