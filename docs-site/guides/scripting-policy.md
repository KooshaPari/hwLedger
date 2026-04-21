# Scripting Policy

hwLedger is **Rust-only** for scripting. New shell is not permitted; extend
an existing Rust crate (typically `hwledger-devtools` or `hwledger-cli`)
instead.

## `--no-verify` and `--force` are structurally blocked

These flags are rejected by the `git-wrapped` shim at exit code 100. They
are not simply discouraged — they cannot be used without an explicit,
signed override. See
[attestation.md](../quality/attestation.md#lockdown-no-verify-and-force-are-structurally-blocked)
for the override procedure.

The rationale: GitHub Actions billing is exhausted, so all quality gates
run locally via lefthook pre-push. The signed attestation manifest at
`.hwledger/attestations.log` is how we prove a push is CI-green. Bypassing
pre-push bypasses CI.

## Adding a new gate

1. Implement the check as a Rust function in the appropriate crate.
2. Add it to the lefthook `pre-push` stages in `lefthook.yml`.
3. Add it to `run_all_gates()` in `crates/hwledger-attest/src/bin/main.rs`
   so the attestation captures its evidence hash.
4. Update `docs-site/quality/attestation.md` with the new check name.
