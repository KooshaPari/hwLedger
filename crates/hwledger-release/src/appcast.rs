//! Sparkle appcast generation and signing via Ed25519.

use crate::error::{ReleaseError, ReleaseResult};
use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::Signer;
use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use tracing::{debug, info};

/// Load Ed25519 private key from base64-encoded file (raw 32-byte format).
fn load_private_key(key_path: &Path) -> ReleaseResult<SigningKey> {
    let key_b64 = fs::read_to_string(key_path).map_err(ReleaseError::Io)?.trim().to_string();

    let key_bytes = STANDARD
        .decode(&key_b64)
        .map_err(|e| ReleaseError::SignatureError(format!("failed to decode key: {}", e)))?;

    if key_bytes.len() != 32 {
        return Err(ReleaseError::SignatureError(format!(
            "ed25519 key must be 32 bytes, got {}",
            key_bytes.len()
        )));
    }

    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&key_bytes);
    Ok(SigningKey::from_bytes(&bytes))
}

/// Generate RFC 2822 date string.
fn rfc2822_date() -> String {
    use chrono::Utc;
    Utc::now().format("%a, %d %b %Y %H:%M:%S +0000").to_string()
}

/// Generate and sign an appcast.xml file.
pub fn generate_appcast(
    dmg_path: &Path,
    version: &str,
    key_path: &Path,
    out_path: &Path,
    download_base: Option<&str>,
) -> ReleaseResult<()> {
    info!("generating appcast for version: {}", version);

    let sk = load_private_key(key_path)?;
    let dmg_bytes =
        fs::read(dmg_path).map_err(|_| ReleaseError::FileNotFound(dmg_path.to_path_buf()))?;

    let signature = sk.sign(&dmg_bytes);
    let sig_b64 = STANDARD.encode(signature.to_bytes());

    let size = dmg_bytes.len();
    let mut hasher = Sha256::new();
    hasher.update(&dmg_bytes);
    let sha256 = format!("{:x}", hasher.finalize());

    let dmg_name = dmg_path.file_name().and_then(|n| n.to_str()).unwrap_or("hwLedger.dmg");

    let dl_base =
        download_base.unwrap_or("https://github.com/KooshaPari/hwLedger/releases/download");
    let download_url = format!("{}/v{}/{}", dl_base, version, dmg_name);
    let pub_date = rfc2822_date();

    let escaped_version = version
        .replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&apos;");

    let xml = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0"
     xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle"
     xmlns:dc="http://purl.org/dc/elements/1.1/">
    <channel>
        <title>hwLedger</title>
        <link>https://kooshapari.github.io/hwLedger/appcast.xml</link>
        <description>hwLedger release feed</description>
        <language>en</language>
        <item>
            <title>Version {}</title>
            <sparkle:version>{}</sparkle:version>
            <sparkle:shortVersionString>{}</sparkle:shortVersionString>
            <sparkle:minimumSystemVersion>14.0</sparkle:minimumSystemVersion>
            <pubDate>{}</pubDate>
            <description><![CDATA[See the GitHub Release notes for changes in v{}.]]></description>
            <enclosure url="{}"
                       sparkle:version="{}"
                       sparkle:edSignature="{}"
                       length="{}"
                       type="application/octet-stream" />
            <!-- sha256: {} -->
        </item>
    </channel>
</rss>"#,
        escaped_version,
        escaped_version,
        escaped_version,
        pub_date,
        escaped_version,
        download_url,
        escaped_version,
        sig_b64,
        size,
        sha256
    );

    fs::create_dir_all(out_path.parent().unwrap_or_else(|| Path::new(".")))?;
    fs::write(out_path, &xml)?;

    info!("appcast written: {}", out_path.display());
    debug!("  version:    {}", version);
    debug!("  dmg:        {}", dmg_name);
    debug!("  size:       {} bytes", size);
    debug!("  sha256:     {}", sha256);
    debug!("  signature:  {}...", &sig_b64[..std::cmp::min(32, sig_b64.len())]);

    Ok(())
}
