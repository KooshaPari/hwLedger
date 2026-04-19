//! GUI recording harness for hwLedger `SwiftUI` app.
//!
//! Provides process-isolated screen recording via `ScreenCaptureKit` (macOS 14+),
//! with ffmpeg-based keyframe extraction and manifest generation for journey verification.
//!
//! # Permissions
//!
//! Requires macOS Screen Recording permission (granted on first use). Fails gracefully
//! if permission is denied.
//!
//! # Example
//!
//! ```ignore
//! use hwledger_gui_recorder::{ScreenRecorder, JourneyRecorder};
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let recorder = ScreenRecorder::new(
//!         PathBuf::from("/tmp/recording.mp4"),
//!         1440,
//!         900,
//!         30,
//!     );
//!
//!     recorder.start_recording("com.kooshapari.hwLedger").await?;
//!     // ... perform app interactions ...
//!     recorder.stop_recording().await?;
//!
//!     // Extract keyframes and generate manifest
//!     let journey = JourneyRecorder::new(
//!         PathBuf::from("/tmp/recording.mp4"),
//!         PathBuf::from("/tmp/journey"),
//!     );
//!     journey.extract_all().await?;
//!
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod ffmpeg;
pub mod manifest;
pub mod recorder;

pub use error::{RecorderError, RecorderResult};
pub use manifest::{JourneyManifest, KeyframeInfo, ManifestWriter};
pub use recorder::{RecordingConfig, ScreenRecorder};

cfg_if::cfg_if! {
    if #[cfg(target_os = "macos")] {
        pub mod sck_bridge;
        pub use sck_bridge::check_screen_capture_permission;
    }
}

/// Main orchestrator for GUI journey recording and keyframe extraction.
pub struct JourneyRecorder {
    recording_path: std::path::PathBuf,
    journey_dir: std::path::PathBuf,
    config: RecordingConfig,
}

impl JourneyRecorder {
    /// Create a new JourneyRecorder for a journey.
    ///
    /// # Arguments
    ///
    /// * `recording_path` - Path to the MP4 file produced by ScreenRecorder
    /// * `journey_dir` - Root directory for journey output (keyframes, manifest, GIF)
    pub fn new(recording_path: std::path::PathBuf, journey_dir: std::path::PathBuf) -> Self {
        Self { recording_path, journey_dir, config: RecordingConfig::default() }
    }

    /// Fully process a recorded journey: extract keyframes, GIF, and generate manifest.
    pub async fn extract_all(&self) -> RecorderResult<JourneyManifest> {
        // Create output directories
        tokio::fs::create_dir_all(&self.journey_dir).await?;

        let keyframes_dir = self.journey_dir.join("keyframes");
        let gif_path = self.journey_dir.join("preview.gif");

        // Extract I-frames and GIF
        self.extract_keyframes(&keyframes_dir).await?;
        self.generate_gif(&gif_path).await?;

        // Generate manifest
        let manifest = JourneyManifest::from_directory(&self.journey_dir, &keyframes_dir).await?;

        // Write manifest
        let manifest_path = self.journey_dir.join("manifest.json");
        let writer = ManifestWriter::new(&manifest_path);
        writer.write(&manifest).await?;

        tracing::info!(
            "journey recording complete: manifest={}, keyframes={}, gif={}",
            manifest_path.display(),
            keyframes_dir.display(),
            gif_path.display()
        );

        Ok(manifest)
    }

    async fn extract_keyframes(&self, keyframes_dir: &std::path::Path) -> RecorderResult<()> {
        tracing::info!(
            "extracting keyframes from {} -> {}",
            self.recording_path.display(),
            keyframes_dir.display()
        );

        tokio::fs::create_dir_all(keyframes_dir).await?;

        ffmpeg::extract_i_frames(
            &self.recording_path,
            keyframes_dir,
            &self.config.keyframe_pattern,
        )
        .await?;

        Ok(())
    }

    async fn generate_gif(&self, gif_path: &std::path::Path) -> RecorderResult<()> {
        tracing::info!("generating optimized GIF preview -> {}", gif_path.display());

        ffmpeg::generate_gif(&self.recording_path, gif_path, self.config.gif_fps).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_journey_recorder_new() {
        let recorder = JourneyRecorder::new(
            std::path::PathBuf::from("/tmp/rec.mp4"),
            std::path::PathBuf::from("/tmp/journey"),
        );
        assert_eq!(recorder.recording_path.to_str().unwrap(), "/tmp/rec.mp4");
    }
}
