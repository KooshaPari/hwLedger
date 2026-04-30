# hwLedger

LLM capacity planner + fleet ledger + desktop inference runtime. Tracks hardware fleet audit and provenance for ML workloads, provides VRAM estimation per layer for any HF/GGUF/MLX/Ollama model, reconciles against live telemetry, and maintains an event-sourced audit log.

## Stack
| Layer | Technology |
|-------|------------|
| Core | Rust (cargo workspace, 26 crates) |
| GUI | SwiftUI (macOS), WinUI 3 (Windows), Slint/Qt6 (Linux) |
| FFI | Shared Rust core via C FFI bindings |
| DB | SQLite (ledger), event-sourced audit log |
| Inference | MLX, mistral.rs, llama.cpp, vLLM, TGI, oMlx |

## Key Commands
```bash
# Build the Rust workspace
cargo build --workspace

# Run tests
cargo test --workspace

# Lint
cargo clippy --all -- -D warnings

# Format
cargo fmt --all

# Run the CLI
cargo run -p hwledger-cli -- --help

# Build the landing app
task landing:install
task landing:build

# DB
sqlite3 ledger.db ".tables"
```

## Key Files
- `crates/` — 26 Rust workspace crates (hwledger-core, hwledger-arch, etc.)
- `apps/` — Native GUI entry points per OS
- `tests/` — Integration tests
- `docs/` — Documentation
- `docs-site/` — Published VitePress docsite
- `PLAN.md` — Implementation roadmap

## Reference
Global Phenotype rules: see `~/.claude/CLAUDE.md` or `/Users/kooshapari/CodeProjects/Phenotype/repos/CLAUDE.md`
