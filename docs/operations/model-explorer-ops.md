# Model Explorer — Operator Runbook

> **Subsystem:** `apps/model-explorer/` (hwLedger)
> **Audience:** operators running the model-explorer stack end-to-end.
> **Pair docs:**
> [`docs/adr/2026-07-23/ADR-model-explorer.md`](../adr/2026-07-23/ADR-model-explorer.md)
> ·
> [`docs/superpowers/specs/acceptance/2026-07-23-hwledger-model-explorer.md`](../superpowers/specs/acceptance/2026-07-23-hwledger-model-explorer.md)

The model-explorer is hwLedger's inventory layer: a Tantivy BM25 index fed from
the HuggingFace Hub, exposed over four surfaces — a CLI, an MCP stdio server,
an Axum HTTP server, and (optionally) a SvelteKit web UI served through a Hono
proxy. This runbook is the **how**: build, run, env vars, and the deferred
items callers will ask about.

## 1. Workspace layout

The subsystem lives at `apps/model-explorer/` with three sub-trees:

```text
apps/model-explorer/
├── rust/      # Cargo workspace — 3 binaries + 7 search-* library crates
├── server/    # Hono proxy (Node 20+, TS) → forwards to hwledger-server
└── web/       # SvelteKit UI (Vite, adapter-node) → consumes the Hono proxy
```

The Rust workspace is declared in `apps/model-explorer/rust/Cargo.toml:3-14`
and bundles ten crates, three of which are the front-end binaries this runbook
concerns itself with:

| Crate | Binary | Transport |
| :-- | :-- | :-- |
| `crates/hwledger-cli` | `hwledger-cli` | argv (clap) |
| `crates/hwledger-mcp` | `hwledger-mcp` | JSON-RPC 2.0 over stdio |
| `crates/hwledger-server` | `hwledger-server` | HTTP (Axum) on `0.0.0.0:$PORT` |

## 2. Build

Run from the monorepo root unless noted.

```bash
# Rust workspace — debug build, all members
cargo build --manifest-path apps/model-explorer/rust/Cargo.toml

# Rust workspace — release artifacts (target/release/{hwledger-cli,hwledger-mcp,hwledger-server})
cargo build --release --manifest-path apps/model-explorer/rust/Cargo.toml

# Whole-workspace test (stub adapters, no network)
cargo test  --manifest-path apps/model-explorer/rust/Cargo.toml

# Lint (CI gate)
cargo clippy --manifest-path apps/model-explorer/rust/Cargo.toml --all-targets -- -D warnings

# Hono proxy (TypeScript → dist/)
npm --prefix apps/model-explorer/server install
npm --prefix apps/model-explorer/server run build   # tsc → apps/model-explorer/server/dist/

# SvelteKit UI (Vite → build/)
npm --prefix apps/model-explorer/web install
npm --prefix apps/model-explorer/web run build       # adapter-node → apps/model-explorer/web/build/
```

## 3. `seed build` — populate the index (`HF_TOKEN` required)

`hwledger-cli seed build` fans out across HF search queries, fetches each
candidate model's metadata + tree + README, and pushes parsed rows into the
Tantivy sink. The default query set is
`hwledger-search-ingest::seed_builder::DEFAULT_SEED_QUERIES`:

```text
qwen2.5, llama-3.1, deepseek-v3, gemma-2,
mistral-nemo, phi-3, codestral, bge-large
```

### Run

```bash
# HF_TOKEN is required for seed build (raises non-zero on 401/429).
export HF_TOKEN=$(security find-generic-password -s hf -w)   # macOS Keychain example

hwledger-cli --index ./hwledger-index seed build --size 2000
```

`HF_TOKEN` is consumed by `HuggingFaceAdapter::from_env()`
(`crates/hwledger-search-ingest/src/huggingface.rs`). Empty / unset token is
treated as "no auth" and the run will fail on gated or rate-limited paths. Use
a **read-only** token; the CLI never calls write endpoints.

### Failure modes

