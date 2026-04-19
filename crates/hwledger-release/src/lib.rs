//! hwLedger release pipeline library.
//!
//! Provides Rust-based release tooling to replace shell scripts:
//! - XCFramework building (arm64 + x86_64, universal binaries)
//! - macOS app bundling and codesigning
//! - DMG creation and signing
//! - Apple notarization workflow
//! - Sparkle appcast generation with Ed25519 signing (replaces deprecated `generate_appcast`)
//! - ffmpeg keyframe extraction and manifest generation
//! - CLI orchestration (`vhs` tape recording with parallel execution)
//!
//! All subprocess execution goes through the unified `subprocess` module with
//! logging and timeout support.

pub mod error;
pub mod subprocess;
pub mod appcast;
pub mod xcframework;
pub mod bundle;
pub mod dmg;
pub mod notarize;
pub mod keyframes;
pub mod record;

pub use error::{ReleaseError, ReleaseResult};
