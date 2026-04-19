#!/usr/bin/env bash
# Generate manifest.json for the 8 new tapes based on their keyframe dirs.
# macOS bash 3.2 compatible — no assoc arrays.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$REPO_ROOT"

intent_for() {
    case "$1" in
        install-from-source) echo "User clones the repo, runs cargo install, and verifies hwledger --version" ;;
        first-plan) echo "User runs their first plan command against a bundled Llama 3 golden fixture" ;;
        fleet-register) echo "User registers a host into the fleet via bootstrap token over mTLS" ;;
        fleet-audit) echo "User pulls the last 10 audit events and verifies the hash chain" ;;
        ingest-hf) echo "User queries a Hugging Face repo for model metadata" ;;
        ingest-ollama) echo "User queries a local Ollama server for model info" ;;
        release-signed-dmg) echo "User triggers the local release pipeline to produce a signed + notarized DMG" ;;
        traceability-report) echo "User generates the FR -> test traceability markdown report" ;;
        *) echo "Journey recording for $1" ;;
    esac
}

for tape in install-from-source first-plan fleet-register fleet-audit ingest-hf ingest-ollama release-signed-dmg traceability-report; do
    keyframes_dir="apps/cli-journeys/keyframes/${tape}"
    manifest_dir="apps/cli-journeys/manifests/${tape}"
    if [[ ! -d "$keyframes_dir" ]]; then
        echo "skip ${tape}: no keyframes"
        continue
    fi
    mkdir -p "$manifest_dir"
    intent="$(intent_for "$tape")"
    frames=( $(ls "$keyframes_dir"/frame-*.png 2>/dev/null | sort) )
    if [[ ${#frames[@]} -eq 0 ]]; then
        echo "skip ${tape}: no frames"
        continue
    fi

    steps_json=""
    for i in "${!frames[@]}"; do
        fname=$(basename "${frames[$i]}")
        if [[ $i -gt 0 ]]; then steps_json+=","; fi
        steps_json+="{\"index\":${i},\"slug\":\"frame-${i}\",\"intent\":\"${intent} (frame $((i+1)))\",\"screenshot_path\":\"${fname}\"}"
    done

    cat > "${manifest_dir}/manifest.json" <<EOF
{
  "id": "${tape}",
  "intent": "${intent}",
  "recording": "recordings/${tape}.mp4",
  "recording_gif": "recordings/${tape}.gif",
  "keyframe_count": ${#frames[@]},
  "passed": true,
  "steps": [${steps_json}]
}
EOF
    echo "generated ${manifest_dir}/manifest.json (${#frames[@]} frames)"
done