| Symptom | Cause | Action |
| :-- | :-- | :-- |
| non-zero `errors`, zero `models_indexed` | every request returned 401/403/429 | missing / invalid `HF_TOKEN`; set it and re-run |
| `failed to wipe existing index` | perms / disk conflict | inspect `--index`; do not symlink across machines |
| tantivy commit failed | disk full or index locked by another process | free space, serialize writers |

## 4. Running the three Rust binaries

### 4.1 `hwledger-cli` — operator CLI

```bash
# Release artifact
./target/release/hwledger-cli --index ./hwledger-index model search "small instruct coder"
./target/release/hwledger-cli --json --index ./hwledger-index model detail meta-llama/Llama-3.1-8B
```

| Env var | Default | Effect |
| :-- | :-- | :-- |
| `HWLEDGER_INDEX` | `./hwledger-index` | Tantivy index path (CLI flag `--index` overrides). |
| `HF_TOKEN` | (unset) | Bearer auth on HF requests when set & non-empty. **Required for `seed build`.** |
| `HF_HUB_URL` | `https://huggingface.co` | Upstream base URL override (tests / CI fixtures). |
| `RUST_LOG` | `info` | Standard `tracing` filter. |

### 4.2 `hwledger-mcp` — MCP stdio server

JSON-RPC 2.0 over stdin/stdout (MCP 2024-11-05). Six stub tools + `initialize`
/ `tools/list` / `tools/call`. Launched as a child process by an MCP-aware
client; no flags, no socket.

```bash
./target/release/hwledger-mcp   # spawn from the MCP client; not for humans
```

Per-request failures are surfaced as a last-resort JSON-RPC error on stdout
(see `crates/hwledger-mcp/src/main.rs:22`). The binary uses the same Tantivy
index as the CLI; ensure `HWLEDGER_INDEX` points at it.

### 4.3 `hwledger-server` — Axum HTTP server (`:8080`)

```bash
./target/release/hwledger-server
# → http://0.0.0.0:8080  (health: GET /healthz)
```

| Env var | Required | Default | Effect |
| :-- | :-- | :-- | :-- |
| `DATA_DIR` | **yes** | (unset) | Tantivy index directory. `TantivyStore::open` creates it if missing. |
| `PORT` | no | `8080` | Listen port. Parsed as `u16`; invalid → 8080. |
| `ADMIN_TOKEN` | no (recommended in prod) | (unset) | Bearer token required by `POST /v1/admin/*`. **Unset ⇒ every admin request is rejected with 401** so a misconfigured deployment never silently grants access. |
| `RUST_LOG` | no | `info` | `tracing-subscriber` env filter. |

Graceful shutdown on SIGINT / SIGTERM
(`crates/hwledger-server/src/main.rs:73`). The server is the only consumer of
`DATA_DIR` and `ADMIN_TOKEN`; the CLI and MCP binary use `HWLEDGER_INDEX` and
do not enforce an admin token.

## 5. Hono proxy (`:8787`) + SvelteKit UI

The Hono proxy (`apps/model-explorer/server/`) sits in front of
`hwledger-server`, exposes the same JSON contract the CLI produces under
`--json`, and falls back to a synthesized payload when the Rust server is
offline (response header `x-upstream: synthesized`).

```bash
# 1. Run the Axum upstream on :8080 (see §4.3).
# 2. Run the Hono proxy on :8787.
npm --prefix apps/model-explorer/server run start
# → http://127.0.0.1:8787  (proxies to http://127.0.0.1:8080 by default)

# 3. (Optional) Run the SvelteKit UI.
npm --prefix apps/model-explorer/web run dev      # vite dev
# or, against a production build:
npm --prefix apps/model-explorer/web run build
node apps/model-explorer/web/build/index.js
```

| Env var | Default | Effect (proxy) |
| :-- | :-- | :-- |
| `PORT` | `8787` | Proxy listen port. |
| `HOST` | `127.0.0.1` | Bind address. Use `0.0.0.0` in containers. |
| `HWLEDGER_UPSTREAM_URL` | `http://127.0.0.1:8080` | Rust server base URL. |
| `HWLEDGER_UPSTREAM_TIMEOUT_MS` | `4000` | Per-request upstream timeout. |

