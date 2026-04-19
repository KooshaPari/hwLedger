//! `FFmpeg` subprocess wrapper for keyframe extraction and GIF generation.
//!
//! Reuses patterns from hwledger-release crate. Wraps ffmpeg subprocess calls
//! with timeout, logging, and error handling.

use crate::error::{RecorderError, RecorderResult};
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;
use tracing::info;

#[expect(
    dead_code,
    reason = "scaffold for future GUI recorder integration — see docs/checklists/GUI-RECORDING-INTEGRATION-CHECKLIST.md"
)]
const FFMPEG_TIMEOUT_SECS: u64 = 300;

/// Extract I-frames (true keyframes) from MP4 recording.
///
/// Falls back to steady sampling (1 fps) if fewer than 3 I-frames found.
pub async fn extract_i_frames(
    recording_path: &Path,
    output_dir: &Path,
    pattern: &str,
) -> RecorderResult<usize> {
    info!(
        "extracting I-frames from {} -> {} (pattern: {})",
        recording_path.display(),
        output_dir.display(),
        pattern
    );

    if !recording_path.exists() {
        return Err(RecorderError::RecordingNotFound(recording_path.to_path_buf()));
    }

    let out_pattern = format!("{}/{}", output_dir.display(), pattern);

    // Try I-frame extraction first
    let status = Command::new("ffmpeg")
        .arg("-i")
        .arg(recording_path.to_str().unwrap())
        .arg("-vf")
        .arg("select='eq(pict_type,I)'")
        .arg("-vsync")
        .arg("vfr")
        .arg("-q:v")
        .arg("2")
        .arg(&out_pattern)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                RecorderError::FfmpegNotFound
            } else {
                RecorderError::FfmpegFailed(e.to_string())
            }
        })?;

    if !status.success() {
        return Err(RecorderError::FfmpegFailed("I-frame extraction failed".to_string()));
    }

    // Count extracted frames
    let frame_count = count_keyframes(output_dir).await?;

    if frame_count < 3 {
        info!("only {} I-frames extracted; falling back to steady sampling at 1 fps", frame_count);

        // Clean up partial extraction
        clean_keyframes(output_dir).await?;

        // Fallback: extract 1 frame per second
        let status = Command::new("ffmpeg")
            .arg("-i")
            .arg(recording_path.to_str().unwrap())
            .arg("-vf")
            .arg("fps=1")
            .arg("-q:v")
            .arg("2")
            .arg(&out_pattern)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map_err(|e| RecorderError::FfmpegFailed(e.to_string()))?;

        if !status.success() {
            return Err(RecorderError::FfmpegFailed("fallback frame sampling failed".to_string()));
        }

        let final_count = count_keyframes(output_dir).await?;
        info!("extracted {} frames via steady sampling", final_count);
        Ok(final_count)
    } else {
        info!("extracted {} I-frames", frame_count);
        Ok(frame_count)
    }
}

/// Generate optimized GIF preview from recording.
///
/// Uses palette-based encoding with dithering for smaller file size.
pub async fn generate_gif(recording_path: &Path, gif_path: &Path, fps: u32) -> RecorderResult<()> {
    info!(
        "generating GIF preview from {} -> {} (fps: {})",
        recording_path.display(),
        gif_path.display(),
        fps
    );

    if !recording_path.exists() {
        return Err(RecorderError::RecordingNotFound(recording_path.to_path_buf()));
    }

    let filter = format!(
        "fps={},scale=720:-1:flags=lanczos[s];[s]split[a][b];[a]palettegen=max_colors=256:stats_mode=diff[pal];[b][pal]paletteuse=dither=bayer:bayer_scale=5:diff_mode=rectangle",
        fps
    );

    let status = Command::new("ffmpeg")
        .arg("-i")
        .arg(recording_path.to_str().unwrap())
        .arg("-filter_complex")
        .arg(&filter)
        .arg("-loop")
        .arg("0")
        .arg(gif_path.to_str().unwrap())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                RecorderError::FfmpegNotFound
            } else {
                RecorderError::FfmpegFailed(e.to_string())
            }
        })?;

    if !status.success() {
        return Err(RecorderError::FfmpegFailed("GIF generation failed".to_string()));
    }

    info!("GIF preview generated: {}", gif_path.display());
    Ok(())
}

/// Count PNG keyframe files in output directory.
async fn count_keyframes(output_dir: &Path) -> RecorderResult<usize> {
    let mut count = 0;
    let mut entries = tokio::fs::read_dir(output_dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().map(|ext| ext == "png").unwrap_or(false) {
            count += 1;
        }
    }

    Ok(count)
}

/// Remove all PNG files from keyframes directory.
async fn clean_keyframes(output_dir: &Path) -> RecorderResult<()> {
    let mut entries = tokio::fs::read_dir(output_dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().map(|ext| ext == "png").unwrap_or(false) {
            tokio::fs::remove_file(path).await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_count_keyframes_empty() -> RecorderResult<()> {
        let temp_dir = tempfile::tempdir()?;
        let count = count_keyframes(temp_dir.path()).await?;
        assert_eq!(count, 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_count_keyframes_with_files() -> RecorderResult<()> {
        let temp_dir = tempfile::tempdir()?;
        tokio::fs::write(temp_dir.path().join("frame_001.png"), b"fake").await?;
        tokio::fs::write(temp_dir.path().join("frame_002.png"), b"fake").await?;
        tokio::fs::write(temp_dir.path().join("other.txt"), b"ignored").await?;

        let count = count_keyframes(temp_dir.path()).await?;
        assert_eq!(count, 2);
        Ok(())
    }
}
