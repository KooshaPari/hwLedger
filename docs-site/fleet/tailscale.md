---
title: Tailscale Discovery
description: Zero-config mesh networking for fleet
---

# Tailscale Discovery

Optional integration with Tailscale for automatic agent discovery, mTLS cert generation, and private network connectivity.

## Overview

Tailscale provides:
- **Mesh VPN**: agents and server reach each other via tailnet (no port forwarding)
- **Auto-discovery**: query Tailscale API to find agents on same tailnet
- **mTLS auto**: Tailscale can issue client certs (using OIDC)
- **Zero-trust**: DNS + IP allowlist built-in

## Enrollment

**Server**:
```bash
sudo tailscale up --auth-key tskey-...
hwledger server --tailscale --tailnet-name=myorg
```

**Agent**:
```bash
sudo tailscale up --auth-key tskey-...
hwledger agent --tailscale --server=fleetserver.myorg.ts.net:5443
```

Agents automatically discover server via Tailscale DNS.

## Auto-discovery API

Query Tailscale daemon for list of online devices:

```rust
let devices = tailscale::local_api()
    .devices()
    .filter(|d| d.tags.contains("hwledger-agent"))
    .collect();

for device in devices {
    server.register_agent(AgentRegistration {
        hostname: device.name,
        ip: device.ip,
        pubkey: device.cert.public_key(),
        ..Default::default()
    });
}
```

Server periodically polls Tailscale API (~1 min interval) to:
- Add new agents (online)
- Mark offline agents (absent from list)

## mTLS with Tailscale certs

Tailscale can issue client certs via OIDC:

```toml
[tailscale]
enable = true
oidc_provider = "yourcompany.okta.com"
```

Server validates cert chain:
1. Agent presents cert issued by `Tailscale Root CA`
2. Server trusts Tailscale Root CA (distributed in Tailscale daemon)
3. Cert CN = agent hostname on tailnet

Result: mTLS without manual key exchange.

## Configuration

**Server**: `~/.config/hwledger/server.toml`

```toml
[tailscale]
enable = true
poll_interval_sec = 60  # Check for new agents every 60s
tag_filter = "hwledger-agent"  # Only agents with this tag
```

**Agent**: `~/.config/hwledger/agent.toml`

```toml
[tailscale]
enable = true
server_hostname = "fleetserver.myorg.ts.net"
server_port = 5443
```

## Auto-tagging

Use Tailscale ACLs to tag agents:

```json
{
  "tagOwners": {
    "tag:hwledger-agent": ["autogroup:members"],
    "tag:gpu-capable": ["group:infra"]
  }
}
```

Server only discovers agents with `tag:hwledger-agent`.

## Network topology

```
Internet
    ↓
Tailscale Coordination Server (auth, discovery)
    ↑ ↓ ↑ ↓
[Fleet Server] ↔ [Agent1] ↔ [Agent2] ↔ [Agent3]
  (tailnet)       (tailnet)  (tailnet)  (tailnet)
```

All traffic encrypted end-to-end. Tailscale servers never see application data.

## Fallback (non-Tailscale)

If Tailscale not available or disabled, server falls back to manual registration:

```bash
hwledger fleet register-ssh --host user@123.45.67.89
```

## Related

- [SSH Fallback: Manual registration](/fleet/ssh-fallback)
- [Fleet Server: Orchestration](/fleet/server)
- [Fleet Agent: Worker daemon](/fleet/agent)
