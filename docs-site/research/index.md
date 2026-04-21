# Research

Archived research briefs from the Haiku research swarm that informed hwLedger's architecture and implementation decisions. All 12 briefs are indexed below with direct links to their full analyses.

## All Research Briefs

### Inference Engines & Backends

1. [oMlx Analysis — MLX Fork Strategy](./01-omlx-analysis.md) — Architecture review, fork viability, sidecar integration design.
2. [Inference Engine Matrix — April 2026](./03-inference-engine-matrix.md) — Comprehensive comparison of MLX, mistral.rs, llama.cpp, vLLM, TGI across platforms.

### Subprocess Communication & Integration

3. [MLX IPC Patterns](./02-mlx-ipc-patterns.md) — JSON-RPC over stdio vs protobuf; venv management; signal discipline.

### Memory & Architecture Formulas

4. [KV Cache Formulas — Per-Architecture Derivations](./04-kv-cache-formulas.md) — Complete math breakdown for MHA, GQA, MQA, MLA, SSM, hybrid, attention-sink.

### Model Configuration

5. [Config Ingestion — Model Metadata Loaders](./05-config-ingestion.md) — Pure-Rust loaders for HF Hub, GGUF, safetensors; subprocess fallback for MLX.

### Hardware Telemetry

6. [GPU Telemetry Backends](./06-gpu-telemetry.md) — NVIDIA nvml-wrapper, AMD rocm-smi, Apple Silicon macmon, Intel Arc (deferred).

### Language Bindings & FFI

7. [FFI Survey — Rust ↔ Native Language Bindings](./07-ffi-survey.md) — UniFFI vs cbindgen vs csbindgen vs cxx-qt vs Slint for SwiftUI, WinUI, Qt.

### Fleet Architecture

8. [Fleet Wire Design](./08-fleet-wire-design.md) — Axum + JSON/HTTPS + mTLS; russh + deadpool SSH; Tailscale integration; phenotype-event-sourcing audit log.

### Competitive Analysis

9. [Competitors Survey — Gap Analysis](./09-competitors-survey.md) — HF Accelerate, can-it-run-llm, LM Studio, vLLM internals. hwLedger differentiators.

### Auditing & Cost Tracking

10. [Event Sourcing — Audit Log & Cost Tracking](./10-event-sourcing.md) — phenotype-event-sourcing reuse; SHA-256 hash chains; LedgerError::Integrity tamper detection.

### Additional Research

11. [Competing VRAM Planners — Comparative Analysis](./11-competing-planners.md) — Deep-dive into existing capacity planning tools and their limitations.
12. [UI Journey Harness — VitePress 2 + Vue 3](./12-ui-journey-harness-2026.md) — Component showcase and interaction patterns for hwLedger desktop UIs.

---

## Imported 2026-04

Distilled from a ChatGPT research dump + the `llm_rnd_handoff` bundle. Source transcripts have been removed after distillation; provenance is captured in each brief's frontmatter.

- [VRAM scaling — weights, KV cache, concurrency](/research/imports-2026-04/vram-scaling) — HF "VRAM" is weights-fit, not serving. Hybrid-attention changes KV math.
- [Speculative decoding and API-level draft-verify](/research/imports-2026-04/speculative-decoding) — TiDAR, quant/MoE caveats, what you can't fake.
- [Latent MAS vs Text MAS](/research/imports-2026-04/latent-vs-text-mas) — 50–80% fewer tokens, 3–7× speedup via hidden-state passing.
- [Self-host vs subscription cost](/research/imports-2026-04/self-host-vs-api-cost) — GPU-hour conversion; break-even at ~8 h/day.
- [Autoregressive vs diffusion LMs](/research/imports-2026-04/ar-vs-diffusion) — why black-box APIs can't project diffusion.
- [VPS options for self-hosted workloads](/research/imports-2026-04/vps-options) — Contabo / Hetzner / OVH tiers for 64 GB bare metal.
- [Hetzner auction upgrade envelope](/research/imports-2026-04/hetzner-auction) — what you can and can't upgrade post-purchase.
- [RL and fine-tuning over black-box APIs](/research/imports-2026-04/rl-finetuning) — RFT vs agent-policy RL, what's actually possible.
- [OSS LLM architectures + monthly compute plan](/research/imports-2026-04/architectures-and-compute-plan) — R&D theses, model bands, experiment matrix.

---

## Key Findings Summary

1. **oMlx fork is the right choice**: HTTP sidecar over Python direct call avoids build complexity.
2. **JSON-RPC is proven**: mistral.rs and MCP both use stdin-based JSON-RPC; fallback to protobuf if throughput saturates.
3. **Math accuracy is paramount**: Each attention mechanism has different KV scaling; per-architecture dispatch is non-negotiable.
4. **GPU telemetry is fragmented**: NVIDIA only has a mature API; AMD and Apple require shell-outs.
5. **FFI converges on standards**: UniFFI for Apple, csbindgen for Windows, cxx-qt for Linux.
6. **Axum + mTLS > gRPC**: Simpler protocol stack; mTLS is sufficient at fleet-of-tens scale.
7. **Event sourcing is critical**: Audit trail for cost reconciliation and fleet diagnostics.
8. **KV-cache + MoE awareness is the differentiator**: No competitor handles MLA, hybrid attention, and active-expert math simultaneously.

---

## Research Organization

All research briefs include:

- **Frontmatter**: Title, description, brief ID, date, status, sources.
- **Executive Summary**: One-paragraph overview.
- **Deep Dives**: Technical analysis, code examples, trade-off tables.
- **Recommendations**: Clear guidance for implementation.
- **Sources**: URLs and citations for validation.
- **See Also**: Links to related ADRs and source files.

## Contributing Research

To add a new research brief:

1. Create `docs/research/NN-slug.md` (where NN is the next brief number)
2. Include YAML frontmatter with `title`, `description`, `brief_id`, `date`, `status`, and `sources`
3. Write 300–600 words with markdown headings, tables, and code blocks
4. Reference related ADRs in "See also" section
5. Run `bun run sync:research` to publish to docsite
6. Submit a PR

See [CONTRIBUTING.md](https://github.com/KooshaPari/hwLedger/blob/main/CONTRIBUTING.md) for full guidelines.

---

## External References

Key papers and resources cited across all briefs:

- [Llama 2: Open Foundation and Fine-Tuned Chat Models](https://arxiv.org/abs/2307.09288)
- [Mistral 7B](https://arxiv.org/abs/2310.06825)
- [Mixtral of Experts](https://arxiv.org/abs/2401.04088)
- [Mamba: Linear-Time Sequence Modeling with Selective State Spaces](https://arxiv.org/abs/2312.00752)
- [Efficient Streaming Language Models with Attention Sinks](https://arxiv.org/abs/2309.17453)
- [DeepSeek-V2: Multi-Head Latent Attention](https://arxiv.org/abs/2405.04434)
- [oMlx GitHub](https://github.com/jundot/omlx)
- [mistral.rs GitHub](https://github.com/mistralai/mistral.rs)
- [vLLM GitHub](https://github.com/vllm-project/vllm)
- [UniFFI Documentation](https://mozilla.github.io/uniffi-rs/)
- [cxx-qt (KDAB)](https://kdab.github.io/cxx-qt/)
- [Axum Web Framework](https://github.com/tokio-rs/axum)
