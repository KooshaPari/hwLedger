//! Screen recording orchestration via ScreenCaptureKit (macOS 14+).

use crate::error::{RecorderError, RecorderResult};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

/// Configuration for screen recording output.
#[derive(Debug, Clone)]
pub struct RecordingConfig {
    /// Output width in pixels (default: 1440).
    pub width: u32,

    /// Output height in pixels (default: 900).
    pub height: u32,

    /// Frames per second (default: 30).
    pub fps: u32,

    /// Video codec (default: h264).
    pub codec: String,

    /// Keyframe extraction pattern (default: "keyframe-%03d.png").
    pub keyframe_pattern: String,

    /// GIF frame rate (default: 10).
    pub gif_fps: u32,
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            width: 1440,
            height: 900,
            fps: 30,
            codec: "h264".to_string(),
            keyframe_pattern: "keyframe-%03d.png".to_string(),
            gif_fps: 10,
        }
    }
}

/// Manages screen recording session via ScreenCaptureKit (Swift).
pub struct ScreenRecorder {
    config: RecordingConfig,
    output_path: PathBuf,
    state: Arc<Mutex<RecorderState>>,
}

/// Internal state machine for recorder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecorderState {
    Idle,
    Recording,
    Stopped,
    PermissionDenied,
}

impl ScreenRecorder {
    /// Create a new ScreenRecorder with default configuration.
    pub fn new(output_path: PathBuf) -> Self {
        Self::with_config(output_path, RecordingConfig::default())
    }

    /// Create a ScreenRecorder with custom configuration.
    pub fn with_config(output_path: PathBuf, config: RecordingConfig) -> Self {
        Self { config, output_path, state: Arc::new(Mutex::new(RecorderState::Idle)) }
    }

    /// Start recording the app window identified by bundle ID.
    ///
    /// # Arguments
    ///
    /// * `app_bundle_id` - Bundle identifier (e.g., "com.kooshapari.hwLedger")
    ///
    /// # Errors
    ///
    /// Returns `PermissionDenied` if Screen Recording permission is not granted.
    pub async fn start_recording(&self, app_bundle_id: &str) -> RecorderResult<()> {
        info!(
            "starting screen recording: app={}, output={}, resolution={}x{}@{}fps",
            app_bundle_id,
            self.output_path.display(),
            self.config.width,
            self.config.height,
            self.config.fps
        );

        let mut state = self.state.lock().await;

        if *state != RecorderState::Idle {
            return Err(RecorderError::InvalidOutputPath(
                "recorder already in use or stopped".to_string(),
            ));
        }

        // Check SCK permission (macOS only)
        #[cfg(target_os = "macos")]
        {
            if !crate::sck_bridge::check_screen_capture_permission()? {
                *state = RecorderState::PermissionDenied;
                return Err(RecorderError::PermissionDenied);
            }
        }

        // Start SCK recording via FFI
        #[cfg(target_os = "macos")]
        {
            crate::sck_bridge::start_recording(
                app_bundle_id,
                self.output_path.to_str().unwrap(),
                self.config.width,
                self.config.height,
                self.config.fps,
            )?;
        }

        *state = RecorderState::Recording;
        info!("recording started: {}", self.output_path.display());
        Ok(())
    }

    /// Stop recording and finalize the MP4 file.
    pub async fn stop_recording(&self) -> RecorderResult<PathBuf> {
        let mut state = self.state.lock().await;

        if *state == RecorderState::PermissionDenied {
            info!(
                "recording was denied permission; output path would be: {}",
                self.output_path.display()
            );
            *state = RecorderState::Stopped;
            return Ok(self.output_path.clone());
        }

        if *state != RecorderState::Recording {
            return Err(RecorderError::NotRecording);
        }

        // Stop SCK recording via FFI
        #[cfg(target_os = "macos")]
        {
            crate::sck_bridge::stop_recording()?;
        }

        *state = RecorderState::Stopped;
        info!("recording stopped: {}", self.output_path.display());
        Ok(self.output_path.clone())
    }

    /// Check if recording is currently in progress.
    pub async fn is_recording(&self) -> bool {
        *self.state.lock().await == RecorderState::Recording
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recording_config_default() {
        let config = RecordingConfig::default();
        assert_eq!(config.width, 1440);
        assert_eq!(config.height, 900);
        assert_eq!(config.fps, 30);
    }

    #[tokio::test]
    async fn test_screen_recorder_new() {
        let recorder = ScreenRecorder::new(PathBuf::from("/tmp/test.mp4"));
        assert!(!recorder.is_recording().await);
    }

    #[tokio::test]
    async fn test_recorder_state_sequence() {
        let recorder = ScreenRecorder::new(PathBuf::from("/tmp/test.mp4"));
        let state = recorder.state.lock().await;
        assert_eq!(*state, RecorderState::Idle);
    }
}
