# Probe backend reference — Apple Silicon (Metal / IOKit)

<Shot src="/cli-journeys/keyframes/probe-list/frame-002.png"
      caption="Metal backend — device 0 detected"
      size="small" align="right" />

<Shot src="/cli-journeys/keyframes/probe-watch/frame-001.png"
      caption="probe watch — live refresh"
      size="small" align="left" />

The `hwledger-probe` crate exposes a trait-based `GpuProbe` abstraction with four concrete backends. This page documents the Apple Silicon (Metal / IOKit) backend in detail, including the IOKit key matrix and chip-support table.

## Design goals

- No subprocesses. The Metal backend is pure-Rust IOKit FFI. `macmon` and every other shell-out has been removed (policy: no subprocess ever).
- Fail loudly, not silently. When the kernel does not expose a metric on a given chip / macOS build the probe returns `ProbeError::UnsupportedMetric { chip, macos_version, metric }`. The CLI and the Streamlit probe page render that inline as `Not supported on Apple M1 Pro` rather than a blank dash.

## IOKit surface used

| Purpose | API | Source |
| --- | --- | --- |
| Service matching | `IOServiceMatching("AGXAccelerator")` | `io-kit-sys 0.5` |
| Service enumeration | `IOServiceGetMatchingServices`, `IOIteratorNext` | `io-kit-sys 0.5` |
| Property dictionary | `IORegistryEntryCreateCFProperties` | `io-kit-sys 0.5` |
| CF value unwrap | `CFDictionary`, `CFNumber`, `CFString` | `core-foundation 0.10` |
| HID temperature | `IOHIDEventSystemClient*` (framework-level FFI) | hand-rolled extern |
| Static host info | `sysctlbyname("hw.memsize" / "machdep.cpu.brand_string" / "kern.osproductversion")` | `libc 0.2` |

The HID event system calls (`IOHIDEventSystemClientCreate`, `IOHIDEventSystemClientSetMatching`, `IOHIDEventSystemClientCopyServices`, `IOHIDServiceClientCopyProperty`, `IOHIDServiceClientCopyEvent`, `IOHIDEventGetFloatValue`) are not exposed by `io-kit-sys`, so the backend links `IOKit.framework` directly and declares them in an `extern "C"` block.

## PerformanceStatistics key matrix

The backend probes an ordered list of keys for each metric and returns the first one that materialises a number. This keeps the code robust against Apple's rename cadence between chip generations and major macOS releases.

| Metric | Candidate keys (in order) |
| --- | --- |
| Utilization | `Device Utilization %` -> `GPU Activity(%)` -> `Device Utilization` -> `GPU Utilization` |
| VRAM used | `In use system memory` -> `Alloc system memory` -> `GPU Core Utilization` -> `inUseSystemMemory` |
| Power (mW) | `Total Power` -> `GPU Power(mW)` -> `gpuPower` |

VRAM used is subtracted from `sysctlbyname("hw.memsize")` to produce free unified memory. Power is reported in milliwatts by the kernel; the backend converts to watts.

## Temperature sensors

GPU die temperature is sampled from IOHID services matching `PrimaryUsagePage = 0xFF00` (kHIDPage_AppleVendor) and `PrimaryUsage = 0x0005` (kHIDUsage_AppleVendor_TemperatureSensor).

The backend copies every matching service's `Product` name and keeps only GPU-family sensors, rejecting battery / NAND / gas-gauge zones. Accepted name fragments (case-insensitive): `TG`, `GPU`, `pACC`, `eACC`, `TPU`, `tdie`, and M1-firmware cluster zones `PMU TP<N>g` / `PMU TP<N>s`. The maximum reading across matching sensors is returned, capped to the sanity window 0 to 150 C.

If no sensor matches (or every sensor reads 0 / NaN) the probe returns `UnsupportedMetric { metric: "temperature" }`.

## Chip support matrix

| Chip | macOS | Util | VRAM used | Temperature | Power |
| --- | --- | --- | --- | --- | --- |
| Apple M1 | 14 to 26 | Yes | Yes | Yes (TP*g / TP*s die zones) | Not supported (needs IOReport) |
| Apple M1 Pro | 14 to 26 | Yes | Yes | Yes (max over tdie + TP*s / TP*g) | Not supported |
| Apple M1 Max | 14 to 26 | Yes | Yes | Yes | Not supported |
| Apple M1 Ultra | 14 to 26 | Yes | Yes | Yes | Not supported |
| Apple M2 / Pro / Max / Ultra | 14 to 26 | Yes | Yes | Yes (Tg* / TPU* sensors) | Yes (`Total Power`) |
| Apple M3 / Pro / Max | 14 to 26 | Yes | Yes | Yes | Yes (`Total Power`) |
| Apple M4 / Pro / Max | 15 to 26 | Yes | Yes | Yes | Yes (`Total Power`) |

M1-family chips do not expose a `Total Power` key inside `PerformanceStatistics`. Accurate power on those SKUs requires the private `IOReport` framework (a follow-up, still pure-Rust and subprocess-free). Until then the backend surfaces a clean `UnsupportedMetric` rather than a fake zero.

## Fallback semantics

- VRAM used: if every candidate key is absent the backend returns `free_vram = total_vram` (rather than erroring) so the UI still shows a usable VRAM bar on unknown firmwares. `utilization`, `temperature`, and `power` always surface `UnsupportedMetric` on absence; they must be honest.
- `UnsupportedMetric` contract: carries `{ chip, macos_version, metric }` so the UI can render a user-actionable message instead of a dash.
- Caching: `CachedProbe` wraps the Metal probe but delegates per-metric trait calls directly to the inner probe, so one `UnsupportedMetric` (for example power on an M1) never poisons a sibling read (temperature on the same M1). Coherent snapshots are still available explicitly via `CachedProbe::snapshot(device_id)`.

## Testing

Unit tests run everywhere (they use `MockBackend`, no real hardware). A single `#[ignore]`d integration test (`metal_probe_live_reads_real_metrics`) runs against the real host when invoked with `cargo test -p hwledger-probe -- --ignored`.

```bash
cargo test -p hwledger-probe                                # unit tests
cargo test -p hwledger-probe metal_probe_live -- --ignored  # live, macOS only
```
