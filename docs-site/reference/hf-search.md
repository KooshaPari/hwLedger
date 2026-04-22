# Hugging Face search

Traces to: **FR-HF-001**, FR-PLAN-003

hwLedger can query the Hugging Face Hub directly to discover models, pull their
`config.json`, and plan memory — no manual download required.

<!-- SHOT-MISMATCH: caption="plan —hf resolves HF model ID and continues" expected=[plan,--hf,resolves,model,continues] matched=[] -->
<Shot src="/cli-journeys/keyframes/plan-hf-resolve/frame-001.png"
      caption="plan --hf resolves HF model ID and continues"
      size="small" align="right" />

<RecordingEmbed tape="plan-hf-resolve" caption="resolver: same input surface in the CLI that the Streamlit app wraps" />

<RecordingEmbed tape="streamlit-hf-search" caption="Streamlit HF search: quick-pick band + type-to-filter + click-to-use" />

<!-- SHOT-MISMATCH: caption="Resolver fallback: full URL → repo id + revision extracted" expected=[resolver,fallback,full,url,repo,revision,extracted] matched=[] -->
<Shot src="/cli-journeys/keyframes/plan-hf-resolve/frame-003.png"
      caption="Resolver fallback: full URL → repo id + revision extracted"
      size="small" align="left" />

## Anonymous by default

Public endpoints on `https://huggingface.co/api/models` work without any
credential. hwLedger runs **anonymous by default**.

| Access | Source | Rate limit (approx.) |
|--------|--------|----------------------|
| Anonymous (default) | IP-scoped | ~1000 req / 5 min |
| Authenticated (`HF_TOKEN`) | Per-token | ~100k req / day |

A token is only required for **gated** or **private** repos. Pass it via
`--hf-token <TOKEN>` or the `HF_TOKEN` env var. Tokens are never logged or
cached.

## Subcommands

### `hwledger search query <QUERY>`

Full-text search with filters.

```bash
hwledger search query "llama 4" --limit 5
hwledger search query --library gguf --sort trending --limit 10
hwledger search query "phi" --pipeline-tag text-generation --author microsoft
hwledger search query "deepseek" --json | jq '.[].id'
```

Columns: `repo-id | params | downloads | likes | library | last-modified`.

### `hwledger search pull <REPO_ID>`

Fetch a model's `config.json` and cache it. Emits the config to stdout; pass
`--print` for a pretty-printed version.

```bash
hwledger search pull deepseek-ai/DeepSeek-V3
hwledger search pull meta-llama/Llama-3.1-8B --revision main --print
```

### `hwledger search plan <REPO_ID>`

One-shot: fetch config, run the planner, print the result.

```bash
hwledger search plan deepseek-ai/DeepSeek-V3 --seq 8192 --users 2
hwledger search plan meta-llama/Llama-3.1-8B --seq 4096 --export vllm
hwledger search plan Qwen/Qwen2.5-7B --kv-quant fp8 --weight-quant int4 --json
```

`--export {vllm,llama-cpp,mlx}` swaps the table for framework-ready flags.

## Caching and offline mode

Responses cache to `~/.cache/hwledger/hf/<repo-id>/*.json` with a 24-hour TTL:

- `search/<fingerprint>.json` — search results
- `<repo-id>/card.json` — full card
- `<repo-id>/config@<rev>.json` — model `config.json`

On network failure the cache is used automatically (with a tracing warning).
Pass `--offline` to force cache-only — any miss errors loudly rather than
silently degrading.

```bash
hwledger search query "llama" --offline        # cache-only
hwledger search plan mistralai/Mistral-7B-v0.1 --offline
```

## Errors

Errors are loud and actionable:

- Gated model without token →
  `model is gated or private: Pass --hf-token <TOKEN> or set HF_TOKEN...`
- Rate-limited anonymous →
  `Hugging Face rate limit hit. Anonymous IPs share ~1000 req/5min. Set HF_TOKEN for ~100k req/day.`
- Offline cache miss → `offline mode: no cached data for <key>`

## See also

- [CLI reference](./cli.md)
- [Getting started](../getting-started/quickstart.md)
