# AGENTS.md — AI-agent operating notes for hwLedger

This file supplements `/Users/kooshapari/.claude/CLAUDE.md` (global) and the workspace-level `repos/CLAUDE.md`. Read those first; the rules here are hwLedger-specific.

## Discipline

- **AgilePlus mandate applies.** Every work package must exist in AgilePlus before code is written.
  ```bash
  cd /Users/kooshapari/CodeProjects/Phenotype/repos/AgilePlus
  agileplus specify --title "hwLedger: <wp-title>" --description "..."
  agileplus status hwledger-v1-macos-mvp --wp <wp-id> --state <state>
  ```
- **Worklogs required.** Write to `repos/worklogs/ARCHITECTURE.md` for ADRs, `repos/worklogs/RESEARCH.md` for research dives, `repos/worklogs/DEPENDENCIES.md` for fork/wrap decisions. Tag entries `[hwLedger]`.
- **Branch discipline.** Work in `repos/.worktrees/hwLedger/<topic>/`, not in the canonical checkout. Canonical `hwLedger/` stays on `main`.
- **No code in docs/plans.** Planner output belongs in `PLAN.md`, `docs/adr/`, and `docs/research/`. Implementation goes in crates.

## Scope policing

- If a task expands beyond its AgilePlus work package, **stop and split** — do not grow the WP mid-flight.
- If you need to reach across Phenotype repos (`phenotype-event-sourcing`, `phenotype-health`, etc.), treat those as shared and follow the cross-project-reuse protocol: extract upward, don't copy sideways.

## Research cadence

- New architecture families (future MLA variants, new hybrid-attention layouts) must be handled by an ADR + a new `AttentionKind` variant + property tests, not by a fudge factor in a dense formula.
- Run the top-20-HF-models smoke test monthly. A model whose predicted VRAM diverges >10 % from vLLM / llama.cpp reported numbers is a regression.

## Quality gates (local)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-features
trufflehog git file://. --since-commit HEAD --only-verified --fail
```

Never use gitleaks (known to hang in multi-agent sessions per workspace memory).

## Agent delegation hints

- **Research**: delegate to Haiku agents in parallel. Cap word counts; demand cited URLs. Archive briefs in `docs/research/`.
- **Implementation**: delegate multi-file work to the `general-purpose` subagent. Direct parent-agent edits should stay narrow: synthesis, integration, finalisation.
- **Exploration**: `Explore` agent with `thoroughness: quick` for the first pass; escalate only on miss.

## Anti-patterns specific to hwLedger

- **Do not display a single "VRAM" number.** Always break into weights / KV / runtime / prefill / free.
- **Do not conflate concurrent-users with batch-size.** One drives persistent memory, the other drives step throughput. The slider UX must keep them on separate axes.
- **Do not ship a green/red gauge without a formula audit trail.** Users must be able to click the gauge and see which formula branch produced the number.
- **Do not vendor a model file into the repo.** Config metadata only; weights live in the user's cache or HF hub.
- **Do not add Python to any crate in `crates/`.** Python is confined to `sidecars/omlx-fork/` behind the JSON-RPC boundary.
