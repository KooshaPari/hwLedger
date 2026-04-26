# ADR 0005 — Shared-crate reuse contract with Phenotype workspace

Constrains: FR-FLEET-006, FR-TEL-001

Date: 2026-04-19
Status: Accepted

## Context

hwLedger lives under the `repos/` tree and is a sibling to `repos/crates/`, which already hosts Phenotype-wide shared crates. Per the `PHENOTYPE_SHARED_REUSE_PROTOCOL` in the workspace CLAUDE.md and the wrap-over-handroll mandate in the global rules, hwLedger must consume these crates rather than reimplement the same abstractions.

Five crates are in scope for hwLedger v1:

| Crate | Used in hwLedger | Purpose |
|-------|------------------|---------|
| `phenotype-event-sourcing` | `hwledger-ledger`, `hwledger-server` | SHA-256 hash-chained append-only audit log. |
| `phenotype-error-core` | all hwLedger crates | Canonical error types; avoids error-enum proliferation (2026-03 LOC-reduction lesson). |
| `phenotype-config` | `hwledger-server`, `hwledger-agent`, `hwledger-cli` | Figment-based unified loader for hwLedger's *own* config, not model configs. |
| `phenotype-cache-adapter` | `hwledger-ingest` | Two-tier LRU + DashMap cache fronting HF metadata fetches. |
| `phenotype-health` | `hwledger-server`, `hwledger-agent` | `HealthChecker` trait for heartbeat endpoints. |

## Decision

### Consumption model

- **Path dependencies** from the hwLedger workspace `Cargo.toml` to `../crates/phenotype-*`. Declared in `[workspace.dependencies]`; member crates pull with `foo.workspace = true`.
- **No submodule vendoring.** We rely on the shared workspace layout being checked out as a sibling. This matches how every other Phenotype project consumes them.
- **Versioning.** We track workspace tip in development. At hwLedger release time we pin to the commit hash of the `repos/crates/` tree in the release notes for auditability.

### Stability contract

- hwLedger treats the Phenotype-shared crates as **semantically stable within a release train** (30-day window). Breaking changes upstream require either a hwLedger PR adopting them or a deliberate version pin.
- hwLedger does **not** fork these crates. Required extensions must either:
  1. Land upstream in `repos/crates/phenotype-*` via a separate PR (preferred), or
  2. Wrap the upstream type in a thin hwLedger-local trait (`hwledger-*-ext`) without copy-paste.

### Extraction policy (outgoing)

hwLedger-originated candidates for promotion to `repos/crates/` per the cross-project reuse protocol (forward-only migration, ask before moving):

| hwLedger crate | Promotion trigger |
|----------------|-------------------|
| `hwledger-probe` (GpuProbe trait + backends) | Second consumer appears (likely `heliosCLI` or `PhenoObservability`). |
| `hwledger-arch` (KV-formula library) | Any other project needs LLM-capacity math (candidate: AgilePlus model-routing). |
| `hwledger-mlx-sidecar` (JSON-RPC protocol) | Any other Rust-hosted Python-sidecar use case lands. |

Extraction is a separate, reviewed ADR — never implicit.

## Consequences

- hwLedger **requires** `repos/crates/` to be present. The workspace will not build standalone. Documented in `AGENTS.md`.
- Path deps complicate OSS distribution: downstream users cloning only `KooshaPari/hwLedger` will need to also clone the Phenotype shared crates, or we publish the shared crates to crates.io ahead of hwLedger v1.0. Tracked as a release-engineering open item (ADR-0006, pending).
- Bus-factor is aligned with the rest of Phenotype: if the shared crates move, hwLedger moves with them.

## Rejected alternatives

- **Git submodules of the shared crates into hwLedger** — duplicates state, bypasses workspace consolidation, rejected.
- **Copy-paste the minimum needed** — would reintroduce the error-enum proliferation explicitly eliminated in the 2026-03-29 Phase-1 LOC-reduction. Rejected.
- **Publish each shared crate to crates.io before each hwLedger change** — too slow a cadence for development; reserve for release time.

## Realized

- 2026-04-25: `phenotype-health` adopted via **git dep** (`KooshaPari/phenoShared` `main` branch) rather than path/submodule. Wired into both `hwledger-server` and `hwledger-agent` `Cargo.toml` as workspace dep `phenotype-health.workspace = true`. This satisfies the ≥3-consumer floor for `phenotype-health` (TestingKit + hwledger-server + hwledger-agent).
- Rationale for git over path: hwLedger CI builds reproducibly from the canonical fork without requiring a sibling-checkout of the parent `repos/` workspace, addressing the OSS-distribution open item flagged in the original Consequences section. Local dev override remains available via `[patch."https://github.com/KooshaPari/phenoShared"]`.
- The original "no submodule vendoring; rely on sibling checkout" decision still holds for crates not yet wired; the git-dep route supersedes it for `phenotype-health` and is the preferred path for future shared-crate adoptions.
- Handler implementation (actual `/health` endpoint wiring) is out of scope for this declaration PR — tracked separately.

## References

- Workspace CLAUDE.md: `PHENOTYPE_SHARED_REUSE_PROTOCOL`.
- Workspace memory: "Phase 1 LOC Reduction Execution Complete (2026-03-29)".
- `repos/crates/` READMEs for canonical API docs.
