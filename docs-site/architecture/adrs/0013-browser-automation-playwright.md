# ADR 0013 — Browser automation: Playwright

Constrains: FR-JOURNEY-004, FR-DOCS-004

Date: 2026-04-19
Status: Accepted

## Context

Journey capture records GUI flows (docs-site, Streamlit dashboards, deployed fleet UI) as screenshots + mp4 + DOM snapshots. The tool must script deterministic interactions, capture network traffic (for API-level attestation), run headlessly on the self-hosted runner, and be wrapped by `hwledger-journey` (Rust).

## Options

| Option | Cross-browser | Async API | Network mock | Video capture | Rust client | License |
|---|---|---|---|---|---|---|
| Playwright | Chromium/Firefox/WebKit | Yes | Yes (route intercept) | Yes (native) | `playwright-rust` (community) | Apache 2 |
| Cypress | Chromium only (+ experimental FF) | Promise chain | Yes | Yes | No | MIT |
| Selenium 4 | All | Yes | Via BiDi | External | `thirtyfour` | Apache 2 |
| Puppeteer | Chromium only | Yes | Yes | Yes | `headless_chrome` | Apache 2 |
| CDP direct (`chromiumoxide`) | Chromium only | Yes | Yes | Manual | Native Rust | MIT |

## Decision

**Playwright** (TypeScript driver) invoked from `apps/journey-renderer/` as a subprocess. Journey manifests compile to Playwright test scripts. WebKit variant used for macOS Safari parity checks.

## Rationale

- Only tool that covers Chromium + Firefox + WebKit with one API; required because hwLedger docs-site targets all three.
- Network interception (`page.route`) enables recording API traffic for attestation payloads (ADR-0015).
- Playwright's video + trace artifacts slot directly into Remotion (ADR-0011) and the docs-site `<Shot>` component.
- Auto-wait semantics dramatically reduce flake vs Selenium.

## Consequences

- Node/Bun runtime required (already needed per ADR-0011).
- Rust binding (`playwright-rust`) is community-maintained and lags behind TS by ~1 minor version. We drive Playwright via TS and surface a thin Rust wrapper.
- WebKit capture on Linux uses GTK WebKit, not Safari; minor rendering diff vs real Safari.

## Revisit when

- `thirtyfour` (Rust Selenium) reaches Playwright feature parity and can swap in without manifest changes.
- `chromiumoxide` grows a multi-browser abstraction.
- WebDriver BiDi becomes a practical cross-browser control protocol (2027+).

## References

- Playwright: https://playwright.dev
- thirtyfour: https://github.com/stevepryde/thirtyfour
- ADR-0011 (Remotion), ADR-0014 (OCR), ADR-0016 (manifest).
