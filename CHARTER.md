# hwLedger — Project Charter

## Mission

Give a single operator with a fragmented fleet of consumer GPUs, Apple laptops, and cheap cloud rentals the same capacity-planning, dispatch, and audit tooling that enterprise LLMops teams pay for — as an Apache-2.0 desktop app + agent pair.

## Principles

1. **Math over marketing.** Every number in the UI is traceable to a formula in `hwledger-core::math` and a field in a canonical `config.json`. No hand-waved "this model needs X GB" labels.
2. **Architecture-correct.** MoE, MLA, hybrid attention, and SSM get first-class formulas — not a dense-model fallback with a fudge factor.
3. **Rust-first.** The core, probe, ledger, agent, and server are Rust. UI toolkits are chosen per-OS for polish, not unified at the cost of native feel.
4. **Wrap / fork / borrow, don't hand-roll.** Follow the Phenotype wrap-over-handroll mandate. Reuse `phenotype-event-sourcing`, `phenotype-health`, `phenotype-cache-adapter`, etc. Fork oMlx; embed mistral.rs; bind nvml-wrapper.
5. **Fail loudly.** No silent fallbacks. If the probe can't read AMD telemetry on Windows, the UI shows "unsupported, shelling out to rocm-smi failed: <reason>" — not "0 GB free".
6. **Ledger everything.** Every dispatch, plan, and config change is an event in the hash-chained audit log. Audits are deterministic from the log alone.
7. **Hobbyist ergonomics.** One-command install per platform. No Kubernetes. No mandatory cloud account. Offline-first.

## Scope

### In

- Capacity planning for dense + MoE + MLA + hybrid + SSM models.
- Live VRAM telemetry across NVIDIA (NVML), AMD (rocm-smi shell-out), Apple Silicon (macmon shell-out), Intel (best-effort).
- Local inference via MLX sidecar (macOS).
- Fleet ledger: local + tailnet + rentals (Vast.ai, RunPod, Lambda, Modal).
- Cost-aware dispatch suggestions (spot price × fit score).
- SSH-exec dispatch; `phenotype-event-sourcing` audit log.
- Per-OS native GUIs: SwiftUI, WinUI 3, Qt 6, Slint.

### Out

- Training or fine-tuning.
- Multi-tenant SaaS.
- Reimplementing vLLM / TGI / SGLang server internals.
- Mobile (iPadOS / iOS) in MVP; deferred to post-v1 (Swift FFI enables it for free later).
- Proper job queueing (deferred to v2).

## Stakeholders

- Sole operator (@kooshapari). Hobbyist fleet, enterprise-bones expectations.
- Phenotype org cross-project reuse: promotes `hwledger-probe`, `hwledger-arch`, `hwledger-mlx-sidecar` to shared crates on demand.
- Future open-source contributors (Apache-2.0 from day 1).

## Success criteria (v1)

1. macOS app ships with all 6 screens, reconciles predicted vs. actual VRAM within ±200 MB for 10 canonical models.
2. Fleet agent + server run stably on 3 heterogeneous hosts for 72 h without restart.
3. Cost estimator matches Vast.ai / RunPod billing within 5 % across a 24 h dispatch window.
4. MLA / MoE / hybrid-attention formulas validate against vLLM and llama.cpp reported numbers for DeepSeek-V3, Qwen3-MoE, Mixtral, and Qwen3.6-A3B.

## Non-goals / anti-patterns

- No Electron.
- No Chromium-wrapped docs site bundled in the app.
- No "might as well add a chat UI" scope creep before v1 ships.
- No bypass of workspace quality gates (trufflehog, clippy -D warnings, cargo fmt --check).
