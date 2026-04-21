//! hwLedger Tauri 2 host (Windows/Linux).
//!
//! Traces to: FR-UI-001, FR-PLAN-003, FR-TEL-002, FR-HF-001.
//!
//! The Tauri host is itself Rust, so we bypass the C FFI surface and call the
//! high-level `hwledger-*` crates directly. `hwledger-ffi` remains the canonical
//! boundary for SwiftUI / Qt / C# clients — see `crates/hwledger-ffi/src/lib.rs`.
//! Wrapping those high-level crates inside `#[tauri::command]` functions gives
//! us strongly typed JSON across the webview bridge (no raw pointers, no manual
//! `CString` juggling).

mod commands;
mod error;
mod types;

use tauri::Manager;

/// Entry point invoked by both `cargo run` (desktop main) and `cargo tauri dev`.
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,hwledger_tauri=debug".into()),
        )
        .try_init()
        .ok();

    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::plan,
            commands::plan_layer_contributions,
            commands::probe_detect,
            commands::probe_sample,
            commands::hf_search,
            commands::core_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running hwledger-tauri");
}
