# hwLedger — Scripting Policy

hwLedger follows the Phenotype-org scripting language hierarchy. This document is a pointer — **do not restate the policy here**; consult the canonical doc.

## Hierarchy (summary)

1. **Rust** — default for every script, tool, pipeline component, hook, codegen step, and CI helper. Toolbelt: `clap` + `anyhow` + `serde` + `tokio` + `walkdir`.
2. **Zig, Mojo, Go** — acceptable alternates with a one-line top-of-file justification.
3. **Python, TypeScript** — second-layer fallback, only when the runtime must live inside an existing Python/TS process (Playwright spec, Jupyter notebook, VitePress config). Not for standalone CLIs.
4. **Bash / sh / zsh / fish / PowerShell** — only for ≤5-line platform glue with a top-of-file comment justifying why Rust/Go/etc. are worse for the specific case.

## Why this matters for hwLedger

hwLedger inherited several `.sh` scripts for CI, hooks (lefthook), and developer setup. Per the canonical policy, **new work must not add shell; existing shell must be migrated when touched.** When you open a PR that edits any `.sh`, your first choice is to delete it and reimplement in a Rust `tools/<name>/` crate or a `cargo xtask` subcommand.

## See also

- **Canonical long-form policy:** `/Users/kooshapari/CodeProjects/Phenotype/repos/docs/governance/scripting_policy.md`
- **Global wording:** `~/.claude/CLAUDE.md` → "Scripting Language Hierarchy"
- **thegent base template:** `Phenotype/repos/thegent/dotfiles/governance/CLAUDE.base.md` § 11a
- **Phenotype-org pointer:** `Phenotype/CLAUDE.md` § "Scripting Language Hierarchy"
