---
title: Fleet Agent
description: Lightweight daemon for remote inference
---

# Fleet Agent (hwledger-agent)

Lightweight binary deployed on remote boxes (rentals, private machines, servers) that registers with the fleet server, maintains heartbeat, and executes inference jobs.

## Bootstrap flow

1. **Agent binary starts**: reads `~/.config/hwledger/agent.toml`
2. **Generate keypair** (if missing): RSA-4096, stored in `~/.config/hwledger/agent.pem`
3. **Register with server**: sends `AgentRegistration { hostname, ip, gpu_info, pubkey }` to fleet server via mTLS
4. **Server responds**: `RegistrationAck { agent_id, server_pubkey, cert }`
5. **Agent persists state**: writes `~/.cache/hwledger/agent.state.json` with `agent_id`, `server_pubkey`
6. **Begin heartbeat loop**

## Heartbeat loop

Every 5 seconds:

1. **Sample GPU**: call `hwledger-probe` → `TelemetrySnapshot { gpu_util, mem_free, temp, vram_free }`
2. **Send heartbeat**: POST `/fleet/heartbeat { agent_id, snapshot }` to server
3. **Receive commands**: server may include `[ { job_id, model, input } ]`
4. **Execute**: run inference, stream results back to server via WebSocket
5. **Update state**: mark job complete in local ledger

## State persistence

**File**: `~/.cache/hwledger/agent.state.json`

```json
{
  "agent_id": "agent-abc123",
  "server_pubkey": "-----BEGIN PUBLIC KEY-----\n...",
  "last_heartbeat": "2026-04-18T21:45:23Z",
  "completed_jobs": ["job-001", "job-002"],
  "failed_jobs": [
    { "job_id": "job-003", "error": "OOM", "timestamp": "2026-04-18T21:40:00Z" }
  ]
}
```

On crash/restart, agent reads this to:
- Resume pending jobs
- Skip already-completed jobs
- Report failures to server

## systemd wrapper

```ini
[Unit]
Description=hwLedger Fleet Agent
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/hwledger agent --config ~/.config/hwledger/agent.toml
Restart=on-failure
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

Installation:
```bash
sudo tee /etc/systemd/system/hwledger-agent.service < /path/to/unit
sudo systemctl daemon-reload
sudo systemctl enable --now hwledger-agent
```

## launchd wrapper (macOS)

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.hwledger.agent</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/hwledger</string>
        <string>agent</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/var/log/hwledger-agent.log</string>
    <key>StandardErrorPath</key>
    <string>/var/log/hwledger-agent.err</string>
</dict>
</plist>
```

Installation:
```bash
launchctl load ~/Library/LaunchAgents/com.hwledger.agent.plist
launchctl start com.hwledger.agent
```

## Configuration

**File**: `~/.config/hwledger/agent.toml`

```toml
[server]
addr = "fleet.hwledger.example.com:5443"
timeout_sec = 10

[heartbeat]
interval_sec = 5
max_retries = 3

[inference]
max_concurrent_jobs = 2
timeout_sec = 300

[logging]
level = "info"
```

## Related

- [Fleet Server: Central orchestration](/fleet/server)
- [Audit Log: Forensics](/fleet/audit-log)
- [Tailscale Discovery](/fleet/tailscale)
