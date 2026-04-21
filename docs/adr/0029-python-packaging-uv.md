# ADR 0029 — Python packaging: uv

Constrains: FR-OPS-001, FR-DASH-001

Date: 2026-04-19
Status: Accepted

## Context

Python appears in journey scripts, Streamlit dashboards (ADR-0026), and ops utilities. We want a single package manager + runner across the monorepo that is fast, deterministic, and lockfile-first. The workspace already standardizes on `uv` per the Phenotype scripting policy.

## Options

| Tool | Install speed | Lockfile | Venv management | Build backend | Maintainer |
|---|---|---|---|---|---|
| uv 0.4+ | Fastest (Rust) | Yes | Yes | PEP 517 | Astral |
| pip + venv | Slow | requirements.txt | Manual | PEP 517 | PyPA |
| poetry | Medium | poetry.lock | Yes | Own backend | Community |
| hatch | Medium | Via hatchling | Yes | Hatchling | PyPA-adjacent |
| pdm | Medium | pdm.lock | Yes | PEP 621 | Frost Ming |
| conda / mamba | Fast (mamba) | environment.yml | Yes | Non-PEP | Anaconda |

## Decision

**uv** is the sole Python package manager and task runner across the workspace. `pyproject.toml` declares deps; `uv.lock` is committed; `uv run`, `uv sync`, `uv tool install` are the only invocations.

## Rationale

- 10–100× faster than pip for cold installs; removes CI bottleneck.
- Astral also owns ruff; their velocity and focus are high.
- Single lockfile across dev + CI + prod; deterministic across platforms.
- Poetry's resolver is slow; hatch and pdm are fine but smaller ecosystems.
- conda is overkill outside data-science-specific native deps; we don't need its non-PEP extras.

## Consequences

- Contributors who hand-craft `pip install` invocations must switch. Mitigated by `uv pip` shim and a Makefile target.
- uv is young (<1.0 in 2026-04). Mitigated by Astral's track record (ruff 1.0) and the underlying PEP-compliant outputs — we can migrate off with `uv export -o requirements.txt`.

## Revisit when

- uv reaches 1.0 (confirm stability) — expected within 2026.
- Astral pivots away from uv.
- A faster tool emerges (unlikely; uv is near the speed floor).

## References

- uv: https://github.com/astral-sh/uv
