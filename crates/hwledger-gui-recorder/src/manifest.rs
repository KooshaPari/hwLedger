//! Journey manifest generation and serialization.
//!
//! Produces JSON manifest linking screenshots, timestamps, and intent labels
//! from XCUITest journey recordings.

use crate::error::RecorderResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Journey manifest linking frames, timestamps, and UI interaction intents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JourneyManifest {
    /// Unique journey identifier (UUID or human-readable ID).
    pub journey_id: String,

    /// Human-readable journey name (e.g., "planner-gui-launch").
    pub name: String,

    /// Recording duration in seconds.
    pub duration_secs: f64,

    /// Keyframe metadata, ordered by timestamp.
    pub keyframes: Vec<KeyframeInfo>,

    /// Path to GIF preview (relative to manifest).
    pub gif_path: PathBuf,

    /// Path to MP4 recording (relative to manifest).
    pub recording_path: PathBuf,

    /// Timestamp when manifest was generated.
    pub generated_at: DateTime<Utc>,

    /// Optional tags for categorization (e.g., ["onboarding", "macos"]).
    pub tags: Vec<String>,
}

/// Single keyframe metadata with timestamp and optional intent label.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyframeInfo {
    /// Sequence number (1-indexed).
    pub frame_num: u32,

    /// Relative path to PNG file.
    pub path: PathBuf,

    /// Estimated timestamp in recording (seconds).
    pub timestamp_secs: f64,

    /// Optional intent label from XCUITest AppDriver (e.g., "tap_planner_button").
    pub intent: Option<String>,

    /// Optional description of what the user sees at this frame.
    pub description: Option<String>,
}

impl JourneyManifest {
    /// Create a new manifest from directory contents.
    ///
    /// Scans `keyframes_dir` for PNG files, orders them chronologically,
    /// and estimates timestamps based on file count and recording duration.
    pub async fn from_directory(
        journey_dir: &Path,
        keyframes_dir: &Path,
    ) -> RecorderResult<Self> {
        let journey_id = journey_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&Uuid::new_v4().to_string())
            .to_string();

        let mut keyframes = Vec::new();
        let mut entries = tokio::fs::read_dir(keyframes_dir).await?;

        let mut files: Vec<_> = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path
                .extension()
                .map(|ext| ext == "png")
                .unwrap_or(false)
            {
                files.push(path);
            }
        }

        // Sort files lexicographically (keyframe-001, keyframe-002, ...)
        files.sort();

        let frame_count = files.len() as f64;
        let duration_secs = if frame_count > 1.0 {
            frame_count * 2.0 // Estimate: 2 seconds per frame on average
        } else {
            10.0
        };

        for (idx, file_path) in files.iter().enumerate() {
            let frame_num = (idx + 1) as u32;
            let timestamp_secs = if frame_count > 1.0 {
                (idx as f64 / (frame_count - 1.0)) * duration_secs
            } else {
                duration_secs / 2.0
            };

            let relative_path = file_path
                .file_name()
                .map(|n| PathBuf::from("keyframes").join(n))
                .unwrap_or_else(|| PathBuf::from("unknown"));

            keyframes.push(KeyframeInfo {
                frame_num,
                path: relative_path,
                timestamp_secs,
                intent: None,
                description: None,
            });
        }

        let name = journey_id.clone();
        Ok(JourneyManifest {
            journey_id,
            name,
            duration_secs,
            keyframes,
            gif_path: PathBuf::from("preview.gif"),
            recording_path: PathBuf::from("recording.mp4"),
            generated_at: Utc::now(),
            tags: vec!["gui-journey".to_string()],
        })
    }

    /// Merge intent labels from an XCUITest AppDriver result.
    ///
    /// Matches AppDriver event timestamps to nearest keyframe and annotates
    /// with intent label (e.g., "user_tapped_button").
    pub fn merge_intents(&mut self, intents: &[(f64, String)]) {
        for (timestamp, intent) in intents {
            if let Some(kf) = self.keyframes.iter_mut().min_by_key(|k| {
                ((k.timestamp_secs - timestamp).abs() * 1000.0) as i32
            }) {
                kf.intent = Some(intent.clone());
            }
        }
    }
}

/// Manifest serialization to JSON.
pub struct ManifestWriter {
    path: PathBuf,
}

impl ManifestWriter {
    /// Create a new manifest writer targeting the given path.
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
        }
    }

    /// Write manifest to JSON file.
    pub async fn write(&self, manifest: &JourneyManifest) -> RecorderResult<()> {
        let json = serde_json::to_string_pretty(manifest)?;
        tokio::fs::write(&self.path, json).await?;

        tracing::info!("manifest written: {}", self.path.display());
        Ok(())
    }

    /// Read manifest from JSON file.
    pub async fn read(&self) -> RecorderResult<JourneyManifest> {
        let json = tokio::fs::read_to_string(&self.path).await?;
        let manifest = serde_json::from_str(&json)?;
        Ok(manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_journey_manifest_from_empty_dir() -> RecorderResult<()> {
        let temp_dir = tempfile::tempdir()?;
        let kf_dir = temp_dir.path().join("keyframes");
        tokio::fs::create_dir(&kf_dir).await?;

        let manifest = JourneyManifest::from_directory(temp_dir.path(), &kf_dir).await?;
        assert_eq!(manifest.keyframes.len(), 0);
        assert!(!manifest.journey_id.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_keyframe_merge_intents() -> RecorderResult<()> {
        let temp_dir = tempfile::tempdir()?;
        let kf_dir = temp_dir.path().join("keyframes");
        tokio::fs::create_dir(&kf_dir).await?;

        tokio::fs::write(kf_dir.join("keyframe-001.png"), b"fake").await?;

        let mut manifest =
            JourneyManifest::from_directory(temp_dir.path(), &kf_dir).await?;

        let intents = vec![(0.5, "tap_button".to_string())];
        manifest.merge_intents(&intents);

        assert_eq!(manifest.keyframes[0].intent, Some("tap_button".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn test_manifest_roundtrip() -> RecorderResult<()> {
        let temp_dir = tempfile::tempdir()?;
        let manifest_path = temp_dir.path().join("manifest.json");

        let manifest = JourneyManifest {
            journey_id: "test-journey".to_string(),
            name: "Test Journey".to_string(),
            duration_secs: 30.0,
            keyframes: vec![
                KeyframeInfo {
                    frame_num: 1,
                    path: PathBuf::from("keyframes/frame-001.png"),
                    timestamp_secs: 0.0,
                    intent: Some("start".to_string()),
                    description: None,
                },
            ],
            gif_path: PathBuf::from("preview.gif"),
            recording_path: PathBuf::from("recording.mp4"),
            generated_at: Utc::now(),
            tags: vec!["test".to_string()],
        };

        let writer = ManifestWriter::new(&manifest_path);
        writer.write(&manifest).await?;

        let loaded = writer.read().await?;
        assert_eq!(loaded.journey_id, "test-journey");
        assert_eq!(loaded.keyframes.len(), 1);
        Ok(())
    }
}
