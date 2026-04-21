---
title: Troubleshooting
description: Common issues and fixes
---

# Troubleshooting

## GPU not detected

<Shot src="/cli-journeys/keyframes/probe-list/frame-002.png"
      caption="Healthy probe output — device row present"
      size="small" align="right" />

<!-- SHOT-TODO: capture an empty probe list result (no-GPU host) -->

**Symptom**: `hwledger probe` returns empty GPU list.

**Diagnosis**:
```bash
hwledger probe --json | jq .gpus
# Returns: []
```

**Fixes**:
1. **Check driver**: `nvidia-smi` / `rocm-smi` / `system_profiler SPDisplaysDataType` (macOS)
2. **Check compute capability**: NVIDIA requires compute capability 3.0+ (Kepler or newer)
3. **Verify env vars**:
   ```bash
   echo $CUDA_VISIBLE_DEVICES  # Should not be empty
   export CUDA_VISIBLE_DEVICES="0"  # Force GPU 0
   ```
4. **macOS Metal**: M1/M2/M3 only. Intel Macs not supported.

## Metal framework missing (macOS)

**Symptom**: Error on macOS with M-chip: "Metal framework not found".

**Fix**:
```bash
brew install metal-tools
# Restart Terminal
```

<Shot src="/cli-journeys/keyframes/ingest-error/frame-001.png"
      caption="Fail-loud error line (E-INGEST-02) — same style as NVML failures"
      size="small" align="right"
      :annotations='[{"bbox":[60,220,480,32],"label":"error code","color":"#f38ba8","style":"dashed"}]' />

## NVML library not found

**Symptom**: NVIDIA GPU detected but: "libnvidia-ml.so not found".

**Fixes**:
```bash
# Linux
sudo apt-get install libnvidia-compute-XXX  # Replace XXX with CUDA version
export LD_LIBRARY_PATH=/usr/local/cuda/lib64:$LD_LIBRARY_PATH

# macOS
brew install nvidia-cuda-toolkit
```

## 0 GB free VRAM

**Symptom**: `hwledger plan --model mistral-7b` fails: "Insufficient VRAM".

**Diagnosis**: Other process hogging GPU memory.

**Fixes**:
```bash
# Check what's using VRAM
nvidia-smi  # Look for Process column

# Kill process
nvidia-smi kill PID

# Or clear all CUDA cache
nvidia-smi --query-compute-apps=pid,used_memory --format=csv,noheader | \
  while read pid mem; do kill -9 $pid 2>/dev/null; done

# Clear VRAM (nuclear option)
sudo nvidia-smi --gpu-reset  # Requires driver reload
```

## Model ingest hangs

**Symptom**: `hwledger ingest --model mistral-7b` stalls indefinitely.

**Diagnosis**: Network issue, HuggingFace API rate-limited, or missing `git-lfs`.

**Fixes**:
```bash
# Check network
curl -I https://huggingface.co/  # Should return 200

# Install git-lfs
brew install git-lfs  # macOS
sudo apt-get install git-lfs  # Linux

# Try with explicit cache dir + verbose
hwledger ingest --model mistral-7b \
  --cache-dir /tmp/hf_cache \
  --log-level debug
```

## Inference timeout

**Symptom**: `hwledger run --model llama-70b input.json` times out after 300 seconds.

**Fixes**:
1. **Increase timeout**: `--timeout 600`
2. **Reduce context**: `--context 4096` (instead of 32K)
3. **Reduce batch**: `--batch 1` (instead of 4)
4. **Use quantization**: `--quant int4` to reduce memory pressure

## Fleet server won't start

**Symptom**: `hwledger server` fails: "Address already in use".

**Fix**:
```bash
# Find what's listening on port 5443
lsof -i :5443
# Kill it
kill -9 PID

# Or use different port
hwledger server --listen 0.0.0.0:5444
```

## Agent can't reach server

**Symptom**: Agent heartbeat fails: "Connection refused" or "CERTIFICATE_VERIFY_FAILED".

**Diagnosis**:
```bash
# Check connectivity
curl -v https://fleet.example.com:5443/health  # Should work with valid cert

# Check agent config
cat ~/.config/hwledger/agent.toml | grep server_addr

# Check cert
openssl s_client -connect fleet.example.com:5443 -showcerts
```

**Fixes**:
1. **Check server is running**: `pgrep hwledger-server` or systemctl status
2. **Check firewall**: `sudo iptables -L` / Security Group (AWS/Azure)
3. **Check DNS**: `nslookup fleet.example.com`
4. **Check cert expiration**: `openssl x509 -in ~/.config/hwledger/server.cert.pem -text -noout | grep -A2 Validity`

## SSH fallback auth fails

**Symptom**: `hwledger fleet register-ssh --host user@remote.box` fails: "Permission denied".

**Fixes**:
1. **Test SSH manually**: `ssh -i ~/.ssh/id_ed25519 user@remote.box nvidia-smi`
2. **Check key permissions**: `chmod 600 ~/.ssh/id_ed25519`
3. **Add to SSH agent**: `ssh-add ~/.ssh/id_ed25519`
4. **Check remote user**: ensure user can run `nvidia-smi` without sudo

## Audit verify fails

**Symptom**: `hwledger audit --verify` fails: "Hash mismatch at event N".

**Diagnosis**: Ledger corrupted or tampered.

**Fix**:
```bash
# Export last known-good backup
hwledger audit --export backup.json --since "2026-04-01T00:00:00Z"

# Reset to known state (WARNING: loses recent events)
rm ~/.cache/hwledger/fleet.db
hwledger server  # Recreates empty DB
```

## Related

- [Configuration](/reference/config)
- [Exit Codes](/reference/exit-codes)
- [FAQ](/guides/faq)
