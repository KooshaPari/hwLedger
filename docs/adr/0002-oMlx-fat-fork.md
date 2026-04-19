# ADR 0002 — oMlx fat fork as Apple-Silicon inference sidecar

Constrains: FR-INF-001, FR-INF-002, FR-INF-003, FR-INF-004

Date: 2026-04-18
Status: Accepted

## Context

Apple Silicon peak throughput lives in MLX, not in llama.cpp's Metal backend or mistral.rs's Metal path. The most mature MLX-based local-inference server is **oMlx** (`jundot/omlx`, Apache-2.0, 10.6k★, active as of v0.3.6 Apr 2026). Its differentiating feature is **paged SSD caching of KV blocks**, which drops TTFT from 30–90 s to 1–3 s for agent-loop workloads — a direct match for hwLedger's planner-and-runner pitch.

Three handling options were considered:

1. **Slim-fork**: drop PyObjC menubar + venvstacks build, keep FastAPI + mlx-lm/mlx-vlm. Fork into `KooshaPari/phenotype-omlx`.
2. **Upstream HTTP-sidecar unmodified**: pin a commit; submit upstream PRs as needed.
3. **Skip oMlx entirely**; drive `mlx-lm` via our own JSON-RPC protocol; reimplement SSD KV caching ourselves.
4. **Fat fork** (chosen): superset of 1 + 3. Fork the full oMlx codebase into our org, retain everything (incl. PyObjC menubar as an optional component behind a feature flag), and add our own JSON-RPC stdio protocol alongside the existing FastAPI HTTP surface. Extend freely with hwLedger-specific features (KV-quant knobs, deterministic benchmarks, per-layer memory reporting).

## Decision

Adopt **option 4 (fat fork)**. Forked repo: `KooshaPari/phenotype-omlx` (Apache-2.0, dual-remote: our origin + upstream tracking for selective cherry-picks).

Sidecar boundary: parent Rust process spawns the Python sidecar under a `uv`-managed pinned venv. Two parallel IPC surfaces:

- **FastAPI HTTP** (inherited from upstream): OpenAI-compat + Anthropic-compat at `localhost:8000`. Used by external agents already wired to those APIs.
- **JSON-RPC over stdio** (our addition): typed `hwledger-mlx-sidecar` protocol for bidirectional token streaming, memory-introspection RPCs, benchmark hooks, and lifecycle control. Length-prefixed protobuf reserved as a future fallback if throughput exceeds JSON-RPC's ceiling.

Upstream-sync cadence: weekly rebase attempt; we keep divergent patches in a numbered series (`sidecars/omlx-fork/patches/`) to ease future re-forks.

## Consequences

- Ongoing Python + PyObjC + venvstacks maintenance burden. Accepted because the SSD-paged KV-cache is a killer feature we do not want to reimplement in Rust from scratch.
- Fork scope is large; initial slim would have been ~30 % of the codebase. We pay the full-scope maintenance tax in exchange for option value.
- Upstream contributions (`jundot/omlx`): we will submit PRs for non-hwledger-specific improvements to reduce long-term divergence.
- Deep extensibility: we are free to add KV-quant dials, layerwise memory reports, and benchmark modes upstream cannot accept.

## Rejected alternatives

- Slim-fork: less maintenance but forecloses future feature extensions we want.
- Upstream-only: least effort but upstream PRs are too slow for our velocity.
- Own-protocol-only: most code, no SSD-KV feature parity.

## References

- Upstream: https://github.com/jundot/omlx (v0.3.6, Apr 2026)
- Research brief: oMlx (archived in `docs/research/01-omlx.md`).
- Research brief: MLX IPC patterns (archived in `docs/research/02-mlx-ipc.md`).
