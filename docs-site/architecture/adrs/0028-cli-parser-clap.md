# ADR 0028 — CLI parser: clap derive

Constrains: FR-CLI-001

Date: 2026-04-19
Status: Accepted

## Context

Every Rust binary in the workspace (`hwledger-server`, `hwledger-journey`, `hwledger-fleet`, `hwledger-docs-mcp-server`) exposes a CLI. Developers want colocated help strings, subcommand hierarchies, and shell completion; contributors want a mainstream parser so there's no learning tax.

## Options

| Parser | Style | Subcommands | Completion | Ecosystem | Compile time |
|---|---|---|---|---|---|
| clap 4 (derive) | Derive macros | Nested | Yes (`clap_complete`) | Huge | Higher |
| clap 4 (builder) | Runtime builder | Nested | Yes | Huge | Lower |
| argh | Derive | Flat | No | Small | Low |
| structopt | Derive (legacy) | Nested | Via clap | Deprecated | Higher |
| gumdrop | Derive | Flat | No | Tiny | Low |
| pico-args | Hand-rolled | Manual | No | Tiny | Trivial |
| lexopt | Hand-rolled | Manual | No | Tiny | Trivial |

## Decision

**clap 4 in derive mode** with `clap_complete` generating bash/zsh/fish completion scripts at build time.

## Rationale

- clap is the ecosystem default; contributors come in already fluent.
- Derive mode colocates docs, types, and validation. Help text matches source comments.
- `clap_complete` + `clap_mangen` generate completion + man pages from the same AST.
- argh/gumdrop are faster to compile but drop nested subcommands, which we need.
- Hand-rolled parsers (pico-args) save kilobytes at the cost of duplicated logic across six binaries.

## Consequences

- clap pulls a multi-second compile cost into every CLI crate. Mitigated by `clap = { version = "4", default-features = false, features = ["derive", "std", "help", "usage", "suggestions"] }` — no unused features.
- Breaking changes across clap majors require coordinated bumps. We pin the workspace version.

## Revisit when

- clap 5 ships with breaking changes that justify reconsideration.
- A significantly smaller derive-based parser reaches feature parity.

## References

- clap: https://github.com/clap-rs/clap
