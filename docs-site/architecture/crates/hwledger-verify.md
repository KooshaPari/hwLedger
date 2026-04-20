---
title: hwledger-verify
description: Black-box VLM-based verification of recorded journeys and audit manifests.
---

# hwledger-verify

**Role.** Runs a vision-language model against recorded GUI journey manifests to confirm each step's screenshot actually shows what the manifest claims. Also validates ledger hash-chain integrity.

## Why this crate

A recorded journey manifest says "Step 3: shows the per-layer KV heatmap." A developer could easily commit a stale screenshot or a wrong step label. Humans reviewing a PR do not catch this; a VLM can. This crate runs that check automatically in CI so documentation screenshots cannot silently drift from the code they document.

Rejected: hand-written image-diff tests with fixed golden PNGs. Rejected because (a) every OS paint cycle changes a few antialiased pixels, and (b) goldens drift with every legitimate UI change and generate review fatigue. A VLM judging semantic equivalence survives irrelevant pixel churn.

**Belongs here:** VLM client, prompt templates, cache of past verdicts, judge verdict types, manifest schema.
**Does not belong here:** the recorder itself (that's `hwledger-gui-recorder`), the hash-chain arithmetic (that's `hwledger-ledger`).

## Public API surface

| Type | Name | Stability | Notes |
|------|------|-----------|-------|
| struct | `Verifier` | stable | Top-level façade |
| struct | `VerifierConfig` | stable | Model name, API key env var, cache dir |
| fn | `VerifierConfig::with_api_key` | stable | Builder |
| fn | `VerifierConfig::with_describe_model` | stable | Builder |
| fn | `VerifierConfig::with_judge_model` | stable | Builder |
| fn | `VerifierConfig::with_base_url` | stable | Builder |
| fn | `VerifierConfig::with_cache_disabled` | stable | Builder |
| struct | `JourneyManifest` | stable | Shared with `hwledger-gui-recorder` |
| struct | `ManifestStep` | stable | One step + caption + screenshot path |
| struct | `StepVerification` | stable | Verdict for a single step |
| struct | `ManifestVerification` | stable | Aggregate verdict |
| struct | `JudgeVerdict` | stable | Pass / Fail / Unclear + rationale |
| struct | `Description` | stable | VLM-produced scene description |
| struct | `AnthropicClient` | stable | Claude Vision wrapper |
| struct | `Cache` | stable | Content-addressed verdict cache |
| enum | `VerifyError` | stable | API / cache / manifest errors |

## When to reach for it

1. **CI step `hwledger verify-journeys docs-site/journeys/*.json`** after tape recordings refresh.
2. **Authoring new recorded journeys** — run verify locally first; the cache means re-runs are cheap.
3. **Investigating a Fail verdict** — the verdict's rationale string points at the mismatch between caption and screen content.

## Evolution

| SHA | Note |
|-----|------|
| `5b20662` | `feat(p3,test,docs): Wave 8 — WP33 CLI + WP28 VitePress docsite + WP27 blackbox VLM verify` — initial landing |
| `ec1f8bf` | `feat(release): ship v0.1.0-alpha + coverage uplift (273->329)` |
| `e23cf4d` | `feat(spec-close): 4 parallel agents land heatmap-v2 + exporters + MLX real + SSH + mTLS CN + zero-coverage fix` |

**Size.** 1,143 LOC, 46 tests (most hit the cache, not the live API).

## Design notes

- Cache key is `sha256(image_bytes) || prompt_version`. Re-running on an unchanged image is free.
- `JudgeVerdict::Unclear` is treated as a soft-fail in CI but a hard-fail locally during spec authoring.
- The VLM is the only external dependency; the crate is usable offline if the cache is warm.

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-verify)
- [hwledger-gui-recorder](./hwledger-gui-recorder)
