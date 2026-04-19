# hwledger-verify: Blackbox Screenshot Verification

Automated user-journey verification via Claude Opus 4.7 (vision) + Claude Sonnet 4.6 (judge equivalence). Traces to WP27 and research brief `docs/research/12-ui-journey-harness-2026.md`.

## What it does

1. **Describe** a PNG screenshot using Claude Opus 4.7 vision (`FR-UX-VERIFY-001`)
2. **Judge** whether the description matches a declared intent using Claude Sonnet 4.6 (`FR-UX-VERIFY-002`)
3. **Verify** all steps in a journey manifest and emit `manifest.verified.json` for rendering (`FR-UX-VERIFY-003`)

## Architecture

```
┌─────────────────────┐
│  Journey Manifest   │ (journeys/<id>/manifest.json)
│  + Step Screenshots │ (PNG files referenced in manifest)
└──────────┬──────────┘
           │
           ▼
    ┌──────────────┐
    │  Verifier    │ (main engine)
    │  + Cache     │ (disk-based SHA256 keying)
    └──────┬───────┘
           │
    ┌──────┴─────────────┬──────────────┐
    │                    │              │
    ▼                    ▼              ▼
┌───────────┐      ┌────────────┐  ┌─────────┐
│ Describe  │      │   Judge    │  │ Verify  │
│ (Opus)    │      │ (Sonnet)   │  │ Manifest│
└───────────┘      └────────────┘  └─────────┘
    │                    │              │
    ▼                    ▼              ▼
   PNG ───► Description  Intent  ───► Verdict
           (structured)   Label          (1-5)

Output: manifest.verified.json
{
  "journey_id": "planner-qwen2-7b-32k",
  "steps": [{
    "intent": "...",
    "description": { "text": "...", "tokens_used": 250 },
    "verdict": { "score_1_to_5": 4, "rationale": "...", "tokens_used": 100 }
  }],
  "overall_score": 4.1,
  "total_tokens": 2350,
  "verified_at": "2026-04-18T..."
}
```

## Cost estimate (NFR-VERIFY-001)

For a typical 8-step journey (e.g., `planner-qwen2-7b-32k`):

| Step | Model | Input Tokens | Output Tokens | Cost |
|------|-------|--------------|---------------|------|
| Describe (8×) | Opus 4.7 | ~20,000 | ~4,000 | $0.066 |
| Judge (8×) | Sonnet 4.6 | ~6,400 | ~800 | $0.002 |
| **Total** | — | ~26,400 | ~4,800 | **~$0.068** |

(Claude Opus 4.7: $3/$15 per 1M in/out; Sonnet 4.6: $3/$15 per 1M in/out)

## Features

- **Caching**: Disk-based cache at `target/hwledger-verify-cache/` keyed on `SHA256(screenshot + model + version)` + judge calls to avoid re-running identical verifications.
- **Retry logic**: Exponential backoff on 429 (rate limit) and 5xx errors; max 3 attempts (250ms, 500ms, 1000ms).
- **Structured responses**: Claude returns JSON; parsed into `Description.structured` for downstream processing.
- **Offline tests**: 28 unit + integration tests with wiremock mocking (no live API calls unless `HWLEDGER_VERIFY_LIVE=1`).

## Installation

```bash
# Via workspace
cargo build -p hwledger-verify

# Binary available at
./target/debug/hwledger-verify
```

## Usage

### Library

```rust
use hwledger_verify::{Verifier, VerifierConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = VerifierConfig::with_api_key("sk-ant-...".to_string());
    let verifier = Verifier::new(config)?;

    // Describe a screenshot
    let png_bytes = std::fs::read("screenshot.png")?;
    let desc = verifier.describe(&png_bytes).await?;
    println!("Description: {}", desc.text);
    println!("Tokens: {}", desc.tokens_used);

    // Judge intent vs. description
    let verdict = verifier.judge(
        "User clicks model picker dropdown",
        &desc.text,
    ).await?;
    println!("Score: {}/5", verdict.score_1_to_5);

    // Verify entire manifest
    let verification = verifier.verify_manifest(
        std::path::Path::new("journeys/planner-qwen2-7b-32k/manifest.json")
    ).await?;
    println!("Overall: {:.2}/5.0", verification.overall_score);
    
    // Write results
    let json = serde_json::to_string_pretty(&verification)?;
    std::fs::write("manifest.verified.json", json)?;

    Ok(())
}
```

### CLI

#### Describe a single screenshot

```bash
hwledger-verify describe screenshot.png \
  --model claude-opus-4-7

# Output:
# Description:
# A user interface showing a dropdown menu with model selection options...
#
# Structured:
# {"description": "...", "visible_elements": [...], "notable_state": "..."}
#
# Tokens used: 250
```

