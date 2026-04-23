use hwledger_user_story_macros::user_story_test;

#[user_story_test(yaml = r#"
journey_id: ok-id
title: "unterminated
persona: x
"#)]
fn t() {}

fn main() {}
