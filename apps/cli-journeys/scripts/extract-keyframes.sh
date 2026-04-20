#!/bin/bash
# Extract keyframes from CLI journey MP4 recordings.
# Usage: ./extract-keyframes.sh [tape-name]
# If no tape-name provided, extracts from all MP4s in recordings/

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
JOURNEYS_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
RECORDINGS_DIR="${JOURNEYS_ROOT}/recordings"
KEYFRAMES_DIR="${JOURNEYS_ROOT}/keyframes"

# Check if ffmpeg is available
if ! command -v ffmpeg &> /dev/null; then
    echo "Error: ffmpeg not found. Install with: brew install ffmpeg"
    exit 1
fi

mkdir -p "${KEYFRAMES_DIR}"

# Determine which recordings to process
RECORDINGS=""
if [ $# -eq 1 ]; then
    TAPE_NAME="$1"
    RECORDING="${RECORDINGS_DIR}/${TAPE_NAME}.mp4"
    if [ ! -f "${RECORDING}" ]; then
        echo "Error: Recording not found: ${RECORDING}"
        exit 1
    fi
    RECORDINGS="${RECORDING}"
else
    # Process all MP4 files
    for f in $(find "${RECORDINGS_DIR}" -name "*.mp4" | sort); do
        RECORDINGS="${RECORDINGS}${f}"$'\n'
    done
fi

if [ -z "${RECORDINGS}" ]; then
    echo "No recordings found in ${RECORDINGS_DIR}"
    exit 1
fi

echo "${RECORDINGS}" | while IFS= read -r recording; do
    [ -z "${recording}" ] && continue
    recording="${recording}"
    # Get tape name from file
    tape_name=$(basename "${recording}" .mp4)
    tape_keyframes_dir="${KEYFRAMES_DIR}/${tape_name}"

    mkdir -p "${tape_keyframes_dir}"

    echo "Extracting keyframes from: ${tape_name}..."

    # Always clear stale frames from prior runs — a shorter re-recording
    # would otherwise leave old frame-NNN.png orphans mixed with new ones.
    rm -f "${tape_keyframes_dir}"/frame-*.png

    # Extract I-frames (true keyframes); if none, fallback to 1 fps
    ffmpeg -i "${recording}" -vf "select='eq(pict_type,I)'" -vsync vfr -q:v 2 \
        "${tape_keyframes_dir}/frame-%03d.png" 2>&1 | grep -v "Press \[q\] to stop" || true

    # Check how many we got
    keyframe_count=$(find "${tape_keyframes_dir}" -name "frame-*.png" 2>/dev/null | wc -l)

    if [ "${keyframe_count}" -lt 3 ]; then
        echo "  Only ${keyframe_count} I-frames; extracting steady 1 fps sample..."
        rm -f "${tape_keyframes_dir}"/frame-*.png
        ffmpeg -i "${recording}" -vf "fps=1" -q:v 2 \
            "${tape_keyframes_dir}/frame-%03d.png" 2>&1 | grep -v "Press \[q\] to stop" || true
        keyframe_count=$(find "${tape_keyframes_dir}" -name "frame-*.png" 2>/dev/null | wc -l)
    fi

    echo "  Extracted ${keyframe_count} keyframes to ${tape_keyframes_dir}/"
done

echo "Keyframe extraction complete!"
