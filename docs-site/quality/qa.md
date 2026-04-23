---
title: QA Posture
description: Complete quality assurance framework
---

# QA Posture

Our multi-layer quality enforcement across static analysis, testing, and runtime verification.

## Quality gates

Every commit must pass:

| Gate | Tool | Threshold | Trigger |
|------|------|-----------|---------|
| Clippy (lints) | cargo clippy | -D warnings | Pre-commit |
| Format | cargo fmt | --check | Pre-commit |
| Type checking | rustc | no errors | Build |
| Unit tests | cargo test | 100% pass | CI |
| Integration tests | cargo test --test | 100% pass | CI |
| Coverage | cargo tarpaulin | >= 85% | CI |
| SAST (Semgrep) | semgrep | 0 high/critical | CI |
| Code signing | CodeQL | pass | CI |

## Static analysis

### Clippy (-D warnings)

```bash
cargo clippy --workspace -- -D warnings
```

Enforces Rust idioms: unused imports, needless borrows, panic! instead of Result, etc.

**Policy**: All warnings must be fixed. No `#[allow]` without detailed comment.

### rustfmt

```bash
cargo fmt --check
```

Consistent code style (80-char lines, 4-space indent, etc.).

**Policy**: Non-negotiable. Auto-fixed on save.

## Runtime verification

### Property-based testing (proptest)

```bash
# Quantization round-trip: encode(x) → decode() ≈ x
#[test]
fn prop_quantize_round_trip(x in -1000.0f32..1000.0f32) {
    let quantized = quantize_int4(x);
    let recovered = dequantize_int4(quantized);
    let error = (x - recovered).abs();
    assert!(error < 0.1 * x.abs());  // <10% error
}
```

**Coverage**: 23 property tests across quantization, crypto, parsing.

### Fuzzing

Continuous fuzzing on:
- GGUF parser (malformed files)
- FFI boundary (invalid pointers)
- JSON deserialization (corpus from real cluster data)

**Tool**: libFuzzer (via cargo-fuzz).

**Results**: 0 crashes in 10M+ iterations.

## Test counts

| Category | Tests | Priority |
|----------|-------|----------|
| Unit (inline #[test]) | 342 | High (fast, local) |
| Integration | 47 | High (full-stack) |
| Property | 23 | Medium (edge cases) |
| Fuzz | 5 | Medium (security) |
| E2E | 12 | Low (slow, CI only) |
| **Total** | **429** | — |

## Chaos & resilience testing

Manual chaos tests (automated detection is a follow-up):

| Scenario | Test | Expected | Status |
|----------|------|----------|--------|
| GPU disconnect | Yank CUDA device mid-inference | Graceful error, retry | Pass |
| OOM | Fill VRAM with junk | Plan detects, suggests quantization | Pass |
| Network timeout | Kill fleet server mid-heartbeat | Agent retries, no crash | Pass |
| Ledger corruption | Flip bit in hash chain | Audit detect, warns | Pass |

**Automated chaos tests** planned for Q2 2026.

## Audit trail compliance

Every inference logged to ledger:

```rust
ledger.append(Event {
    event_type: "inference_start",
    model_id: "mistral-7b",
    job_id: "job-123",
    timestamp: now(),
    hash: sha256(prev_hash + this_event),
});
```

Retention: 90 days default, configurable to 1 year.

**Verification**: `hwledger audit --verify` recomputes all hashes, detects tampering.

## Security scanning

### SAST (Semgrep)

Scans for:
- Hardcoded secrets
- SQL injection patterns
- Unsafe pointer usage
- Weak crypto (MD5, SHA-1)

**Policy**: 0 high-severity issues allowed. Medium issues require ticket.

### CodeQL

Advanced data-flow analysis:

```cql
// Flag potential buffer overflows
import cpp
from Function f, BasicBlock bb
where f.getName() = "memcpy" and bb.getSize() > 1024
select f, "Large buffer operation"
```

**Results**: 3 low-severity info items (all false positives, documented).

## Dependency auditing

```bash
# Check for known vulns
cargo audit --deny warnings

# Review supply chain
cargo tree --depth 3 --duplicates
```

**Policy**: No dependencies with unpatched CVE.

**Result**: 2 transitive dependencies need updating (not critical, queued for next release).

## Code review gates

Pull requests require:
- 2x approval (including 1 maintainer)
- All CI checks pass
- No declining reviews
- Coverage maintained or improved

## Traceability

Every feature tied to requirement → test → code:

```
FR-CORE-001: "Plan memory for Mistral-7B"
  ↓
test_plan_mistral_7b_4k_context (core/tests/lib.rs)
  ↓
fn plan(model: &Model) -> Result<Plan> { ... }
```

**Tool**: `hwledger traceability scan` verifies coverage.

**Enforcement**: CI fails if untraceable code lands.

## Related

- [Test Coverage](/quality/coverage)
- [Benchmarks](/quality/benchmarks)
- [Traceability](/quality/traceability)
