# Research Briefs

Archived Haiku-agent research briefs backing hwLedger's architecture decisions. Each brief answers a focused question with cited sources; the live sub-agent transcripts live at `/private/tmp/claude-501/.../tasks/` and are summarised here.

| # | Topic | ADR backlink | Date |
|---|-------|--------------|------|
| 01 | oMlx — what it is, fork viability, competitive landscape | ADR-0002 | 2026-04-18 |
| 02 | Rust ↔ MLX subprocess IPC patterns | ADR-0002 | 2026-04-18 |
| 03 | Inference engine landscape (mistral.rs / candle / llama.cpp / vLLM / TGI / SGLang / ExLlamaV2 / MLX / TensorRT-LLM / Ollama) | PLAN §7 | 2026-04-18 |
| 04 | KV / state formulas per architecture (MHA, GQA, MQA, MLA, hybrid, sliding, SSM, sinks, quantisation) | PLAN §5.1, ADR-0004 (pending) | 2026-04-18 |
| 05 | Model config ingestion (HF Hub, GGUF, safetensors, MLX, vLLM CLI, Ollama, LM Studio) | FR-PLAN-001 | 2026-04-18 |
| 06 | Rust GPU telemetry (nvml-wrapper, rocm-smi, macmon, Level-Zero) | FR-TEL-001 | 2026-04-18 |
| 07 | Rust ↔ Swift FFI (UniFFI + cargo-xcframework + Swift Package) | ADR-0001 | 2026-04-18 |
| 08 | Rust ↔ WinUI FFI (csbindgen + C# .NET 9 + WinUI 3 + Velopack) | ADR-0001 | 2026-04-18 |
| 09 | Rust ↔ Qt 6 FFI (cxx-qt + QML, Slint escape hatch) | ADR-0001 | 2026-04-18 |
| 10 | Fleet wire — Axum + mTLS vs. gRPC; russh; Tailscale; rentals | ADR-0003 | 2026-04-18 |
| 11 | Competing capacity planners (HF Accelerate, can-it-run-llm, LM Studio, vLLM internals) | PLAN §3 #11 | 2026-04-18 |

## How to re-run

All briefs were produced by Haiku agents in a single parallel swarm. See `PLAN.md` §3 for one-line summaries. To re-run a brief, spawn a Haiku-model general-purpose agent with the original prompt (reconstructable from the headings in each brief file).

## Notes

- Brief 11 was also dumped as `RESEARCH_VRAM_PLANNERS.md` in the repo root by the agent; that file will be moved here in the next housekeeping pass.
- The `chatdocs.md` and `chatdocs2.md` files are the original product conversation that motivated this research. `chatdocs2.md` is a duplicate of `chatdocs.md` (same bytes) and will be removed.
