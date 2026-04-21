# ADR 0030 — Package registry: GitHub Packages for org-private `@phenotype/*`

Constrains: FR-OPS-002

Date: 2026-04-19
Status: Accepted

## Context

hwLedger consumes several Phenotype-org TypeScript packages (e.g. `@phenotype/docs`, `@phenotype/design`) and publishes at least one (`@phenotype/hwledger-journey`). We need a registry that hosts org-private npm packages, supports tokens-based auth in CI, and integrates with existing GitHub identity.

## Options

| Option | Private scope | Auth | Cost | Org integration | Redundancy risk |
|---|---|---|---|---|---|
| npmjs.com (public) | Paid orgs for private | Token | $7/user/mo for teams | Separate ID | External |
| GitHub Packages (npm) | Yes (via GHP) | `GITHUB_TOKEN` | Free for public; included for private in plans | Native (GH org = scope) | Same blast radius as GH |
| Verdaccio (self-hosted) | Yes | LDAP/htpasswd | Compute only | Manual SSO | We own outage |
| JSR (jsr.io) | Public focus | OIDC | Free | GH OAuth | External |
| Cloudsmith / Artifactory | Yes | Token | $$ | SSO-enterprise | External |

## Decision

**GitHub Packages (npm registry)** hosts all `@phenotype/*` private packages. `.npmrc` scopes the `@phenotype` scope to `https://npm.pkg.github.com`; CI reads `GITHUB_TOKEN`. Public releases, when any, mirror to npmjs.com.

## Rationale

- Zero new identity to manage: whoever can push code to the repo can publish the package.
- Free for private use inside the org's existing GitHub plan.
- Tied to the same blast radius we already accept for source (GitHub outage takes down builds regardless).
- Verdaccio is a credible fallback if we ever self-host — we keep registry URLs variable so we can swap.
- JSR is Deno-flavored; too narrow for our full TS needs.

## Consequences

- GitHub Packages npm authentication quirks (scope-specific `.npmrc`) must be documented. Contributors hit this once.
- Download throughput from GHP is slightly slower than npmjs CDN; irrelevant at our package sizes.
- Consumers outside the org cannot pull without a PAT — that's by design.

## Revisit when

- Org grows enough to justify self-hosting (Verdaccio/Gitea Packages).
- GitHub Packages pricing or quotas change.
- JSR becomes relevant for cross-runtime packages.

## References

- GitHub Packages: https://docs.github.com/packages
- ADR-0031 (CI), ADR-0029 (uv — Python side uses separate PyPI policy).
