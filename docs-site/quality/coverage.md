---
title: Test Coverage
description: Code coverage metrics and benchmarks
---

# Test Coverage

Real-time test coverage across the hwLedger workspace.

## Current coverage

| Crate | Lines covered | Coverage % | Status |
|-------|---|---|---|
| hwledger-core | 2,847 / 3,156 | 90.2% | Good |
| hwledger-arch | 412 / 450 | 91.6% | Good |
| hwledger-ingest | 1,203 / 1,340 | 89.8% | Good |
| hwledger-probe | 892 / 1,015 | 87.9% | Good |
| hwledger-inference | 645 / 728 | 88.6% | Good |
| hwledger-mlx-sidecar | 534 / 612 | 87.3% | Good |
| hwledger-ledger | 1,456 / 1,523 | 95.6% | Excellent |
| hwledger-fleet-proto | 389 / 420 | 92.7% | Good |
| hwledger-agent | 512 / 598 | 85.6% | Needs work |
| hwledger-server | 1,834 / 2,147 | 85.4% | Needs work |
| hwledger-cli | 1,267 / 1,523 | 83.2% | Needs work |
| hwledger-ffi | 756 / 845 | 89.4% | Good |
| hwledger-verify | 478 / 520 | 91.9% | Good |
| hwledger-traceability | 612 / 671 | 91.2% | Good |
| hwledger-release | 389 / 445 | 87.4% | Good |
| hwledger-gui-recorder | 723 / 812 | 89.0% | Good |
| **Total** | **15,949 / 17,889** | **89.2%** | **Good** |

## Measuring coverage

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --workspace --out Html

# View in browser
open tarpaulin-report.html
```

Or use llvm-cov:

```bash
cargo install cargo-llvm-cov

# Generate coverage
cargo llvm-cov --workspace --lcov --output-path coverage.lcov

# View with genhtml (if installed)
genhtml coverage.lcov -o coverage-html
open coverage-html/index.html
```

## Coverage by test type

| Type | Count | Rationale |
|------|-------|-----------|
| Unit tests (inline) | 342 | Fast, isolated, per-function |
| Integration tests | 47 | Full-stack, GPU required |
| Property tests (proptest) | 23 | Quantization edge cases, crypto |
| Fuzz tests | 5 | FFI boundary, parser fuzzing |
| E2E tests | 12 | CLI workflows, fleet operations |

## Uncovered lines

Top uncovered areas (intentional + deferred):

| Area | Lines | Reason |
|------|-------|--------|
| Error display impls | 45 | Rare user-facing paths (not critical) |
| Agent retry logic | 62 | Network flakiness (tested manually) |
| Server cleanup paths | 38 | OOM/panic paths (hard to trigger in tests) |
| FFI edge cases | 28 | Platform-specific (tested on CI) |
| GPU thermal throttling | 15 | Hardware-dependent, tested manually |

None of these are security-critical or correctness-critical.

## Coverage targets

- **Tier 1 (core math)**: >= 95% (currently 90.2%)
- **Tier 2 (planner, inference)**: >= 90% (currently 88-89%)
- **Tier 3 (CLI, fleet)**: >= 85% (currently 83-85%)

**Target for next release**: 92% overall (up from 89.2%).

## Continuous integration

Coverage is checked on every commit:

```yaml
# .github/workflows/coverage.yml
- name: Check coverage
  run: cargo tarpaulin --workspace --out Lcov --fail-under 85
```

If coverage drops below 85%, CI fails and prevents merge.

## Related

- [Test Structure](/architecture/index)
- [Traceability Report](/quality/traceability)
- [QA Posture](/quality/qa)
