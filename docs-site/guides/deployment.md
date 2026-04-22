---
title: Deployment Guide
description: Running fleet server on a spare box
---

# Deployment Guide

Setting up a persistent fleet server on commodity hardware (old laptop, spare desktop, cloud instance).

## Prerequisites

- Linux or macOS with dedicated IP/hostname
- HTTPS/mTLS termination (self-signed cert OK)
- 2+ GB disk for ledger + model cache
- Network connectivity to agents

## Installation

<Shot src="/cli-journeys/keyframes/fleet-register/frame-001.png"
      caption="fleet register bootstrap"
      size="small" align="right" />

<Shot src="/cli-journeys/keyframes/fleet-register/frame-007.png"
      caption="Signature attached to attestation"
      size="small" align="left"
      :annotations='[{"bbox":[60,180,480,28],"label":"ed25519 signature","color":"#a6e3a1","position":"center-top"}]' />

<Shot src="/cli-journeys/keyframes/fleet-audit/frame-005.png"
      caption="audit summary after first run"
      size="small" align="right" />

<RecordingEmbed tape="fleet-register" caption="fleet register: walk through the bootstrap on a spare box" />

<RecordingEmbed tape="fleet-audit" caption="fleet audit: first-run verification against the fresh ledger" />

<Shot src="/cli-journeys/keyframes/fleet-audit/frame-001.png"
      caption="Audit entry point — operator types `hwledger audit --verify`"
      size="small" align="left" />

<Shot src="/cli-journeys/keyframes/fleet-audit/frame-003.png"
      caption="Chain integrity confirmed across N events"
      size="small" align="right"
      :annotations='[{"bbox":[80,140,440,24],"label":"chain OK","color":"#a6e3a1","position":"bottom-right"}]' />

### 1. Download binary

```bash
# From GitHub releases
curl -L https://github.com/KooshaPari/hwLedger/releases/download/v0.1.0/hwledger-linux-x86_64 \
  -o /usr/local/bin/hwledger
chmod +x /usr/local/bin/hwledger
```

### 2. Create directories

```bash
mkdir -p ~/.config/hwledger ~/.cache/hwledger/models ~/.cache/hwledger/archive
chmod 700 ~/.config/hwledger  # Private config
```

### 3. Generate TLS certificate

```bash
# Self-signed cert valid for 10 years
openssl req -x509 -newkey rsa:4096 -sha256 -days 3650 \
  -nodes -keyout ~/.config/hwledger/server.key.pem \
  -out ~/.config/hwledger/server.cert.pem \
  -subj "/CN=fleetserver.example.com"

chmod 600 ~/.config/hwledger/server.key.pem
```

### 4. Configure server

**File**: `~/.config/hwledger/server.toml`

```toml
[server]
listen_addr = "0.0.0.0:5443"
cert_path = "~/.config/hwledger/server.cert.pem"
key_path = "~/.config/hwledger/server.key.pem"
admin_token = "secret-bearer-token-here"

[db]
path = "~/.cache/hwledger/fleet.db"

[job_dispatch]
strategy = "round_robin"
max_concurrent_per_agent = 2

[ledger]
path = "~/.cache/hwledger/ledger.db"
retention_days = 90
verify_on_startup = true

[logging]
level = "info"
path = "/var/log/hwledger-server.log"
```

## Systemd setup (Linux)

### 1. Create unit file

**File**: `/etc/systemd/system/hwledger-server.service`

```ini
[Unit]
Description=hwLedger Fleet Server
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=hwledger
Group=hwledger
WorkingDirectory=/home/hwledger
ExecStart=/usr/local/bin/hwledger server
Restart=on-failure
RestartSec=10

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/home/hwledger/.cache /home/hwledger/.config

StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

### 2. Create user

```bash
sudo useradd --system --shell /bin/false --home-dir /home/hwledger hwledger
sudo mkdir -p /home/hwledger/.config /home/hwledger/.cache
sudo chown -R hwledger:hwledger /home/hwledger
```

### 3. Enable and start

```bash
sudo systemctl daemon-reload
sudo systemctl enable hwledger-server
sudo systemctl start hwledger-server

# Verify
sudo systemctl status hwledger-server
sudo journalctl -u hwledger-server -n 20  # Last 20 log lines
```

## launchd setup (macOS)

### 1. Create plist

**File**: `~/Library/LaunchAgents/com.hwledger.server.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.hwledger.server</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/hwledger</string>
        <string>server</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>WorkingDirectory</key>
    <string>/Users/YOUR_USERNAME</string>
    <key>StandardOutPath</key>
    <string>/tmp/hwledger-server.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/hwledger-server.err</string>
</dict>
</plist>
```

### 2. Load and start

```bash
launchctl load ~/Library/LaunchAgents/com.hwledger.server.plist
launchctl start com.hwledger.server

# Verify
launchctl list | grep hwledger
tail /tmp/hwledger-server.log
```

## Tailscale integration (optional)

For zero-config mesh networking:

```bash
# Install Tailscale
curl -fsSL https://tailscale.com/install.sh | sh

# Start Tailscale
sudo systemctl start tailscale
sudo tailscale up --auth-key tskey-...

# Agents connect via tailnet
hwledger fleet agents  # Should show Tailscale IPs
```

## Monitoring

### Health check

```bash
curl -k https://localhost:5443/health
# Output: {"status":"ok"}
```

### Log rotation

```bash
# Linux: logrotate
echo '/var/log/hwledger-server.log {
    daily
    rotate 7
    compress
    delaycompress
    notifempty
    create 0640 hwledger hwledger
    sharedscripts
    postrotate
        systemctl reload hwledger-server > /dev/null 2>&1 || true
    endscript
}' | sudo tee /etc/logrotate.d/hwledger-server
```

### Metrics

```bash
# Query active jobs
hwledger fleet jobs --status running

# Query agent uptime
hwledger fleet agents --json | jq '.agents[] | {id, last_heartbeat}'

# Check ledger size
du -h ~/.cache/hwledger/ledger.db
```

## Firewall

### UFW (Ubuntu)

```bash
sudo ufw allow 5443/tcp  # Fleet server
```

### AWS Security Group

Inbound rule:
- Type: Custom TCP
- Port: 5443
- Source: 0.0.0.0/0 (or restrict to agent subnets)

## Backup

```bash
# Backup config + ledger
tar czf hwledger-backup-$(date +%Y%m%d).tar.gz \
  ~/.config/hwledger \
  ~/.cache/hwledger/ledger.db

# Store off-box
scp hwledger-backup-*.tar.gz backup-server:/backups/
```

## Related

- [Fleet Server](/fleet/server)
- [Fleet Agent](/fleet/agent)
- [Troubleshooting](/guides/troubleshooting)
