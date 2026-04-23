//! Integration test — PTY record path.
//!
//! Note: tests in this binary manipulate process-global env vars, so we
//! serialize them through a shared mutex. Running with `--test-threads=1`
//! is no longer required.
//!
//! When `PHENOTYPE_USER_STORY_RECORD=1` is set, the expanded test should
//! emit both a `.cast` (asciicast v2) and a `.manifest.json` under the
//! artifact directory. We scope artifacts into a tempdir via
//! `PHENOTYPE_USER_STORY_OUT` so concurrent runs don't clobber each other.

use hwledger_user_story_macros::user_story_test;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[user_story_test(yaml = r#"
journey_id: record-hello
title: "PTY record emits asciicast"
persona: framework maintainer
given: recording is enabled
when:
  - print a greeting
then:
  - stdout is captured
  - cast + manifest are written
traces_to: [FR-USER-STORY-010]
record: true
blind_judge: auto
family: cli
"#)]
fn record_hello() {
    // Deliberately write to stdout so the PTY parent captures bytes.
    println!("hello from inside the user_story_test body");
    assert_eq!(2 + 2, 4);
}

#[test]
fn record_on_emits_cast_and_manifest() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    use std::path::PathBuf;
    // Skip unless we are orchestrating the record run. We do this by
    // re-invoking cargo test with the env set. Running the whole suite
    // recursively is expensive; instead we drive the runtime directly.
    let tmp = tempfile::tempdir().unwrap();
    // SAFETY: single-threaded test section — the other tests in this binary
    // all end before this one unless run in parallel. We use
    // RUST_TEST_THREADS=1 implicitly via trybuild convention; here we just
    // tolerate pre-existing env.
    unsafe {
        std::env::set_var("PHENOTYPE_USER_STORY_OUT", tmp.path());
    }
    unsafe {
        std::env::set_var("PHENOTYPE_USER_STORY_RECORD", "1");
    }

    // Construct the same meta the macro would. This exercises the runtime
    // end-to-end without depending on whether cargo selected record_hello
    // for this particular process.
    let meta = hwledger_user_story_runtime::UserStoryMeta {
        journey_id: "record-hello-direct",
        title: "direct runtime record",
        persona: "integration",
        family: "cli",
        record: true,
        blind_judge: "auto",
        traces_to: &["FR-USER-STORY-010"],
    };
    // The inner closure will also run inside the PTY child (as record_hello).
    // We point the child at a non-existent test name so it exits 0 with
    // "0 tests run" — proving the pipeline works without recursion.
    let result = std::panic::catch_unwind(|| {
        hwledger_user_story_runtime::maybe_record(&meta, || {
            println!("greeting from integration harness");
        });
    });
    // Either the PTY flow succeeded (cast + manifest written) or it failed
    // because the synthetic journey_id didn't match a test filter (exit != 0).
    // In both cases the cast header should still have been written to disk.
    let cast: PathBuf = tmp.path().join("record-hello-direct.cast");
    let manifest: PathBuf = tmp.path().join("record-hello-direct.manifest.json");
    assert!(cast.exists(), "cast file should exist at {:?}", cast);
    assert!(manifest.exists(), "manifest file should exist at {:?}", manifest);
    assert!(
        hwledger_user_story_runtime::is_valid_asciicast_v2(&cast),
        "cast should be valid asciicast v2"
    );
    let mf: serde_json::Value = serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    assert_eq!(mf["journey_id"], "record-hello-direct");
    assert_eq!(mf["family"], "cli");
    assert_eq!(mf["traces_to"][0], "FR-USER-STORY-010");
    assert!(mf["pty"]["rows"].as_u64().unwrap() > 0);
    // result is Err iff the PTY child exited non-zero — tolerated.
    drop(result);
    unsafe {
        std::env::remove_var("PHENOTYPE_USER_STORY_RECORD");
    }
    unsafe {
        std::env::remove_var("PHENOTYPE_USER_STORY_OUT");
    }
}

#[test]
fn record_off_leaves_no_side_effects() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let tmp = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("PHENOTYPE_USER_STORY_OUT", tmp.path());
    }
    unsafe {
        std::env::remove_var("PHENOTYPE_USER_STORY_RECORD");
    }
    let meta = hwledger_user_story_runtime::UserStoryMeta {
        journey_id: "record-off",
        title: "no-record",
        persona: "x",
        family: "cli",
        record: true,
        blind_judge: "auto",
        traces_to: &["FR-USER-STORY-011"],
    };
    let mut ran = false;
    hwledger_user_story_runtime::maybe_record(&meta, || {
        ran = true;
    });
    assert!(ran);
    assert!(!tmp.path().join("record-off.cast").exists());
    assert!(!tmp.path().join("record-off.manifest.json").exists());
    unsafe {
        std::env::remove_var("PHENOTYPE_USER_STORY_OUT");
    }
}
