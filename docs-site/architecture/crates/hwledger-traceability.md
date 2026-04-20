---
title: hwledger-traceability
description: Spec → test → code traceability scanner — reports which FRs have tests and which tests reference which FR.
---

# hwledger-traceability

**Role.** Parses `FUNCTIONAL_REQUIREMENTS.md`, scans Rust sources and tests, and emits a coverage report linking each FR to its tests and to the code that implements it.

## Why this crate

The hwLedger governance bar is "every FR has ≥1 test and every test references ≥1 FR." That is a hard CI rule, not an aspiration. A scanner that enforces it automatically is the only way to keep it honest as the spec grows. Without this crate, FR orphans — requirements with no test coverage — would pile up silently; so would tests that reference deleted FRs.

Rejected: using an external traceability tool (Reqtify, Jama). Rejected because the tooling weight was absurd for a Rust workspace and because the scanner needs to live in the repo and run in `cargo test`-adjacent time (<5 s for the whole tree).

**Belongs here:** Markdown FR parsing, `walkdir`-driven Rust scan, coverage classification.
**Does not belong here:** CI wiring (that's a workspace-level script), UI rendering.

## Public API surface

| Type | Name | Stability | Notes |
|------|------|-----------|-------|
| struct | `PrdParser` | stable | Parses FR headings + IDs |
| struct | `FrSpec` | stable | One FR: id, text, anchors |
| enum | `FrKind` | stable | Functional / Non-functional / Constraint |
| struct | `Stats` | stable | Totals + gap counts |
| struct | `FrCoverage` | stable | Per-FR evidence |
| enum | `CoverageLevel` | stable | `None / Partial / Full` |
| struct | `CoverageReport` | stable | Full audit result |
| mod | `scan` | stable | Filesystem walker + ref extraction |

## When to reach for it

1. **Pre-merge gate** — `cargo run -p hwledger-traceability -- report` must print `CoverageLevel::Full` for every FR.
2. **Onboarding** — pointing at an FR and asking "show me its tests" is a one-liner.
3. **Refactor safety** — after moving code, rerun to ensure FR references still resolve.

## Evolution

| SHA | Note |
|-----|------|
| `97fcc68` | `feat(p3,p5,test,docs): Wave 9 — WP26 VHS CLI pipeline + WP32 traceability + WP31 AgilePlus cycle + ADR-0008` — initial landing |
| `ec1f8bf` | `feat(release): ship v0.1.0-alpha + coverage uplift (273->329) + cross-dim traceability (85%+)` |
| `91ecc5d` | `feat(FR-PLAN-007): add config exporters for vLLM, llama.cpp, MLX` — scanner extended for exporter FRs |
| `e23cf4d` | `feat(spec-close): 4 parallel agents ... + zero-coverage fix` |

**Size.** 1,473 LOC, 21 tests.

## Design notes

- FR IDs use the convention `FR-<AREA>-NNN`; the scanner's regex is pinned in one place.
- Report is renderable as JSON or Markdown; Markdown is what gets committed into `docs-site/quality/`.
- No filesystem writes; the CLI subcommand does the printing.

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-traceability)
- [Quality Traceability](/quality/traceability)
