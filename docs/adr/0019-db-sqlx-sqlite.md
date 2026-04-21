# ADR 0019 — Database: sqlx + SQLite

Constrains: FR-FLEET-010, FR-TEL-002

Date: 2026-04-19
Status: Accepted

## Context

`hwledger-server` persists fleet state (agents, jobs, heartbeats, audit events). MVP is single-node, so we want embedded SQLite. Ergonomics must support query-level compile-time validation, async execution, and migrations.

## Options

| Option | Compile-time check | Async | Migration support | Backend portability | Learning curve |
|---|---|---|---|---|---|
| sqlx 0.8 | Yes (`query!` macro) | Yes | Yes (`sqlx-cli`) | pg, mysql, sqlite, mssql | Low |
| diesel 2 | Yes (schema-based) | Partial (diesel_async) | Yes | pg, mysql, sqlite | High (DSL) |
| sea-orm 1.x | Yes (entities) | Yes | Yes | Same as sqlx | Medium |
| rusqlite | No (stringly) | No (blocking) | Manual | sqlite only | Low |
| turbosql | Yes (procs) | Yes | Auto | sqlite only | Low |

## Decision

**sqlx 0.8 with SQLite** for the MVP. `DATABASE_URL=sqlite://ledger.db` at the workspace root. Migrations live in `crates/hwledger-server/migrations/` driven by `sqlx migrate`.

## Rationale

- Query-as-strings with `query!` macro gives compile-time SQL validation without the ORM abstraction tax. Matches how the team thinks (SQL first).
- Async-native — composes with axum (ADR-0018) and the tokio runtime.
- Same API lets us graduate to Postgres in v2 without rewriting call sites; changing the connection URL is most of the work.
- Diesel's DSL is verbose and its async story is still bolted on.
- sea-orm's active-record style buries SQL intent we want visible for auditability.

## Consequences

- SQLite single-writer limits write concurrency. Acceptable for MVP (single server, <100 agents). v2 migrates to Postgres when load demands.
- Compile-time checks require `DATABASE_URL` to be set during `cargo build` or `sqlx prepare` to vendor offline metadata. We vendor via `.sqlx/` checked into git.

## Revisit when

- Fleet exceeds ~100 concurrent agents or writes exceed ~50/s.
- Horizontal scaling or multi-region becomes a goal.
- sqlx maintainership changes.

## References

- sqlx: https://github.com/launchbadge/sqlx
- ADR-0018 (axum), ADR-0009 (admin CN).
