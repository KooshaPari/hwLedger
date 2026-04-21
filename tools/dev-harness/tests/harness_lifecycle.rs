//! Integration test for `hwledger-dev-harness` PID-file lifecycle.
//!
//! Uses the `mock-spawn` feature (see lib.rs) so no real cargo/bun/uv processes
//! are needed. The harness should:
//!   1. Spawn a stand-in (`sleep 3600`) per requested service.
//!   2. Write a PID file with one ServiceRecord per service.
//!   3. `teardown` should SIGTERM each PID and delete the PID file.
//!
//! Run with: `cargo test -p hwledger-dev-harness --features mock-spawn`.

#![cfg(feature = "mock-spawn")]

use std::collections::HashMap;
use std::path::PathBuf;

use hwledger_dev_harness::{
    pid_file, spawn_service, teardown, HarnessState, Palette, UpConfig,
};

fn setup_home() -> PathBuf {
    let dir = tempfile::tempdir().expect("tempdir").keep();
    std::env::set_var("HWLEDGER_HOME", &dir);
    dir
}

#[test]
fn up_writes_pid_file_and_down_tears_down() {
    let _home = setup_home();
    let cfg = UpConfig {
        clients: vec!["cli".into(), "streamlit".into()],
        port_base: 8000,
        repo_root: std::env::current_dir().unwrap(),
        release: false,
    };

    let mut palette = Palette::default();
    let mut state = HarnessState::default();
    let env: HashMap<String, String> = HashMap::new();

    // Spawn two mock services.
    let a = spawn_service(
        &cfg,
        "svc-a",
        "unused",
        &[],
        &env,
        &cfg.repo_root,
        Some(8080),
        &mut palette,
    )
    .expect("spawn a");
    let b = spawn_service(
        &cfg,
        "svc-b",
        "unused",
        &[],
        &env,
        &cfg.repo_root,
        Some(8511),
        &mut palette,
    )
    .expect("spawn b");
    state.services.push(a.clone());
    state.services.push(b.clone());

    let pid_path = pid_file().expect("pid_file");
    state.save(&pid_path).expect("save");
    assert!(pid_path.exists(), "pid file should exist after save");

    let loaded = HarnessState::load(&pid_path).expect("load");
    assert_eq!(loaded.services.len(), 2);
    assert!(loaded.services.iter().any(|s| s.name == "svc-a"));
    assert!(loaded.services.iter().any(|s| s.name == "svc-b"));
    assert_eq!(loaded.services[0].port, Some(8080));

    // Tear down.
    let killed = teardown(&pid_path).expect("teardown");
    assert_eq!(killed.len(), 2, "both mock pids should receive SIGTERM");
    assert!(!pid_path.exists(), "pid file should be removed");
}
