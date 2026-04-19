#!/usr/bin/env bash
# Generate manifest.json for the 8 new tapes based on their keyframe dirs.
# macOS bash 3.2 compatible — no assoc arrays.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$REPO_ROOT"

intent_for() {
    case "$1" in
        install-cargo) echo "Install hwledger from source with cargo, then verify version and help" ;;
        first-plan) echo "Run your first plan with colored output showing token distribution for 4 users" ;;
        fleet-register) echo "Register a new agent with the fleet, then verify it appears in fleet status" ;;
        fleet-audit) echo "Audit the fleet with a 3-agent limit to see agent metadata and status" ;;
        ingest-error) echo "Show graceful error handling when a model file doesn't exist" ;;
        ingest-local-gguf) echo "Ingest a local GGUF model file and output JSON metadata" ;;
        plan-deepseek) echo "Plan with Deepseek at 2K seq, then at 32K to show KV growth under scaling" ;;
        plan-help) echo "Show the plan command's help text with all available options" ;;
        plan-mla-deepseek) echo "Show MLA classification and KV sequence invariance across 2K, 32K, 128K sequences" ;;
        probe-list) echo "List all available probes in both table and JSON formats" ;;
        probe-watch) echo "Watch probe metrics update in real time with 1-second refresh intervals" ;;
        traceability-report) echo "Generate a markdown traceability report with coverage data and inspect the output" ;;
        traceability-strict) echo "Show strict mode enforcement with passing and failing traceability checks" ;;
        *) echo "Journey recording for $1" ;;
    esac
}

for tape in install-cargo first-plan fleet-register fleet-audit ingest-error ingest-local-gguf plan-deepseek plan-help plan-mla-deepseek probe-list probe-watch traceability-report traceability-strict; do
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
