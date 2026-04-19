//! Apple notarization workflow.
//!
//! Submits a DMG or .app to Apple's notary service and staples the ticket.

use crate::error::{ReleaseError, ReleaseResult};
use crate::subprocess::ReleaseCommand;
use std::path::Path;
use tracing::{info, debug};

/// Probe whether a keychain profile exists for notarytool.
pub fn has_keychain_profile(profile: &str) -> ReleaseResult<bool> {
    debug!("probing keychain profile: {}", profile);
    let result = ReleaseCommand::new("xcrun")
        .arg("notarytool")
        .arg("history")
        .arg("--keychain-profile")
        .arg(profile)
        .timeout(10)
        .run();

    Ok(result.is_ok())
}

/// Notarize a DMG or .app bundle using Apple's notary service.
///
/// Supports two credential modes:
/// 1. Keychain profile (preferred): `xcrun notarytool store-credentials hwledger ...`
/// 2. Explicit env vars: `APPLE_NOTARY_KEY_ID`, `APPLE_NOTARY_ISSUER_ID`, `APPLE_NOTARY_KEY_PATH`
pub fn notarize(
    dmg_path: &Path,
    profile: Option<&str>,
    key_id: Option<&str>,
    issuer_id: Option<&str>,
) -> ReleaseResult<()> {
    info!(
        "notarizing: {} (profile={:?})",
        dmg_path.display(),
        profile
    );

    let profile_name = profile.unwrap_or("hwledger");

    // Check if keychain profile exists
    let use_profile = has_keychain_profile(profile_name)?;

    let mut cmd = ReleaseCommand::new("xcrun");
    cmd = cmd
        .arg("notarytool")
        .arg("submit")
        .arg(dmg_path.to_str().unwrap())
        .arg("--wait");

    if use_profile {
        debug!("using keychain profile: {}", profile_name);
        cmd = cmd.arg("--keychain-profile").arg(profile_name);
    } else {
        let key_id = key_id.ok_or_else(|| {
            ReleaseError::Credentials(
                "APPLE_NOTARY_KEY_ID not set and keychain profile not found".to_string(),
            )
        })?;
        let issuer_id = issuer_id.ok_or_else(|| {
            ReleaseError::Credentials(
                "APPLE_NOTARY_ISSUER_ID not set and keychain profile not found".to_string(),
            )
        })?;

        debug!("using explicit credentials (key_id={})", key_id);
        cmd = cmd
            .arg("--key-id")
            .arg(key_id)
            .arg("--issuer-id")
            .arg(issuer_id);
    }

    cmd.timeout(1200).run()?;

    // Staple the notarization ticket
    staple(dmg_path)?;

    info!("notarization complete and stapled: {}", dmg_path.display());
    Ok(())
}

/// Staple a notarization ticket to a DMG or .app.
fn staple(path: &Path) -> ReleaseResult<()> {
    debug!("stapling notarization ticket to: {}", path.display());

    ReleaseCommand::new("xcrun")
        .arg("stapler")
        .arg("staple")
        .arg(path.to_str().unwrap())
        .timeout(60)
        .run()?;

    info!("stapling complete: {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keychain_profile_probe() -> ReleaseResult<()> {
        // This test will fail on non-macOS or if the profile isn't set up.
        // Conditionally skip it in CI.
        if std::env::var("HWLEDGER_RELEASE_LIVE").is_ok() {
            let has_profile = has_keychain_profile("hwledger")?;
            println!("keychain profile found: {}", has_profile);
        }
        Ok(())
    }
}
