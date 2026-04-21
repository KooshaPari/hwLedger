//! Integration test: simulate `git push --no-verify` through the wrapper,
//! assert rejection with the expected exit code and banner.
//!
//! Traces to: FR-ATTEST-006 — lockdown blocks --no-verify.

#![cfg(unix)]

use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn rejects_no_verify() {
    let mut cmd = Command::cargo_bin("git-wrapped").unwrap();
    // Unset any override the caller's env might carry.
    cmd.env_remove("HWLEDGER_ALLOW_FORCE").env_remove("HWLEDGER_FORCE_JUSTIFICATION").args([
        "push",
        "--no-verify",
        "origin",
        "main",
    ]);
    cmd.assert().failure().code(100).stderr(contains("BLOCKED")).stderr(contains("--no-verify"));
}

#[test]
fn rejects_force() {
    let mut cmd = Command::cargo_bin("git-wrapped").unwrap();
    cmd.env_remove("HWLEDGER_ALLOW_FORCE")
        .env_remove("HWLEDGER_FORCE_JUSTIFICATION")
        .args(["push", "--force", "origin", "main"]);
    cmd.assert().failure().code(100).stderr(contains("--force"));
}

#[test]
fn allows_override_with_justification() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    // Still should exec /usr/bin/git which will fail on bogus args, but we
    // should get past the lockdown banner.
    let mut cmd = Command::cargo_bin("git-wrapped").unwrap();
    cmd.env("HWLEDGER_ALLOW_FORCE", "1")
        .env("HWLEDGER_FORCE_JUSTIFICATION", tmp.path())
        // Point at a non-existent git so exec fails predictably — we only
        // care that the lockdown gate was passed.
        .env("HWLEDGER_REAL_GIT", "/nonexistent/git-bogus")
        .args(["push", "--no-verify"]);
    let out = cmd.assert().failure();
    out.stderr(contains("lockdown override accepted"));
}
