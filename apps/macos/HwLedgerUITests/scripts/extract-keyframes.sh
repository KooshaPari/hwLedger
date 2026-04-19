#!/bin/bash
# Extract keyframes from a journey's recording.mp4 using ffmpeg.
# Usage: ./scripts/extract-keyframes.sh <journey-id>

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
JOURNEY_ID="${1:-}"

if [ -z "${JOURNEY_ID}" ]; then
    echo "Usage: $0 <journey-id>"
    echo "Example: $0 planner-qwen2-7b-32k"
    exit 1
fi

JOURNEY_DIR="${PROJECT_ROOT}/journeys/${JOURNEY_ID}"
RECORDING="${JOURNEY_DIR}/recording.mp4"
KEYFRAME_DIR="${JOURNEY_DIR}/keyframes"

# Check if ffmpeg is available
if ! command -v ffmpeg &> /dev/null; then
    echo "Error: ffmpeg not found. Install with: brew install ffmpeg"
    exit 1
fi

# Check if recording exists
if [ ! -f "${RECORDING}" ]; then
    echo "Warning: No recording found at ${RECORDING}"
    echo "Skipping keyframe extraction (journey may have been recorded with recording_denied=true)"
    exit 0
fi

# Create keyframe directory
mkdir -p "${KEYFRAME_DIR}"

echo "Extracting keyframes from ${RECORDING}..."

# Extract I-frames (true keyframes)
ffmpeg -i "${RECORDING}" -vf "select='eq(pict_type,I)'" -vsync vfr -q:v 2 \
    "${KEYFRAME_DIR}/keyframe-%03d.png" 2>&1 | grep -v "Press \[q\] to stop"

# If we got fewer than 3 keyframes, fallback to steady sampling (1 frame every 3 seconds)
KEYFRAME_COUNT=$(ls "${KEYFRAME_DIR}"/keyframe-*.png 2>/dev/null | wc -l)
if [ "${KEYFRAME_COUNT}" -lt 3 ]; then
    echo "Found only ${KEYFRAME_COUNT} I-frames; extracting steady sample at 1 fps (1 frame/second)..."
    rm -f "${KEYFRAME_DIR}"/keyframe-*.png
    ffmpeg -i "${RECORDING}" -vf "fps=1" -q:v 2 \
        "${KEYFRAME_DIR}/keyframe-%03d.png" 2>&1 | grep -v "Press \[q\] to stop"
fi

FINAL_COUNT=$(ls "${KEYFRAME_DIR}"/keyframe-*.png 2>/dev/null | wc -l)
echo "Extracted ${FINAL_COUNT} keyframes to ${KEYFRAME_DIR}/"

# Also create an optimized GIF for quick preview
echo "Creating optimized GIF preview..."
ffmpeg -i "${RECORDING}" \
    -filter_complex "fps=10,scale=720:-1:flags=lanczos[s];[s]split[a][b];[a]palettegen=max_colors=256:stats_mode=diff[pal];[b][pal]paletteuse=dither=bayer:bayer_scale=5:diff_mode=rectangle" \
    -loop 0 \
    "${JOURNEY_DIR}/preview.gif" 2>&1 | grep -v "Press \[q\] to stop"

echo "Done. Keyframes ready for VLM verification."
