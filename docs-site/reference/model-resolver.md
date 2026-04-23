---
title: Model resolver
description: How hwLedger dispatches `plan` inputs — gold fixtures, local configs, HF repo ids, HF URLs, free-text queries.
---

# Model resolver

Traces to: **FR-HF-001**

<!-- SHOT-MISMATCH: caption="Resolver accepts an HF repo id and dispatches" expected=[resolver,accepts,repo,dispatches] matched=[] -->
<Shot src="/cli-journeys/keyframes/plan-hf-resolve/frame-001.png"
      caption="Resolver accepts an HF repo id and dispatches"
      size="small" align="right" />

<!-- SHOT-PENDING: capture ambiguous-input disambiguation prompt -->

Every client of hwLedger — the CLI, the Streamlit app, the SwiftUI GUI, and the
C ABI used by third-party bindings — funnels user input through a single
**model resolver**. The resolver is a pure function from a string to one of a
small set of dispatches: a golden fixture, a local `config.json` on disk, a
Hugging Face repo id, a Hugging Face URL (with optional revision), an
ambiguous free-text query, or an empty-input error. This page is the canonical
rule matrix — if a new input form appears, it goes here first and then into
the Rust resolver crate.

## Rule matrix

| Input | Dispatch | Example |
|---|---|---|
| `gold:<name>` | `GoldenFixture(tests/golden/<name>.json)` | `gold:deepseek-v3` |
| absolute `.json` path | `LocalConfig(<path>)` | `/path/to/config.json` |
| relative `.json` path (exists on disk) | `LocalConfig(<path>)` | `tests/golden/deepseek-v3.json` |
| `org/repo-id` | `HfRepo { repo_id, revision: None }` | `deepseek-ai/DeepSeek-V3` |
| HF URL (`https://huggingface.co/<org>/<repo>[/tree/<rev>]`) | `HfRepo { repo_id, revision }` | `https://huggingface.co/meta-llama/Llama-3-70B/tree/main` |
| free text (no slash, no protocol) | `AmbiguousQuery { candidates: [] }` | `deepseek v3` |
| empty string / whitespace | `Error::Empty` | `""` |
| non-HF URL | `AmbiguousQuery { candidates: [] }` | `https://github.com/foo/bar` |

Dispatch order is strict: `gold:` prefix wins over path-like inputs, absolute
path beats HF-shaped strings, and HF URLs are matched before free-text
fallback. Ambiguous inputs never silently fall through to a lookup — they
surface a loud error so callers can prompt the user to disambiguate.

## Four example invocations

### Via CLI

```bash
# 1. Gold fixture (offline, deterministic — used in CI and tapes).
hwledger plan gold:deepseek-v3 --seq 32K

# 2. Local config.json on disk.
hwledger plan tests/golden/deepseek-v3.json --seq 32768 --users 2

# 3. HF repo id — fetches config.json via the 24h cache.
hwledger plan deepseek-ai/DeepSeek-V3 --seq 4K

# 4. Full HF URL (revision extracted from the `/tree/<rev>` segment).
hwledger plan https://huggingface.co/meta-llama/Llama-3-70B --seq 8K
```

<RecordingEmbed tape="streamlit-hf-search" kind="streamlit" caption="Streamlit HF search → click row → auto-redirect to Planner with model pre-filled (primary resolver surface)" />

<RecordingEmbed tape="streamlit-planner" kind="streamlit" caption="Streamlit Planner: resolver dispatches the pre-filled model through the same pure function used by every client" />

<RecordingEmbed tape="plan-hf-resolve" kind="cli" caption="CLI fallback — same resolver dispatching repo id, URL, and gold fixture shortcut (useful for scripting)" />

<!-- SHOT-MISMATCH: caption="Dispatch badge: `hf-repo` for a bare repo id" expected=[dispatch,badge,hf-repo,bare,repo] matched=[] -->
<Shot src="/cli-journeys/keyframes/plan-hf-resolve/frame-002.png"
      caption="Dispatch badge: `hf-repo` for a bare repo id"
      size="small" align="left"
      :annotations='[{"bbox":[60,120,280,24],"label":"hf-repo","color":"#f9e2af","position":"top-left"}]' />

<Shot src="/cli-journeys/keyframes/plan-hf-resolve/frame-004.png"
      caption="Dispatch badge: `gold` for a fixture shortcut"
      size="small" align="right" />

### Via Streamlit

The web app exposes a single text input labelled "Model". It forwards the raw
string to the resolver and renders the dispatch kind as a badge below the
input (`gold`, `local`, `hf-repo`, `hf-url`, `ambiguous`). Ambiguous inputs
trigger a disambiguation picker that lists HF search results.

### Via SwiftUI

The macOS GUI's planner window has a model field backed by the same resolver
(via the `hwledger_core` FFI). The autocomplete menu surfaces `gold:*`
fixtures first, then recent HF repos, then search candidates for free text.

### Via FFI

Third-party bindings (`hwledger-core` C ABI) call
`hwledger_resolve_model(const char *input, ResolvedModel *out)` and receive a
tagged union with the dispatch kind plus any extracted revision or candidate
list. The ABI mirrors the rule matrix above verbatim.

## Error cases

The resolver fails loudly; no silent degradation.

| Input | Error | Exit / kind |
|---|---|---|
| `""` or whitespace-only | `resolver: empty input` | `2` (usage) |
| `gold:does-not-exist` | `resolver: golden fixture 'does-not-exist' not found in tests/golden/` | `2` |
| `/abs/path/missing.json` | `resolver: local config not found: <path>` | `2` |
| `org/repo-id` that HF returns 404 for | `HF 404: <repo_id> (check spelling, or pass --hf-token for private repos)` | `4` (network) |
| gated HF repo without token | `model is gated or private: Pass --hf-token <TOKEN> or set HF_TOKEN` | `5` (auth) |
| free text with zero candidates | `resolver: ambiguous query '<input>' — no HF candidates; try a repo id or gold: fixture` | `2` |

See [Exit Codes](./exit-codes) for the full exit-code table.

## Caching

HF-shaped dispatches (`HfRepo`) route through the same cache as
[`hwledger search`](./hf-search):

- Cache root: `~/.cache/hwledger/hf/`
- TTL: **24 hours**
- Keyed on `<repo-id>/config@<rev>.json` (revision defaults to `main`)
- Offline fallback: on network failure the last cached `config.json` is used,
  with a tracing warning; `--offline` forces cache-only and errors loudly on
  miss.

Gold fixtures and local paths bypass the cache entirely — they read directly
from disk on every call, which is what makes them the preferred input for CI
and recorded tapes.

## Related

- [Quickstart](../getting-started/quickstart)
- [Hugging Face search](./hf-search)
- [CLI reference](./cli)
- [Exit Codes](./exit-codes)
