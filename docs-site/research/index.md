# Research

Archived research briefs from the Haiku research swarm that informed hwLedger's architecture and implementation decisions.

## Research Topics

All research briefs are available in `docs/research/`:

### Inference Engines

- **oMlx (MLX fork)**: 10.6K⭐, Apache-2.0, active. Python-based with SSD-paged KV cache achieving 30–90s TTFT and 1–3s for agent loops.
- **Inference engine matrix (Apr 2026)**: MLX for Apple peak, mistral.rs for Rust-native with MoE+GGUF support, llama.cpp as universal fallback, vLLM/TGI for remote.

### IPC & Subprocess Communication

- **MLX IPC patterns**: JSON-RPC over stdio proven in mistral.rs + MCP + Ollama. Fallback to length-prefixed protobuf if throughput saturates.
- **Dependency pinning**: uv for reproducible Python venv pinning; parent-managed subprocess with `signal_hook` for SIGTERM.

### KV Cache & Math

- **Architecture-keyed KV formulas**: Derivation for MHA, GQA, MQA, MLA, sliding window, SSM/Mamba, hybrid attention, sink tokens.
- **Per-layer breakdown**: MoE treats resident vs active parameters separately; hybrid models check per-layer attention type.

### Config Ingestion

- **Pure-Rust loaders**: hf-hub for HF Hub, gguf-rs-lib for GGUF, safetensors for safetensors.
- **Subprocess fallback**: MLX .npz inspection via Python subprocess.
- **REST APIs**: Ollama/LM Studio/vLLM CLI flag endpoints.

### GPU Telemetry

- **NVIDIA**: nvml-wrapper is mature and canonical.
- **AMD**: rocm-smi --json (no production-grade crate).
- **Apple Silicon**: macmon --json (no public Metal memory API).
- **Intel Arc**: Vacuum (deferred).

### FFI & Language Bindings

- **UniFFI + cargo-xcframework**: Preferred for SwiftUI (Mozilla/Signal/1Password converged).
- **C# .NET 9+ via csbindgen**: Windows WinUI 3 native AOT + Velopack auto-update.
- **cxx-qt (KDAB)**: Qt 6.6+ active, LGPL dynamic-link compatible with Apache-2.0.
- **Slint**: Credible escape hatch if cxx-qt integration proves painful.

### Fleet Architecture

- **Axum + mTLS**: Preferred over gRPC for fleet-of-tens scale. rustls+rcgen for auto-generated certs (no PKI).
- **russh + deadpool**: Agentless SSH with connection pooling.
- **Vast/RunPod/Lambda/Modal**: reqwest async HTTP + vendor SDKs.
- **phenotype-event-sourcing**: Reusable module for audit log.

### Competitors & Differentiation

- **Gap analysis**: HF Accelerate, can-it-run-llm, LM Studio all underweight KV cache and overweight MoE.
- **hwLedger differentiator**: KV-cache-and-MoE-aware math plus slider UX over live per-layer breakdown.

## Key Findings

1. **oMlx fork is the right choice**: HTTP sidecar over Python direct call avoids build complexity.
2. **JSON-RPC is proven**: mistral.rs and MCP both use stdin-based JSON-RPC; fallback to protobuf if throughput saturates.
3. **Math accuracy is paramount**: Each attention mechanism has different KV scaling; per-architecture dispatch is non-negotiable.
4. **GPU telemetry is fragmented**: NVIDIA only has a mature API; AMD and Apple require shell-outs.
5. **FFI converges on standards**: UniFFI for Apple, csbindgen for Windows, cxx-qt for Linux.
6. **Axum + mTLS > gRPC**: Simpler protocol stack; mTLS is sufficient at fleet-of-tens scale.
7. **Event sourcing is critical**: Audit trail for cost reconciliation and fleet diagnostics.

## Research Briefs

All full briefs are linked from `docs/research/*.md`. Start with:

- `01-omlx-analysis.md` — Deep dive into oMlx architecture and fork strategy
- `02-mlx-ipc-patterns.md` — IPC trade-offs and JSON-RPC design
- `03-inference-engine-matrix.md` — Comparison of MLX, mistral.rs, llama.cpp, vLLM, TGI
- `04-kv-cache-formulas.md` — Derivations per architecture
- `05-config-ingestion.md` — HF/GGUF/safetensors loaders
- `06-gpu-telemetry.md` — NVIDIA/AMD/Apple Silicon probes
- `07-ffi-survey.md` — UniFFI, csbindgen, cxx-qt comparison
- `08-fleet-wire-design.md` — Axum vs gRPC, mTLS, agentless SSH
- `09-competitors-survey.md` — Analysis of existing VRAM calculators
- `10-event-sourcing.md` — Audit log and cost tracking
- `11-shared-crate-reuse.md` — Phenotype org module extraction
- `12-ui-journey-harness.md` — VitePress 2 + Vue 3 components

## Contributing Research

To add a research brief:

1. Create `docs/research/NN-description.md` with findings
2. Include sources, code examples, and conclusions
3. Reference in this index
4. Submit a PR

See [CONTRIBUTING.md](https://github.com/KooshaPari/hwLedger/blob/main/CONTRIBUTING.md).

## References

All briefs include URLs and citations. Key external sources:

- [Llama 2: Open Foundation and Fine-Tuned Chat Models](https://arxiv.org/abs/2307.09288)
- [Mistral 7B](https://arxiv.org/abs/2310.06825)
- [Mixtral of Experts](https://arxiv.org/abs/2401.04088)
- [Mamba: Linear-Time Sequence Modeling with Selective State Spaces](https://arxiv.org/abs/2312.00752)
- [Efficient Streaming Language Models with Attention Sinks](https://arxiv.org/abs/2309.17453)
- [oMlx GitHub](https://github.com/jundot/omlx)
- [mistral.rs GitHub](https://github.com/mistralai/mistral.rs)
- [vLLM GitHub](https://github.com/vllm-project/vllm)
