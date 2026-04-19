//! FFmpeg keyframe extraction and manifest generation.

use crate::error::ReleaseResult;
use crate::subprocess::ReleaseCommand;
use std::path::Path;
use tracing::info;

pub fn extract_keyframes(
    tape_path: &Path,
    output_dir: &Path,
) -> ReleaseResult<()> {
    info!(
        "extracting keyframes from: {} -> {}",
        tape_path.display(),
        output_dir.display()
    );

    std::fs::create_dir_all(output_dir)?;

    let out_pattern = format!("{}/frame_%04d.png", output_dir.display());
    ReleaseCommand::new("ffmpeg")
        .arg("-i")
        .arg(tape_path.to_str().unwrap())
        .arg("-vf")
        .arg("select=eq(pict_type\\,I)")
        .arg("-vsync")
        .arg("0")
        .arg(&out_pattern)
        .timeout(300)
        .run()?;

    info!("keyframes extracted to: {}", output_dir.display());
    Ok(())
}

pub fn generate_manifest(
    tape_id: &str,
    keyframes_dir: &Path,
    output_manifest: &Path,
) -> ReleaseResult<()> {
    info!(
        "generating manifest for tape: {} -> {}",
        tape_id,
        output_manifest.display()
    );

    let manifest = format!(
        r#"{{
  "tape_id": "{}",
  "keyframes_dir": "{}",
  "generated_at": "{}"
}}"#,
        tape_id,
        keyframes_dir.display(),
        chrono::Local::now().to_rfc3339()
    );

    std::fs::write(output_manifest, manifest)?;
    info!("manifest written: {}", output_manifest.display());
    Ok(())
}
