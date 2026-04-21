# ADR 0031 — CI: hybrid lefthook-authoritative + self-hosted runner verification

Constrains: FR-QA-001, FR-JOURNEY-007

Date: 2026-04-19
Status: Accepted

## Context

GitHub-hosted runners are billable and — per the workspace-wide billing constraint — unusable for this org. But we still need automated verification of journey attestation + signed releases. The question is where CI authoritatively runs.

## Options

| Option | Cost | Throughput | Hardware parity with users | Billing risk | Attestation trust |
|---|---|---|---|---|---|
| GitHub-hosted runners | $$$$ | High | No (Ubuntu/macOS VMs) | High (blocks org) | Medium |
| Self-hosted macOS runner | Electricity | Medium | Yes | Zero | High (we own it) |
| Cirrus CI | $$ | High | macOS M-series | Medium | Medium |
| CircleCI | $$$ | High | Variable | Medium | Medium |
| Buildkite | $$ (agent only) | High | Bring-your-own | Zero | High |
| Local-only (lefthook) | $0 | Dev-machine | Yes | Zero | Low (no witness) |

## Decision

Two-tier hybrid:

1. **lefthook is authoritative for green/red.** Every commit runs lefthook's pre-push (fmt + clippy + test + docs lint + journey smoke). A failing lefthook blocks the push; nothing else gates merges.
2. **Self-hosted runner re-verifies signed attestations.** The runner (a dedicated Apple Silicon mac at home) pulls main after each push, runs the full journey suite, signs the attestation with the org ed25519 key (ADR-0024), and publishes to the ledger.

GitHub Actions workflows exist but are thin: they trigger the self-hosted runner webhook and otherwise no-op. No billable minutes consumed.

## Rationale

- Zero billing exposure. Matches the workspace billing constraint.
- Self-hosted runner has exact hardware parity (M3 Max) with end users, which is critical for GPU telemetry validation (ADR-0023).
- Signed attestations produced off-runner carry the same trust as any git-signed tag: the key, not the runner, is the trust root.
- Lefthook as authority means red/green is visible to the developer before push — no "waiting for CI."

## Consequences

- A single self-hosted runner is a single point of failure. Mitigated by a second runner on a different machine, hot spare.
- Contributors must install lefthook locally. Documented in `CONTRIBUTING.md`.
- External contributors (via PR fork) cannot run the self-hosted tier; maintainers run it on merge.

## Revisit when

- GitHub Actions billing is resolved (budget or self-hosted GH runner on infra).
- Runner throughput becomes the bottleneck (parallelize or add more runners).
- Attestation requirements change (e.g., SLSA L3 demands a hardened builder).

## References

- lefthook: https://github.com/evilmartians/lefthook
- ADR-0024 (ed25519 attestation).
