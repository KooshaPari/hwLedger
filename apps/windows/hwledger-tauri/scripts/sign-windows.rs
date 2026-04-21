//! Azure Trusted Signing helper — invoked via Tauri's `bundle.windows.signCommand`.
//!
//! Tauri 2 does not natively support Azure Trusted Signing in
//! `tauri.conf.json`'s certificate block; the community workaround is to set
//! `"signCommand"` to a custom command that drives the `trusted-signing-cli`
//! binary. This file is the blueprint for a small Rust wrapper we'll ship in
//! `hwledger-release` once the signing credential is issued.
//!
//! Usage (from tauri.conf.json):
//!
//! ```jsonc
//! "windows": {
//!   "signCommand": "cargo run -p hwledger-release --bin sign-windows -- \"%1\""
//! }
//! ```
//!
//! Implementation sketch — uncomment and wire up once `trusted-signing-cli`
//! is installed on the Windows build machine.
//!
//! ```ignore
//! use std::{env, process::Command};
//!
//! fn main() -> std::io::Result<()> {
//!     let file = env::args().nth(1).expect("usage: sign-windows <file>");
//!     let endpoint = env::var("AZURE_TRUSTED_SIGNING_ENDPOINT")?;
//!     let account  = env::var("AZURE_TRUSTED_SIGNING_ACCOUNT")?;
//!     let profile  = env::var("AZURE_TRUSTED_SIGNING_PROFILE")?;
//!     let status = Command::new("trusted-signing-cli")
//!         .args(["sign",
//!             "-e", &endpoint,
//!             "-a", &account,
//!             "-c", &profile,
//!             "-d", "hwLedger",
//!             "-u", "https://hwledger.dev",
//!             &file])
//!         .status()?;
//!     if !status.success() {
//!         std::process::exit(status.code().unwrap_or(1));
//!     }
//!     Ok(())
//! }
//! ```
//!
//! Background: see `docs-site/research/windows-client-strategy-2026-04.md`
//! §4 ("Code signing on Windows") — Azure Trusted Signing is first-class from
//! Microsoft's side but Tauri's cert block only accepts SHA-1 thumbprints for
//! local certs, so any HSM-backed flow has to go through `signCommand`.
fn main() {
    eprintln!(
        "sign-windows is a stub. See the file header for the real wrapper \
         once the Azure Trusted Signing credential lands."
    );
    std::process::exit(2);
}
