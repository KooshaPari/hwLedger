---
title: SSH Fallback (Agentless Mode)
description: Query remote GPUs without agent installation
---

# SSH Fallback (Agentless Mode)

For environments where installing the hwledger-agent binary is not possible, the fleet server can query remote GPU state via SSH using only `nvidia-smi` or `rocm-smi`.

## Flow

1. **User registers via SSH**: `hwledger fleet register-ssh --host user@remote.box --key ~/.ssh/id_ed25519`
2. **Server stores SSH config**: hostname, IP, SSH key fingerprint
3. **On heartbeat request**: server SSHes into remote, runs `nvidia-smi --query-gpu=index,name,memory.total,memory.free --format=csv,noheader`
4. **Parse output**: extract GPU info, return as TelemetrySnapshot
5. **No persistent state**: each heartbeat is stateless SSH call

## Advantages

- No binary to deploy (already have SSH)
- Works on shared clusters (HPC, cloud)
- Key-based auth (no passwords in config)

## Disadvantages

- Slower: SSH overhead ~200ms per heartbeat (vs 5ms local agent)
- Limited info: only GPU state (no CPU, memory)
- No persistent job queue on remote (server queues, SSH calls trigger fetch-and-run)

## Configuration

**File**: `~/.config/hwledger/ssh-agents.toml`

```toml
[[agents]]
name = "vast-rental-1"
hostname = "123.45.67.89"
ssh_user = "root"
ssh_key_path = "~/.ssh/id_ed25519"
ssh_port = 22
gpu_query_cmd = "nvidia-smi --query-gpu=index,name,memory.total,memory.free --format=csv,noheader"

[[agents]]
name = "lambda-labs-2"
hostname = "compute-2.lambda-labs.com"
ssh_user = "ubuntu"
ssh_key_path = "~/.ssh/lambda_key"
gpu_query_cmd = "rocm-smi --showtemp --showmeminfo --csv"
```

## Heartbeat via SSH

Server routine (runs every 5s per agent):

```rust
async fn heartbeat_ssh(agent: &SshAgent) -> Result<TelemetrySnapshot> {
    let session = agent.ssh_connect().await?;
    let output = session.exec(agent.gpu_query_cmd).await?;
    let snapshot = TelemetrySnapshot::from_nvidia_csv(&output)?;
    session.close().await;
    Ok(snapshot)
}
```

## Job execution via SSH

User submits job to agentless remote:

1. Server queues job in DB
2. On next heartbeat, server prepares job
3. Server SSHes, writes job JSON to `/tmp/hwledger-job-XXX.json`
4. Server SSHes, executes: `hwledger run /tmp/hwledger-job-XXX.json --output /tmp/result-XXX.json`
5. Server SSHes, reads result, deletes temp files

Result streaming: not available (SSH fallback is pull-based, not push).

## Bastion/jump host support

For proxied SSH (bastion, VPN jump):

```toml
[[agents]]
name = "vpn-private-gpu"
hostname = "internal-gpu.local"
ssh_user = "ubuntu"
ssh_key_path = "~/.ssh/id_ed25519"

[agents.bastion]
hostname = "bastion.example.com"
ssh_user = "root"
ssh_key_path = "~/.ssh/bastion_key"
```

Server automatically chains:
```
local → bastion.example.com → internal-gpu.local
```

## Limitations

| Feature | Agent | SSH Fallback |
|---------|-------|--------------|
| Real-time streaming | Yes | No |
| Persistent queue | Yes | No |
| Sub-second heartbeat | Yes | No |
| CPU/memory telemetry | Yes (partial) | No |
| Requires binary | No | No (SSH only) |

## Related

- [Fleet Agent: Full-featured](/fleet/agent)
- [Fleet Server: Orchestration](/fleet/server)
- [Cloud Rentals: Provider integration](/fleet/cloud-rentals)
