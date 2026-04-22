//! Per-OS capture backends. Each backend owns the full lifecycle:
//! permission/sandbox setup → capture → write MP4 → teardown.

pub mod scsk;
pub mod winrdp;
pub mod xvfb;
