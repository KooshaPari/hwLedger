//! macOS app bundling and codesigning.

use crate::error::ReleaseResult;
use crate::subprocess::ReleaseCommand;
use std::path::Path;
use tracing::info;

pub fn bundle_app(
    repo_root: &Path,
    app_name: &str,
    bundle_id: &str,
    codesign: bool,
) -> ReleaseResult<String> {
    info!("bundling app: {} (bundle_id={}, codesign={})", app_name, bundle_id, codesign);

    let script_path = repo_root.join("apps/macos/HwLedgerUITests/scripts/bundle-app.sh");

    let mut cmd = ReleaseCommand::new(script_path.to_str().unwrap());
    cmd = cmd.arg("--app-name").arg(app_name);
    cmd = cmd.arg("--bundle-id").arg(bundle_id);

    if codesign {
        cmd = cmd.arg("--codesign");
    }

    cmd.timeout(120).run()?;

    let app_path = repo_root
        .join("apps/build")
        .join(format!("{}.app", app_name))
        .to_string_lossy()
        .to_string();

    info!("app bundled: {}", app_path);
    Ok(app_path)
}

#[cfg(test)]
mod tests {
    

    #[test]
    fn test_bundle_params() {
        // Placeholder for integration test
    }
}
