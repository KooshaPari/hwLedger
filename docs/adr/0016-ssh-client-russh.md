# ADR 0016 — SSH client: russh (native Rust)

Constrains: FR-FLEET-008

Date: 2026-04-19
Status: Accepted

## Context

`hwledger-fleet` drives provisioning + job dispatch over SSH on rented GPU hosts (vast.ai, Hetzner, runpod). The SSH client must be embeddable in a Rust async runtime (tokio), stream stdout/stderr without shelling out, authenticate via ed25519 keys + agent, and forward local ports for reverse tunnels.

## Options

| Option | Pure Rust | Async | Key formats | Port forwarding | Maintainer activity | License |
|---|---|---|---|---|---|---|
| russh 0.46 | Yes | tokio-native | ed25519, rsa, ecdsa, agent | Yes (direct-tcpip + tcpip-forward) | Active (Pixelcode) | Apache 2 |
| thrussh (legacy) | Yes | futures | ed25519, rsa | Partial | Inactive (fork base of russh) | Apache 2 |
| libssh2 (via `ssh2` crate) | No (C FFI) | Blocking | ed25519, rsa | Yes | Stable but slow | BSD |
| libssh (via `libssh-rs`) | No (C FFI) | Blocking | All | Yes | Active C project | LGPL (problematic) |
| OpenSSH subprocess | N/A | via pipes | All | Via `-L` flag | N/A | BSD |

## Decision

**russh 0.46** is the fleet SSH client. We depend on `russh-keys` for agent socket parsing and `russh-sftp` for file transfer.

## Rationale

- Only pure-Rust async SSH client in 2026 with first-class tokio integration and tcpip-forward support. Native integration avoids a C dep in every release artifact.
- Apache-2 licensed; compatible with our distribution.
- Active maintainer; responsive to CVE reports (russh-0.45.0 had a rekey bug fixed in 0.45.2).
- libssh2 is blocking → does not compose with the rest of our tokio-based fleet code.
- libssh-rs is LGPL; adds a licensing footnote for static linking.
- OpenSSH subprocess works but loses structured error handling and forces a shell on the build host.

## Consequences

- Smaller implementation than OpenSSH; we inherit russh's audit surface. Mitigated by pinning to reviewed versions + running cargo-audit in CI.
- Key agent (ssh-agent) protocol support is manual via `russh-keys`; we cap supported agents to OpenSSH + 1Password SSH.

## Revisit when

- A fully tokio-native libssh wrapper ships.
- russh is abandoned or a fork supersedes it.
- Post-quantum SSH key exchange (sntrup-761x25519) support is needed — russh is tracking; we upgrade when available.

## References

- russh: https://github.com/Eugeny/russh
- OpenSSH PQC: https://www.openssh.com/releasenotes.html
- ADR-0009 (mTLS), ADR-0019 (HTTP).
