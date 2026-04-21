//! Typed Rust mirror of the enriched manifest shape the TS pipeline expects.
//! Kept intentionally loose (all-optional) so we don't reject canonical
//! phenotype-journeys manifests that lack enrichment fields.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub bbox: [u32; 4],
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JourneyStep {
    pub index: u32,
    pub slug: String,
    pub intent: String,
    pub screenshot_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blind_description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<Annotation>>,
    /// Preserve opaque verification-ground-truth assertions without modelling them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assertions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneSpec {
    pub step: u32,
    #[serde(rename = "calloutText")]
    pub callout_text: String,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "calloutSubText")]
    pub callout_sub_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "calloutColor")]
    pub callout_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "durationFrames")]
    pub duration_frames: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceoverSpec {
    pub backend: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RichManifest {
    pub id: String,
    pub intent: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recording: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recording_gif: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recording_rich: Option<String>,
    #[serde(default)]
    pub keyframe_count: u32,
    #[serde(default)]
    pub passed: bool,
    #[serde(default)]
    pub steps: Vec<JourneyStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenes: Option<Vec<SceneSpec>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voiceover: Option<VoiceoverSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annotated_keyframes: Option<Vec<String>>,
    /// Preserve verification block if present on the source manifest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<serde_json::Value>,
}
