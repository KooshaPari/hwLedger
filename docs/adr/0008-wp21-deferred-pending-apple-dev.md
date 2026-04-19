# ADR-0008: WP21 Deferred — Apple Codesign + DMG + Sparkle

**Date**: 2026-04-19  
**Status**: ACCEPTED  
**Deciders**: KooshaPari (author)

## Context

WP21 (Phase P4.1) covers Apple macOS deployment hardening:
- Code signing (Developer ID Application certificate)
- DMG creation (disk image for distribution)
- Sparkle integration (auto-update framework)

hwLedger v1 MVP (WPs 01–20, 22–25, 27–28, 33) is feature-complete and buildable on macOS. However, WP21 requires:
1. Active Apple Developer Program membership (annual fee)
2. Developer ID Application certificate (issued by Apple)
3. Secure management of private keys + provisioning profiles

These prerequisites fall outside the scope of code implementation and are user-dependent.

## Decision

**Defer WP21 to post-MVP.**

Users who wish to distribute hwLedger beyond local development must:
1. Enroll in Apple Developer Program
2. Generate Developer ID Application certificate
3. Integrate Sparkle for auto-update flow (minimal code changes; template provided in WP33 CLI work)

## Rationale

- MVP is functionally complete without codesign/Sparkle (local dev + manual install works)
- Codesign is infrastructure, not business logic
- Sparkle is a nice-to-have, not a blocker
- Defer infrastructure work to "post-MVP hardening" phase

## Alternatives Considered

1. **Implement codesign anyway**: Requires dummy certs or self-signed keys; fails Notarization; not useful for real distribution. Rejected.
2. **Use self-signed keys**: Does not satisfy "code signing" requirement; users must override security prompts. Rejected.
3. **Skip Sparkle**: Users do manual updates; acceptable for MVP. Accepted as part of this deferral.

## Consequences

- hwLedger v1 macOS binary must be distributed as unsigned app or codesigned with user's own cert
- Sparkle auto-update not available until post-MVP
- No change to v1 MVP feature set or acceptance criteria
- WP21 becomes part of v1.1 hardening cycle (planned for Q2 2026 or user-driven demand)

## Implementation Guidance for Users

When ready to do WP21:
1. Follow Apple's [code signing guide](https://developer.apple.com/documentation/security/code_signing_your_app_to_prepare_for_distribution)
2. Use `codesign` CLI to sign the binary
3. Integrate Sparkle via `sparkle-core` Rust crate (Swift wrapper provided in WP16)
4. Test notarization flow (`xcrun notarytool submit …`)

Post-MVP issue template: "WP21 Implementation Guide for hwLedger Distribution"
