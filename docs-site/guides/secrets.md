---
title: Secrets Management
description: Sparkle key, keychain, credentials
---

# Secrets Management

Handling private keys, Sparkle certificates, and cloud API credentials securely.

<Shot src="/cli-journeys/keyframes/fleet-register/frame-007.png"
      caption="Signature attached during fleet register"
      size="small" align="right" />

<!-- SHOT-TODO: capture keychain unlock prompt when running hwledger fleet register -->

## Sparkle private key

The Sparkle framework requires an Ed25519 private key for signing app updates.

### Generation

```bash
# Generate key pair (one-time)
openssl genpkey -algorithm ED25519 -out sparkle_private.key
openssl pkey -in sparkle_private.key -pubout -out sparkle_public.key

# Store private key securely
mv sparkle_private.key ~/.config/hwledger/sparkle-key.pem
chmod 600 ~/.config/hwledger/sparkle-key.pem
```

### macOS Keychain storage

Instead of storing key in plaintext, use Keychain:

```bash
# Import key to keychain
security import ~/.config/hwledger/sparkle-key.pem \
  -k ~/Library/Keychains/login.keychain-db \
  -T /usr/local/bin/hwledger

# Reference in code
let key = try keychain.getItem(kSecClassKey, label: "hwledger-sparkle-key")
```

### CI/CD (GitHub Actions)

1. Store base64-encoded key in GitHub Secrets: `SPARKLE_PRIVATE_KEY`
2. Decode at runtime:

```yaml
- name: Decode Sparkle key
  env:
    SPARKLE_KEY_BASE64: ${{ secrets.SPARKLE_PRIVATE_KEY }}
  run: |
    echo "$SPARKLE_KEY_BASE64" | base64 -d > /tmp/sparkle.key
    chmod 600 /tmp/sparkle.key
    export SPARKLE_KEY_PATH=/tmp/sparkle.key
```

## Fleet server mTLS certificates

### Self-signed generation

```bash
# One-time setup
openssl req -x509 -newkey rsa:4096 -sha256 -days 3650 \
  -nodes -keyout ~/.config/hwledger/server.key.pem \
  -out ~/.config/hwledger/server.cert.pem

chmod 600 ~/.config/hwledger/server.key.pem
```

### CA-signed certificates

For production, use a trusted CA (Let's Encrypt, corporate CA):

```bash
# Generate CSR
openssl req -new -key ~/.config/hwledger/server.key.pem \
  -out server.csr \
  -subj "/CN=fleetserver.example.com"

# Submit to CA, receive signed cert
# Install signed cert
cp server.cert.pem ~/.config/hwledger/server.cert.pem

# Verify
openssl x509 -in ~/.config/hwledger/server.cert.pem -text -noout | grep Issuer
```

## Cloud API keys

Store API keys in environment variables or secure config files.

### Vast.ai API key

```bash
# Never commit to git
echo "HWLEDGER_CLOUD_API_KEY_VAST=vast-api-key-here" >> ~/.bashrc

# Or in config
cat > ~/.config/hwledger/cloud.toml << 'EOF'
[providers.vast]
api_key = "vast-api-key-here"
EOF

chmod 600 ~/.config/hwledger/cloud.toml
```

### RunPod API key

```bash
export HWLEDGER_CLOUD_API_KEY_RUNPOD="runpod-api-key-here"
```

## SSH key management

For agentless fleet mode:

```bash
# Generate key for agents
ssh-keygen -t ed25519 -f ~/.ssh/hwledger-agents -N ""

# Distribute public key to agents
ssh-copy-id -i ~/.ssh/hwledger-agents.pub root@agent1.example.com

# Add to SSH agent
ssh-add ~/.ssh/hwledger-agents

# Configure hwledger
cat > ~/.config/hwledger/ssh-agents.toml << 'EOF'
[[agents]]
name = "agent1"
hostname = "agent1.example.com"
ssh_key_path = "~/.ssh/hwledger-agents"
EOF
```

## 1Password / Bitwarden integration (optional)

Store secrets in password manager:

```bash
# Load API key from 1Password before running
export HWLEDGER_CLOUD_API_KEY_VAST=$(op read op://hwledger/vast-api-key/password)

hwledger cloud list
```

## Secret rotation

### Sparkle key rotation

1. Generate new key (see above)
2. Update CI/CD secrets (GitHub, etc.)
3. Rebuild and re-notarize macOS binary
4. Release new version with new key in appcast

### Fleet server certificate rotation

1. Generate new cert + key
2. Update `server.toml`
3. Restart server: `sudo systemctl restart hwledger-server`
4. Agents auto-refresh on next heartbeat

### Cloud API key rotation

1. Generate new key in provider dashboard (Vast, RunPod, etc.)
2. Update environment variable or config
3. No restart needed (env vars read per request)

## Audit trail

All secret access is logged:

```bash
# Check if API key was used
journalctl -u hwledger-server | grep "api_call"

# Review Vast.ai interactions
hwledger audit --since "2026-04-17T00:00:00Z" | grep vast
```

## Disposal

```bash
# Securely wipe files
shred -u ~/.config/hwledger/sparkle-key.pem
shred -u ~/.config/hwledger/server.key.pem

# Or use dd for entire disk
dd if=/dev/zero of=~/.config/hwledger bs=1M status=progress
```

## Related

- [Deployment Guide](/guides/deployment)
- [Troubleshooting](/guides/troubleshooting)
