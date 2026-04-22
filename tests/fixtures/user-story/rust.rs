//! Fixture: canonical Rust user-story frontmatter.
//!
//! Not compiled — included only by `user-story-extract` tests and harvester
//! integration.

// @user-story
// journey_id: fixture-rust-first-plan
// title: First plan on a MacBook
// persona: Solo developer on a MacBook M3
// given: >
//   A clean install of hwLedger on macOS with the cargo toolchain already
//   present. See FR-PLAN-001 for the contract.
// when:
//   - run `hwledger plan --model deepseek-ai/DeepSeek-R1 --target m3-max`
//   - answer the interactive prompt with `y`
// then:
//   - stdout contains a planned quantization selection
//   - exit code is 0
//   - a `plan-*.json` manifest is written to `~/.hwledger/plans/`
// traces_to:
//   - FR-PLAN-001
//   - FR-PLAN-002
// record: true
// blind_judge: auto
// family: cli
// @end
#[test]
fn fixture_rust_first_plan() {
    // Body intentionally empty — fixture is harvested, not executed here.
}
