# Local-CI Attestation + Tamper-Guard

hwLedger uses a **local-CI attestation** system instead of remote GitHub
Actions runners (billing is exhausted). Every pre-push run emits a signed,
hash-chained JSON manifest attesting that the local quality gates passed.
The (optional self-hosted) `attestation-gate.yml` workflow verifies the
manifest — it does *not* re-run the gates. **The local run IS the CI.**

## Architecture

```
pre-commit hooks          pre-push hooks (lefthook)                     remote
─────────────────         ──────────────────────────────────────        ──────
 fmt                       unverified-manifests                          github
 secrets-files             workspace-verify (clippy + test + fmt)          │
 trufflehog                tape-assertions                                 │
                           traceability-journeys                           │
                           swift-main                                      │
                           ─────────────── attest-build ───────────────┐   │
                           attest-verify-push                          │   │
                           release-tag                                 │   │
                                                                       ▼   ▼
                                              .hwledger/attestations.log   attestation-gate.yml
                                              (append-only, hash-chained,   (verifies signature
                                               ed25519-signed)               + chain, not gates)
```

## Attestation shape

```jsonc
{
  "version": 1,
  "commit_sha": "51af2d33dcadd5a0e0300a210c95b0b7575ffd64",
  "tree_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "parent_attestation_hash": "5b84d7c49be5f3500619c62fae26a03a781cd23bf6d2ec0c9addaddbc999b84f",
  "checks": [
    { "name": "cargo-fmt-check",    "passed": true, "duration_ms":  843, "evidence_sha256": "..." },
    { "name": "cargo-clippy",       "passed": true, "duration_ms":12034, "evidence_sha256": "..." },
    { "name": "cargo-test",         "passed": true, "duration_ms":41209, "evidence_sha256": "..." },
    { "name": "tape-assertions",    "passed": true, "duration_ms": 3120, "evidence_sha256": "..." },
    { "name": "traceability-journeys","passed": true,"duration_ms": 890, "evidence_sha256": "..." }
  ],
  "host": { "os": "macos", "hostname": "dev-laptop", "user": "koosha", "timestamp": "2026-04-21T04:13:18Z" },
  "signature": { "key_id": "default", "sig_hex": "5cbf..0d" },
  "hash": "66fa8d25a6efda8d3f17102f8e298451f8a5f54d7458dcd0d592126cd3dbc6e1"
}
```

## Hash chain (event-sourced, tamper-evident)

Each attestation carries `parent_attestation_hash = sha256(previous
canonical_payload || "|" || previous.sig_hex)`. Any of the following break
the chain and are detected by `hwledger attest chain`:

- reordered log entries
- rewritten entries (hash mismatch against stored `hash`)
- forged entries (signature fails against the keystore)
- synthesised entries inserted anywhere — their parent won't match

## Signing flow

1. Gather `commit_sha` (from `git rev-parse HEAD`), `tree_hash`
   (sha256 of `git ls-tree -r HEAD`), the last attestation's `hash` as
   parent.
2. Run each gate; capture stdout+stderr, sha256 → `evidence_sha256`.
3. Canonicalise the payload (sorted-key JSON, no whitespace).
4. ed25519-sign the canonical bytes with `$HWLEDGER_ATTEST_KEY_DIR/<key_id>.sk`.
5. `hash = sha256(canonical_payload || "|" || sig_hex)`.
6. Append JSON-line to `.hwledger/attestations.log`.

## Trust model

Public keys live at `~/.hwledger/attest-keys/<dev-id>.pub` (raw 32-byte
ed25519). On CI runners, they're committed to `.hwledger/pub-keys/` and
`HWLEDGER_ATTEST_KEY_DIR` points there. Signatures by any key **not** in
that registry are rejected.

**Key rotation:** append the new key (`<dev-id>-2026-04.pub`); the old key
stays for verifying older entries. Never delete a key that signed a
still-referenced attestation — doing so breaks the chain.

**Key loss recovery:** generate a new key, submit a PR adding the new
`.pub` to `.hwledger/pub-keys/`, and write a ROTATION.md entry linking
commit ranges to key ids. Past attestations from the lost key remain
verifiable; future pushes use the new key.

## Lockdown: `--no-verify` and `--force` are structurally blocked

The `git-wrapped` binary (built from `crates/hwledger-attest`) is aliased
as `git` and rejects `--no-verify`, `--force`, `-f`, and
`--force-with-lease` with exit code 100 unless both:

- `HWLEDGER_ALLOW_FORCE=1`
- `HWLEDGER_FORCE_JUSTIFICATION=/path/to/signed-justification.txt` exists

are set. Add this to `~/.zshrc`:

```sh
alias git="$HOME/.cargo/bin/git-wrapped"
export HWLEDGER_REAL_GIT=/usr/bin/git
```

Remote-side lockdown is configured via `scripts/branch-protection.json`:
`allow_force_pushes=false`, `enforce_admins=true`, `required_linear_history=true`,
and `attestation-gate / verify-attestation` as a required status check.

Apply with:

```sh
gh api -X PUT repos/KooshaPari/hwLedger/branches/main/protection \
  --input scripts/branch-protection.json
```

## First-run setup

```sh
cargo install --path crates/hwledger-attest --bin hwledger-attest
cargo install --path crates/hwledger-attest --bin git-wrapped
cargo run -p hwledger-attest -- genkey default
# (optional) publish your .pub to .hwledger/pub-keys/<dev-id>.pub and commit
```

## CLI reference

| command | description |
|---|---|
| `hwledger attest build --key-id <id>` | Run gates, sign, append to log. |
| `hwledger attest verify [<file>]` | Verify one attestation (file or HEAD). |
| `hwledger attest chain` | Walk the chain, highlight breaks. |
| `hwledger attest genkey <id>` | Generate a new ed25519 keypair. |
| `hwledger-attest verify-push` | Verify HEAD commit has a valid attestation (used by CI). |
| `hwledger-attest lockdown-check -- <git args>` | Used by `git-wrapped`. |

## Recovery procedures

- **Missing attestation for a commit already pushed** — rebuild it with
  `hwledger attest build` against the same commit (tree hash will match),
  then force-push the log (override required, see above).
- **Chain break after a merge** — re-linearise: discard the broken range
  of the log, rebuild one attestation per commit since the last valid
  parent.
- **Lost signing key** — follow "Key loss recovery" above.
