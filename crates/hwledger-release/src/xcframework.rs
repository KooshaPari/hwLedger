//! XCFramework build orchestration.

use crate::error::ReleaseResult;
use crate::subprocess::ReleaseCommand;
use std::path::Path;
use tracing::info;

#[derive(Debug, Clone, Copy)]
pub enum BuildMode {
    Release,
    Debug,
}

impl BuildMode {
    pub fn cargo_flag(&self) -> Option<&str> {
        match self {
            BuildMode::Release => Some("--release"),
            BuildMode::Debug => None,
        }
    }
}

pub fn build_xcframework(
    repo_root: &Path,
    mode: BuildMode,
    universal: bool,
) -> ReleaseResult<String> {
    info!("building XCFramework (mode={:?}, universal={})", mode, universal);

    let script_path = repo_root.join("scripts/build-xcframework.sh");
    let mut cmd = ReleaseCommand::new(script_path.to_str().unwrap());

    match mode {
        BuildMode::Release => {
            cmd = cmd.arg("--release");
        }
        BuildMode::Debug => {
            cmd = cmd.arg("--debug");
        }
    }

    if universal {
        cmd = cmd.arg("--universal");
    }

    cmd.timeout(600).run()?;

    let xcframework_path = repo_root
        .join("apps/macos/xcframework/HwLedgerCore.xcframework")
        .to_string_lossy()
        .to_string();

    info!("XCFramework built: {}", xcframework_path);
    Ok(xcframework_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_mode_flags() {
        assert_eq!(BuildMode::Release.cargo_flag(), Some("--release"));
        assert_eq!(BuildMode::Debug.cargo_flag(), None);
    }
}
