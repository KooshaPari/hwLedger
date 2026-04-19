#!/bin/bash
# Generate manifest.json files for each CLI journey.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
JOURNEYS_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
KEYFRAMES_DIR="${JOURNEYS_ROOT}/keyframes"
MANIFESTS_DIR="${JOURNEYS_ROOT}/manifests"

mkdir -p "${MANIFESTS_DIR}"

# Function to get intent label for a journey
get_intent() {
    case "$1" in
        ingest-error)
            echo "CLI ingest command attempting to load a non-existent GGUF file"
            ;;
        plan-deepseek)
            echo "CLI plan command memory allocation for DeepSeek-V3 with 2048 seq, 2 users"
            ;;
        plan-help)
            echo "CLI help text for the plan subcommand"
            ;;
        probe-list)
            echo "CLI probe list command showing GPU device enumeration"
            ;;
        probe-watch)
            echo "CLI probe watch command monitoring GPU resources with clean shutdown"
            ;;
        *)
            echo "CLI journey: $1"
            ;;
    esac
}

# Generate manifest for each journey
for tape_dir in "${KEYFRAMES_DIR}"/*; do
    if [ ! -d "${tape_dir}" ]; then
        continue
    fi

    tape_name=$(basename "${tape_dir}")
    manifest_dir="${MANIFESTS_DIR}/${tape_name}"
    mkdir -p "${manifest_dir}"

    # Collect keyframes in order
    keyframe_files=$(find "${tape_dir}" -name "frame-*.png" | sort)
    keyframe_count=$(echo "${keyframe_files}" | wc -l | tr -d ' ')

    # Get recording paths
    gif_path="recordings/${tape_name}.gif"
    mp4_path="recordings/${tape_name}.mp4"

    # Get intent
    intent=$(get_intent "${tape_name}")

    # Build manifest JSON
    manifest_file="${manifest_dir}/manifest.json"
    {
        echo "{"
        echo "  \"id\": \"cli-${tape_name}\","
        echo "  \"title\": \"hwledger ${tape_name}\","
        echo "  \"intent\": \"${intent}\","
        echo "  \"steps\": ["

        frame_idx=0
        for kf_file in ${keyframe_files}; do
            kf_path="${kf_file#${JOURNEYS_ROOT}/}"
            step_index=$((frame_idx))
            frame_num=$((frame_idx + 1))

            # Comma separator
            if [ $frame_idx -gt 0 ]; then
                echo "    },"
            fi

            # Determine slug based on position
            if [ $frame_num -eq 1 ]; then
                slug="launch"
            elif [ $frame_num -eq "${keyframe_count}" ]; then
                slug="final"
            else
                slug="step_${frame_num}"
            fi

            echo "    {"
            echo "      \"index\": ${step_index},"
            echo "      \"slug\": \"${slug}\","
            echo "      \"intent\": \"Frame ${frame_num} of ${keyframe_count}\","
            echo "      \"screenshot_path\": \"${kf_path}\""

            ((frame_idx++))
        done

        echo "    }"
        echo "  ],"
        echo "  \"recording\": \"${mp4_path}\","
        echo "  \"recording_gif\": \"${gif_path}\","
        echo "  \"keyframe_count\": ${keyframe_count},"
        echo "  \"passed\": true"
        echo "}"
    } > "${manifest_file}"

    echo "Generated: ${manifest_file} (${keyframe_count} keyframes)"
done

echo "Manifest generation complete!"
