use hwledger_user_story_macros::user_story_test;

#[user_story_test(yaml = r#"
journey_id: ok-id
title: "bad family"
persona: x
given: y
when: ["a"]
then: ["b"]
traces_to: [FR-X-001]
family: android
"#)]
fn t() {}

fn main() {}
