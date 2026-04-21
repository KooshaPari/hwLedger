# Benchmark Corpus

The full corpus lives in [`crates/hwledger-predict/data/benchmarks.yaml`](https://github.com/KooshaPari/hwLedger/blob/main/crates/hwledger-predict/data/benchmarks.yaml).
Loaded at compile time via `include_str!`, consumed by CLI, FFI, and UI.

> **If a number looks wrong**, edit the YAML. Tests in
> `hwledger-predict` verify every row carries a citation and the file parses.

## Families covered (50+ rows)

- **Llama 3 / 3.1 / 3.2** — 1B, 8B, 70B, 405B — A100, H100, B200, L40S, M3 Max, M3 Ultra
- **Qwen 2.5 / 3** — 1.5B, 7B, 32B, 72B, 30B-A3B (MoE), 235B-A22B (MoE)
- **DeepSeek V2 / V3 / R1 / Coder V2 Lite** — A100, H100, B200, M3 Ultra
- **Mixtral** 8x7B, 8x22B (inc. REAP 40%); **Mistral** 7B, Nemo-12B
- **Gemma** 2-2B, 2-9B, 2-27B, 3-27B
- **Mamba / Mamba-2** 2.7B, 2.8B; **Mamba-2 hybrid** 8B
- **Phi** 3 mini, 3 medium, 3.5 mini, **Phi-4**
- **Cohere** Command-R+, **01.AI** Yi-34B

## Hardware coverage

A100-80G, H100-80G, B200-180G, L40S, M3-Max-128G, M3-Ultra-192G.

## Included variants

- Decoding tricks: `+speculative`, `+medusa`, `+eagle2` Llama-3-70B rows
- Quantized: AWQ, GPTQ int4 variants
- Long context: Llama-3-70B @ 32K, Qwen-2.5-72B @ 128K
- Batched: batch=16, batch=64 throughput rows

## Citation sources used

Top-line arxiv / vendor sources referenced by the corpus:

- [arxiv:2407.21783](https://arxiv.org/abs/2407.21783) — Llama 3 technical report
- [arxiv:2412.15115](https://arxiv.org/abs/2412.15115) — Qwen 2.5 report
- [arxiv:2412.19437](https://arxiv.org/abs/2412.19437) — DeepSeek-V3 report
- [arxiv:2501.12948](https://arxiv.org/abs/2501.12948) — DeepSeek-R1 paper
- [arxiv:2401.04088](https://arxiv.org/abs/2401.04088) — Mixtral of Experts
- [arxiv:2408.00118](https://arxiv.org/abs/2408.00118) — Gemma 2 report
- [arxiv:2312.00752](https://arxiv.org/abs/2312.00752) — Mamba
- [arxiv:2405.21060](https://arxiv.org/abs/2405.21060) — Mamba-2
- [arxiv:2404.14219](https://arxiv.org/abs/2404.14219) — Phi-3
- [arxiv:2412.08905](https://arxiv.org/abs/2412.08905) — Phi-4
- [arxiv:2510.13999](https://arxiv.org/abs/2510.13999) — REAP
- vendor: NVIDIA TensorRT-LLM perf overview, NVIDIA A100/L40S/Blackwell whitepapers
- vendor: Apple MLX Examples

## Adding a row

Append to `benchmarks.yaml`:

```yaml
- model: your/Model-Name
  family: llama           # matches Plan.family downstream
  params_b: 70.0
  hardware: H100-80G
  batch: 1
  seq: 2048
  weight_quant: fp16
  kv_quant: fp16
  decode_tps: 28.0
  ttft_ms: 430.0
  runtime: tensorrt-llm
  source: "arxiv:XXXX.XXXXX"   # or vendor: or hf:
  url: "https://arxiv.org/abs/XXXX.XXXXX"
  notes: "optional"
```

The next `cargo build` picks it up automatically.
