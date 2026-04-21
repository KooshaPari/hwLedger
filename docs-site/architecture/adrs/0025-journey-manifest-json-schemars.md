# ADR 0025 — Journey manifest schema: JSON + schemars-derived JSONSchema

Constrains: FR-JOURNEY-001..008

Date: 2026-04-19
Status: Accepted

## Context

Journey manifests describe a capture run: scenes, terminals, browser steps, narration lines, expected sentinels, and attestation pointers. The manifest is authored by humans, produced by tools (generator), and consumed by agents across Rust (`hwledger-journey`), TypeScript (Remotion, Playwright), and Python (docs-site scripts). It is also signed (ADR-0024) and embedded in docs.

## Options

| Format | Human-auth | Schema tooling | Signing-friendly | Language support | Size |
|---|---|---|---|---|---|
| JSON + JSONSchema (schemars) | Yes | Excellent | Yes (canonical JSON) | Every language | Medium |
| Protobuf | No | Native | Needs canonicalisation | Via plugins | Small |
| FlatBuffers | No | Native | Needs canonicalisation | Multi | Smallest |
| Cap'n Proto | No | Native | Needs canonicalisation | Multi | Small |
| MessagePack | No | External | Yes | Multi | Small |
| CBOR | No | External | Yes | Multi | Small |
| YAML | Yes | JSONSchema reuse | Ambiguous canon form | Multi | Medium |
| TOML | Yes | Custom schema | Poor (unordered maps) | Multi | Medium |

## Decision

Manifests are **JSON** with schemas derived from Rust types via **schemars 0.8**. Canonical form (for signing) is [RFC 8785 JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785). Schema is exported to `docs-site/journeys/manifest.schema.json` and referenced by `$schema` in every manifest.

## Rationale

- JSON has zero install cost in every consumer language.
- schemars lets us single-source types: Rust struct → both serde (runtime) and JSONSchema (editor autocomplete + CI validation).
- RFC 8785 gives a deterministic byte sequence for signing without inventing our own canonicalizer.
- Binary formats (protobuf/flatbuffers/capnp) optimize for size + speed, neither of which is the bottleneck for 50 KB manifests authored weekly.
- YAML's anchor/alias features and whitespace ambiguity complicate signing and editor tooling.
- TOML can't express nested arrays-of-maps cleanly for scene graphs.

## Consequences

- Manifests are larger than binary formats (~3×), irrelevant at our size.
- Schema drift between Rust and consumers is caught by the CI check that regenerates `manifest.schema.json` and fails if it differs from the committed file.
- Signing is stable: canonicalize → hash → sign (ADR-0024).

## Revisit when

- Manifests grow into the megabytes (streaming capture, unlikely).
- A Rust-native canonical-JSON formatter replaces our current `olpc-cjson` / hand-rolled implementation and becomes the ecosystem default.
- JSONSchema Draft 2026+ introduces features schemars cannot emit.

## References

- schemars: https://github.com/GREsau/schemars
- RFC 8785: https://www.rfc-editor.org/rfc/rfc8785
- ADR-0024 (ed25519 signing).