#### Judge intent vs. description

```bash
hwledger-verify judge \
  --intent "User opens model picker and selects Qwen2-7B" \
  --description "A dropdown menu appears with model options; Qwen2-7B is highlighted"

# Output:
# Intent:
# User opens model picker and selects Qwen2-7B
#
# Description:
# A dropdown menu appears with model options; Qwen2-7B is highlighted
#
# Verdict:
# Score: 5/5
# Rationale: Perfect match: description accurately describes the action and visible state.
#
# Tokens used: 85
```

#### Verify an entire journey manifest

```bash
hwledger-verify manifest journeys/planner-qwen2-7b-32k/manifest.json \
  --out manifest.verified.json

# Output:
# Results:
#   Journey ID: planner-qwen2-7b-32k
#   Steps: 8
#   Overall score: 4.2/5.0
#   Total tokens: 2350
#
# Per-step scores:
#   Step 0: 5 — App launches and shows Planner as default screen
#   Step 1: 5 — Planner screen is the default visible tab
#   Step 2: 4 — User opens model picker and selects Qwen2-7B
#   ...
#
# Verification written to manifest.verified.json
```

### Environment variables

- `ANTHROPIC_API_KEY`: Primary API key source (checked first)
- `HWLEDGER_ANTHROPIC_API_KEY`: Fallback API key
- `ANTHROPIC_BASE_URL`: Override API endpoint (for testing; defaults to `https://api.anthropic.com`)
- `RUST_LOG`: Control logging verbosity (e.g., `RUST_LOG=hwledger_verify=debug`)

### Caching

Cache is stored at `target/hwledger-verify-cache/` and keyed on:

- Describe calls: `SHA256(screenshot_bytes + model + "describe-v1")`
- Judge calls: `SHA256(intent + description + model + "judge-v1")`

**Clear cache**:
```bash
rm -rf target/hwledger-verify-cache/
```

**Disable cache** (e.g., for testing):
```rust
let config = VerifierConfig::default().with_cache_disabled();
let verifier = Verifier::new(config)?;
```

or CLI:
```bash
hwledger-verify manifest manifest.json --no-cache
```

## Testing

```bash
# All tests (offline, no API calls)
cargo test -p hwledger-verify --all-targets

# Specific test
cargo test -p hwledger-verify test_api_retry_on_429

# With logging
RUST_LOG=hwledger_verify=debug cargo test -p hwledger-verify -- --nocapture

# Integration tests only
cargo test -p hwledger-verify --test integration

# Library tests only
cargo test -p hwledger-verify --lib
```

### Test coverage

- **28 total tests**: 12 unit (lib) + 16 integration
- All tests traced to FR-UX-VERIFY-* functional requirements
- Mocking: wiremock for HTTP (200, 429, 500, 401 scenarios)
- Golden file test: hardcoded judge response with fixed verdict parsing
- Cache tests: round-trip serialization, cache hit detection
- Manifest parsing: real manifest.json structure validation

## API Integration Gotchas

### Vision image limits (Claude Opus 4.7)

- **Max images per request**: 100
- **Max image dimensions**: 2576 × 2576 px (3.75 MP)
- **Max request size**: 32 MB
- **Animated GIF support**: ❌ Only first frame processed (see brief §3)

### Token accounting

- Input + output tokens are summed and returned in `tokens_used`
- Vision images consume tokens proportionally (rough: 85 tokens per 400×600 image at Opus pricing)
- Retry delays do NOT incur additional API charges

### Error handling

- **429 (rate limit)**: Automatic exponential backoff; transparent to caller
- **5xx (server error)**: Retry; fails after max attempts
- **4xx (client error)**: Fail immediately; check API key, request format
- **Missing screenshot**: Fails loudly with path during manifest verification

## Anthropic API version

- **API version**: `2023-06-01` (currently pinned in `client.rs`)
- **Models**: `claude-opus-4-7` (describe), `claude-sonnet-4-6` (judge)
- Check Anthropic docs for latest model IDs and pricing

## Related work products

- **WP25**: XCUITest + ScreenCaptureKit recording (SwiftUI app driver + PNG/MP4 capture)
- **WP26**: VHS tape scripts + ffmpeg pipeline (CLI recording)
- **WP28**: VitePress JourneyViewer component + auto-sidebar (rendering verified manifests)
- **WP29**: Keyframe gallery + LLM-judge (this crate)
- **WP30**: VitePress docs integration (embedding MP4 + verification badges)

## License

Apache-2.0 (inherited from hwLedger workspace)

## Specification

See `docs/research/12-ui-journey-harness-2026.md` (Part 2, WP29) for full research, constraints, cost analysis, and R&D unknowns.
