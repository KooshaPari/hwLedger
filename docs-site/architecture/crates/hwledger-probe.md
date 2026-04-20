---
title: hwledger-probe
description: GPU telemetry abstraction over NVML, rocm-smi, Metal, Intel Level Zero, and sysinfo.
---

# hwledger-probe

**Role.** Discovers GPUs and samples live telemetry (VRAM used/total, utilization, temperature) across NVIDIA (NVML), AMD (rocm-smi), Apple (metal-rs), Intel, and a sysinfo fallback.

## Why this crate

The planner's "will this model fit?" question reduces to comparing a computed KV-cache + weight footprint against real, current free VRAM. Reading that figure from four vendor APIs without a trait abstraction would duplicate the boxed-dyn plumbing and caching in every consumer (CLI, server, agent). This crate is a thin trait + four concrete probes + a TTL-cached decorator.

Rejected: shelling out to `nvidia-smi` / `rocm-smi` from the CLI. Rejected because (a) fleet agents run as non-interactive daemons and forking every second is wasteful, (b) `metal-rs` has no CLI analog, and (c) subprocess parsing is brittle across vendor driver versions.

**Belongs here:** GPU enumeration, telemetry sampling, TTL caching of expensive probes.
**Does not belong here:** placement decisions (that's `hwledger-server::rentals`), model ingestion, any UI.

## Public API surface

| Type | Name | Stability | Notes |
|------|------|-----------|-------|
| trait | `GpuProbe: Send + Sync` | stable | Object-safe so `Vec<Box<dyn GpuProbe>>` is the canonical handle |
| struct | `Device` | stable | Vendor, PCIe id, VRAM, driver version |
| struct | `NvidiaProbe` | stable | NVML via `nvml-wrapper` |
| struct | `AmdProbe` | stable | rocm-smi JSON output |
| struct | `MetalProbe` | stable | Apple Metal via `metal-rs` (macOS only) |
| struct | `IntelProbe` | MVP | Level Zero, limited driver coverage |
| struct | `CachedProbe` | stable | TTL decorator, see `default_ttl()` |
| enum | `ProbeError` | stable | `VendorUnavailable`, `Timeout`, `Parse` |
| fn | `detect()` | stable | Returns every probe that initialized successfully |

Intel support is marked MVP: discovery works but utilization/temperature are best-effort on current Intel GPU drivers.

## When to reach for it

1. **`hwledger probe` CLI subcommand** — enumerate and print all local accelerators.
2. **Fleet agent heartbeat** — `CachedProbe::sample()` feeds `TelemetrySnapshot` in `hwledger-fleet-proto`.
3. **Planner "can this fit?" check** — compare live `Device.free_mem` against computed KV bytes.

## Evolution

| SHA | Note |
|-----|------|
| `db67d58` | Bootstrap with NVIDIA probe only |
| `812e526` | `feat(p1,p2): Wave 2 ... AMD/Metal/Intel probes` — four-vendor matrix completed |
| `97fcc68` | `feat(p3,p5,test,docs): Wave 9 — WP26 VHS CLI pipeline + WP32 traceability` |
| `c7a2474` | `feat(perf): add criterion benchmarks to all hot-path crates` — probe path benchmarked to guard against regressions in the telemetry loop |
| `ec1f8bf` | `feat(release): ship v0.1.0-alpha + coverage uplift` |

**Size.** 1,462 LOC, 42 tests. Vendor-specific modules gated behind Cargo features to keep non-macOS builds from linking Metal.

## Design notes

- `CachedProbe` wraps any `GpuProbe` with a TTL; `default_ttl()` is tuned for agent heartbeats (seconds, not milliseconds).
- `detect()` swallows per-vendor init errors and returns the set that succeeded. The server logs the failures — this is deliberately the one place in the stack where partial success is acceptable because a dual-vendor box is the common case.
- All probes are `Send + Sync` so they can live in `AppState` behind an `Arc`.

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-probe)
- [ADR-0005: Shared crate reuse](/architecture/adrs/0005-shared-crate-reuse)