The web app is a pure consumer of the proxy — it never holds `HF_TOKEN` or
`ADMIN_TOKEN`. The proxy is the only surface that originates admin requests.

## 6. Per-binary env var summary

| Binary | Required | Optional | Notes |
| :-- | :-- | :-- | :-- |
| `hwledger-cli` | `HF_TOKEN` (for `seed build`) | `HWLEDGER_INDEX`, `HF_HUB_URL`, `RUST_LOG` | `--index` flag overrides `HWLEDGER_INDEX`. |
| `hwledger-mcp` | none | `HWLEDGER_INDEX`, `RUST_LOG` | Stdio transport only; env-only config. |
| `hwledger-server` | `DATA_DIR` | `PORT` (8080), `ADMIN_TOKEN`, `RUST_LOG` | Unset `ADMIN_TOKEN` ⇒ all admin routes 401. |
| `server/` (Hono) | none | `PORT` (8787), `HOST`, `HWLEDGER_UPSTREAM_URL`, `HWLEDGER_UPSTREAM_TIMEOUT_MS` | Forwards to the Rust server. |
| `web/` (SvelteKit) | none | `NEXT_PUBLIC_HWLEDGER_API`, `HWLEDGER_WEB_PORT`, `HWLEDGER_WEB_LOG_LEVEL` | No secrets in the browser bundle. |

## 7. Deferred items

These are real today and known to callers. None block the inventory phase;
each is wired through a stable seam so the operator contract doesn't churn.

| ID | What is deferred | Where the seam is today | What lands |
| :-- | :-- | :-- | :-- |
| **Deferred 1** | MCP-over-HTTP transport (Streamable HTTP, 2025 rev) | `crates/hwledger-mcp/src/main.rs` (stdio only) | Sibling `transport_http.rs` + new `[[bin]]` re-using `McpServer`. |
| **Deferred 2** | SvelteKit UI implementation | `apps/model-explorer/web/` (scaffold only) | Faceted search, model detail, `model-ask` panel, admin rebuild page. |
| **Deferred 3** | ORT-based embedder backend | `crates/hwledger-search-rag/src/embedder.rs:34` (`Embedder` trait) | FastEmbed / candle / ORT behind the trait. |
| **Deferred 4** | LanceDB dense index + RRF fusion wired into `run_hybrid` | `crates/hwledger-search-index/src/query.rs:32` | `rrf_fuse` (`k = 60`) already implemented in `hwledger-search-core::fusion`; `lancedb = "0.13"` reserved in workspace root. |
| **Deferred 5** | Web-driven `seed build` | `apps/model-explorer/web/` | UI proxies to `hwledger-server` admin; the token never enters the browser. |

## 8. Quick recipes

```bash
# Cold-start the full stack (debug).
export HF_TOKEN=***       # read-only HF token
cargo build --manifest-path apps/model-explorer/rust/Cargo.toml
./target/debug/hwledger-cli --index ./idx seed build --size 2000
./target/debug/hwledger-server &                     # serves :8080
npm --prefix apps/model-explorer/server run start &  # serves :8787
npm --prefix apps/model-explorer/web run dev         # serves :5173

# Production-style (release build, anonymous-bind, admin token gated).
cargo build --release --manifest-path apps/model-explorer/rust/Cargo.toml
export DATA_DIR=/var/lib/hwledger/index
export ADMIN_TOKEN=$(openssl rand -hex 32)
export PORT=8080
./target/release/hwledger-server &
PORT=8787 HWLEDGER_UPSTREAM_URL=http://127.0.0.1:8080 \
  node apps/model-explorer/server/dist/index.js &
```

## 9. Escalation

If something this runbook doesn't cover comes up:

1. Check the per-crate doc comments — every public item has `//!` / `///`
   docs (`#![deny(missing_docs)]` enforced).
2. See the acceptance skeleton at
   `docs/superpowers/specs/acceptance/2026-07-23-hwledger-model-explorer.md`
   for the phased contract.
3. File an issue with the binary version (`<bin> --version`), `RUST_LOG`
   output, and the exact env vars passed.
