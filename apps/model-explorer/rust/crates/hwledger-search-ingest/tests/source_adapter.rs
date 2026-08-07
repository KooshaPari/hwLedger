//! End-to-end check for [`HuggingFaceAdapter::from_env`] env wiring.

use hwledger_search_ingest::HuggingFaceAdapter;

#[test]
fn from_env_reads_hf_token_when_set_and_none_otherwise() {
    let prior_token = std::env::var("HF_TOKEN").ok();

    // Case 1: HF_TOKEN set → adapter carries the token.
    std::env::set_var("HF_TOKEN", "secret-token");
    let a = HuggingFaceAdapter::from_env().expect("from_env ok with token");
    assert_eq!(a.token_snapshot(), Some("secret-token"));

    // Case 2: HF_TOKEN unset → adapter still builds, no token.
    std::env::remove_var("HF_TOKEN");
    let b = HuggingFaceAdapter::from_env().expect("from_env ok without token");
    assert_eq!(b.token_snapshot(), None);

    // Restore prior env so we don't disturb other tests.
    if let Some(v) = prior_token {
        std::env::set_var("HF_TOKEN", v);
    }
}
