# ADR 0034 — User stories sourced from tests

Constrains: FR-PLAN-001 through FR-PLAN-007, FR-UI-001 through FR-UI-004, FR-FLEET-001 through FR-FLEET-008, FR-TEL-001 through FR-TEL-004, FR-TRACE-001 through FR-TRACE-004, FR-VERIFY-001, FR-UX-VERIFY-001 through FR-UX-VERIFY-003.

Date: 2026-04-22
Status: Accepted

## Context

The journey-recording pipeline today is a four-artefact chain per journey:

1. a hand-written `.tape` (VHS cassette) or Playwright spec,
2. a hand-written `*.intents.yaml` matching VLM-audit annotations,
3. a rendered `manifest.verified.json`, and
4. a curated docs page (`docs-site/.../journeys/<id>.md`).

Across 26 shipped journeys (15 CLI, 6 Streamlit, 5 macOS GUI) we observe that the four artefacts drift: intent labels stop matching rendered frames, doc prose lags behind new assertions, and the `traces_to` list in the manifest is maintained by hand. Every new journey needs ~4 commits touching at least 6 files to land cleanly, and the "source of truth" for a journey's contract (the persona, the Given/When/Then, the FR trace) is effectively duplicated in every artefact.

The user-story-as-test proposal turns the model inside out: a single test file in the native test framework for each platform carries the full contract as YAML frontmatter, and every downstream artefact (tape, manifest, video, doc page, traceability row) is generated from that test.

## Options

| Option | Source of truth | Author ergonomics | Tool chain | Drift risk | Works across Rust / Swift / Playwright / k6 |
|---|---|---|---|---|---|
| **A. Tests-as-source (chosen)** | Native test file w/ language-idiomatic comment block | High — one file, one PR, language-native | Small Rust harvester + per-lang plugin | Low — one copy of each fact | Yes |
| B. Status quo (hand-written tapes + manifests + doc pages) | Scattered | Low | None extra | High — observed | Yes |
| C. Cucumber `.feature` files | `.feature` text | Medium — extra DSL, foreign to each test framework | Cucumber runners per lang | Medium — .feature vs. step-defs drift | Partially; k6 poor fit |
| D. Standalone YAML DSL (no test) | `journeys/*.yaml` | Low — adds a 3rd artefact class | Custom runner per lang | Medium | Yes, but by duplicating logic |
| E. Codegen-from-manifest (inverse direction) | `manifest.verified.json` | Low — post-hoc, hard to author | Codegen templates | Medium — manifests are an output, not a contract | Weak |

### Why not B

Observed drift is the motivating bug. Doing nothing keeps the 4-commits-per-journey cadence and the silent doc lag.

### Why not C

Cucumber works well for web-first teams but imposes a DSL that is foreign to XCUITest and k6, and requires step-definition glue that recreates the drift problem one layer deeper.

### Why not D

A third artefact class (YAML beside test beside manifest) multiplies surface area. We want fewer artefacts, not more.

### Why not E

Reversing the direction means the manifest — which is a build output — becomes the contract. Manifests are video-keyed, binary-adjacent, and machine-written; they are a terrible authoring surface.

## Decision

Adopt **Option A**: every user-story lives as frontmatter on the native test that exercises it.

- **Rust** — `// @user-story ... // @end` line-comment block above a `#[test]` or `#[tokio::test]`. A future proc-macro (Batch 2) will additionally surface the body as a runtime constant.
- **Swift** — `// MARK: @user-story ... // MARK: @end` above an `XCTestCase` method. Xcode's MARK navigator surfaces the story inline.
- **Playwright / Streamlit** — JSDoc block `/** @user-story ... */` above a `test(...)` call.
- **k6** — `/* @user-story ... */` block comment at module scope.

A single Rust binary, `user-story-extract`, walks declared roots, parses the comment blocks, validates them against the canonical `user-story.schema.json` (shared with `phenotype-journeys`), and writes `docs-site/quality/user-stories.json`. Downstream consumers (tape generator, manifest reconciler, auto-doc pipeline, traceability matrix) read only that JSON index.

Batch 1 ships: the schema, the extractor, fixtures in four languages, ADR 0034, and a migration plan. Later batches add the Rust proc-macro, the Playwright runtime plugin, the XCUITest helper, the auto-doc generator, and the actual migration of the 26 existing journeys.

## Consequences

### Positive

- 4 commits per journey collapse to 1 test file.
- The Given/When/Then is collocated with the assertions that enforce it — reading the test is reading the contract.
- `traces_to` is enforced at harvest time against `PRD.md`; unknown FRs fail CI (`user-story-extract check-coverage`).
- Duplicate `journey_id`s fail CI (`check-duplicate-ids`).
- The harvester is pure Rust with a schema drawn from `phenotype-journeys` — no new non-Rust runtime introduced, scripting-policy compliant.

### Negative / obligations

- The existing 26 journeys need migration. This is a non-trivial sweep, planned and sequenced in `docs-site/quality/user-story-migration-2026.md`; migration is carved into batches 2 – 6 by platform family.
- A proc-macro crate (Batch 2) is needed to give Rust tests a runtime handle on the frontmatter (for tape-emission and assertion-embedding). The library surface in Batch 1 intentionally does not require it.
- The Playwright plugin (Batch 3) must wire JSDoc frontmatter into `@playwright/test` `testInfo.annotations` so recordings can tag frames with the story.
- XCUITest needs a small helper (Batch 4) because MARK comments are not reflected into the XCUITest runtime; the helper will take a `journey_id` string and look it up in the harvested index.
- The Rust backend needs PTY subprocess wiring (also Batch 2) to drive real CLI binaries and capture terminal state for tape emission; this reuses `cli-journey-record` machinery but upgrades it to read from harvested stories rather than hand-written tapes.
- Existing hand-written `.intents.yaml` files become derived outputs; until migration completes, both the old and the harvested forms must coexist. The auto-doc generator (Batch 5) will prefer harvested stories and fall back to hand-written manifests only for journeys not yet migrated.

## Revisit when

- The proc-macro becomes restrictive (e.g., cannot handle conditional stories, multi-step journeys spanning tests).
- Test frameworks fragment further — a new target platform (Android, WASM, TUI-native) would need another comment-block dialect.
- User stories require runtime-only data (e.g., stories parametrised by fleet inventory at test-run time) that YAML frontmatter cannot express. At that point, consider a hybrid: frontmatter for the static contract, a Rust builder API for dynamic bindings.
- The `phenotype-journeys` schema diverges enough from hwLedger's needs to justify a local superset; at that point, fork the schema and version it explicitly.

## References

- Schema: `phenotype-journeys/crates/phenotype-journey-core/schema/user-story.schema.json`
- Harvester: `tools/user-story-extract/`
- Fixtures: `tests/fixtures/user-story/{rust.rs,swift.swift,playwright.spec.ts,k6.js}`
- Migration plan: `docs-site/quality/user-story-migration-2026.md`
- Prior art: ADR 0025 (journey-manifest-json-schemars), ADR 0018 (web-framework-streamlit)
