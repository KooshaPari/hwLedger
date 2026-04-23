use hwledger_user_story_macros::user_story_test;

#[user_story_test(yaml = r#"
journey_id: ok-id
title: "bad traces_to"
persona: x
given: y
when: ["a"]
then: ["b"]
traces_to: [lowercase-not-fr]
"#)]
fn t() {}

fn main() {}
