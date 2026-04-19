//! DMG creation and codesigning.
//!
//! Wraps hdiutil and codesign commands to build a signed DMG for distribution.

use crate::error::ReleaseResult;
use crate::subprocess::ReleaseCommand;
use std::path::Path;
use tracing::info;

/// Build a signed DMG from a .app bundle.
///
/// Equivalent to `./scripts/build-dmg.sh --app <path> --out <output>`.
pub fn build_dmg(
    repo_root: &Path,
    app_path: &Path,
    output_path: &Path,
    codesign_identity: Option<&str>,
) -> ReleaseResult<()> {
    info!(
        "building DMG: {} -> {}",
        app_path.display(),
        output_path.display()
    );

    let script_path = repo_root.join("scripts/build-dmg.sh");

    let mut cmd = ReleaseCommand::new(script_path.to_str().unwrap());
    cmd = cmd
        .arg("--app")
        .arg(app_path.to_str().unwrap())
        .arg("--out")
        .arg(output_path.to_str().unwrap());

    if let Some(identity) = codesign_identity {
        cmd = cmd.arg("--codesign-identity").arg(identity);
    }

    cmd.timeout(120).run()?;

    info!("DMG built: {}", output_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dmg_params() {
        // Placeholder for integration test
    }
}
