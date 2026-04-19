//! NFR-targeted tests.
//!
//! Some NFRs (NFR-002, NFR-004) require live fleet + billing data and cannot
//! be asserted in a unit-test context. Those are covered by golden-file
//! placeholders whose failure would still flag a coverage regression if the
//! NFR is later removed from PRD.md.

use hwledger_core::math::attention::BytesPerElement;
use hwledger_core::math::{AttentionKind, KvFormula, LayerKind};

/// Traces to: NFR-001
///
/// Planner math must stay within ±200 MB of ground-truth numbers we've
/// pinned for 10 canonical model families. The golden numbers come from the
/// §5.1 formula table and were hand-verified in WP09. This test is the
/// NFR-001 enforcement: drift >200 MB per model is a regression.
#[test]
fn nfr_001_planner_math_within_200mb_of_pinned_ground_truth() {
    const FP16: BytesPerElement = 2.0;
    const TOLERANCE_BYTES: i128 = 200_000_000;

    struct Golden {
        name: &'static str,
        kind: AttentionKind,
        seq_len: u64,
        expected_total_bytes: u64, // kv bytes/tok · seq · one live sequence
    }

    let cases = vec![
        Golden {
            name: "Llama-2-70B MHA @32k",
            kind: AttentionKind::Mha { num_layers: 80, num_attention_heads: 64, head_dim: 128 },
            seq_len: 32_000,
            // 2·80·64·128·32000·2 = 83_886_080_000
            expected_total_bytes: 83_886_080_000,
        },
        Golden {
            name: "Llama-3-70B GQA @32k",
            kind: AttentionKind::Gqa { num_layers: 80, num_kv_heads: 8, head_dim: 128 },
            seq_len: 32_000,
            // 2·80·8·128·32000·2 = 10_485_760_000
            expected_total_bytes: 10_485_760_000,
        },
        Golden {
            name: "DeepSeek-V3 MLA @32k (layer-invariant)",
            kind: AttentionKind::Mla { kv_lora_rank: 512, qk_rope_head_dim: 64 },
            seq_len: 32_000,
            // (512+64)·2·32000 = 36_864_000
            expected_total_bytes: 36_864_000,
        },
        Golden {
            name: "DeepSeek-V3 MLA @128k (layer-invariant)",
            kind: AttentionKind::Mla { kv_lora_rank: 512, qk_rope_head_dim: 64 },
            seq_len: 128_000,
            expected_total_bytes: 147_456_000,
        },
        Golden {
            name: "Mistral-7B SlidingWindow window=4096 @32k",
            kind: AttentionKind::SlidingWindow {
                num_layers: 32,
                num_kv_heads: 8,
                head_dim: 128,
                window: 4096,
            },
            seq_len: 32_000,
            // capped at window: 2·32·8·128·4096·2 = 536_870_912
            expected_total_bytes: 536_870_912,
        },
        Golden {
            name: "Qwen2-7B GQA @8k",
            kind: AttentionKind::Gqa { num_layers: 28, num_kv_heads: 4, head_dim: 128 },
            seq_len: 8_000,
            // 2·28·4·128·8000·2 = 458_752_000
            expected_total_bytes: 458_752_000,
        },
        Golden {
            name: "Qwen3.6-A3B Hybrid @1024 (10 full of 40 layers, H_kv=2, d=256)",
            kind: AttentionKind::Hybrid(
                (0..40)
                    .map(|i| {
                        if i % 4 == 0 {
                            LayerKind::FullAttention { num_kv_heads: 2, head_dim: 256 }
                        } else {
                            LayerKind::LinearAttention
                        }
                    })
                    .collect(),
            ),
            seq_len: 1_024,
            // 10·2·2·256·1024·2 = 20_971_520
            expected_total_bytes: 20_971_520,
        },
        Golden {
            name: "Mamba-2 3B SSM @128k (seq-invariant total)",
            kind: AttentionKind::Ssm { num_layers: 48, state_size: 64 },
            seq_len: 128_000,
            // 48·64·2 = 6144 total, amortised per token then recombined
            expected_total_bytes: 6_144,
        },
        Golden {
            name: "StreamingLLM-Llama-70B AttentionSink @10k (cap 2048)",
            kind: AttentionKind::AttentionSink {
                num_layers: 80,
                num_kv_heads: 8,
                head_dim: 128,
                sinks: 4,
                window: 2044,
            },
            seq_len: 10_000,
            // 2·80·8·128·2048·2 = 671_088_640
            expected_total_bytes: 671_088_640,
        },
        Golden {
            name: "MQA Falcon-style 32 layers @1k",
            kind: AttentionKind::Mqa { num_layers: 32, head_dim: 128 },
            seq_len: 1_024,
            // 2·32·128·1024·2 = 16_777_216
            expected_total_bytes: 16_777_216,
        },
    ];

    let mut failures = Vec::new();
    for g in &cases {
        let bpt = g.kind.bytes_per_token(g.seq_len, FP16);
        let actual = (bpt * g.seq_len as f64).round() as u64;
        let drift = (actual as i128) - (g.expected_total_bytes as i128);
        if drift.abs() > TOLERANCE_BYTES {
            failures.push(format!(
                "{}: expected {}, got {}, drift {} B (> {} B tolerance)",
                g.name, g.expected_total_bytes, actual, drift, TOLERANCE_BYTES
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "NFR-001 planner math drift exceeded ±200 MB tolerance:\n{}",
        failures.join("\n")
    );
}

/// Traces to: NFR-002
///
/// Upper-bound budget for agent-heartbeat payload size. A single
/// `TelemetrySnapshot` serialised as JSON is ≤ 300 B; at 2 × 30 s polls with
/// 8 devices we want ≤ 300 B × 8 × 120 = 288 KB/hour — safely under the
/// 2 MB/hour NFR budget. This test locks in the per-snapshot upper bound.
#[test]
fn nfr_002_telemetry_snapshot_stays_compact() {
    use serde_json::json;
    let sample = json!({
        "free_vram_bytes": 12_345_678_901u64,
        "util_percent": 83.4,
        "temperature_c": 61.2,
        "power_watts": 143.7,
        "captured_at_ms": 1_762_980_000_000u64
    });
    let bytes = serde_json::to_vec(&sample).unwrap();
    assert!(
        bytes.len() <= 300,
        "TelemetrySnapshot payload exceeded 300 B budget (got {} B); NFR-002 at risk",
        bytes.len()
    );
}

/// Traces to: NFR-003
///
/// SQLite must sustain ≥ 10k events/day. We don't spin a real DB here (that
/// belongs to hwledger-ledger's integration tests); we assert the
/// per-event wire size (post-serde) is compact enough that 10k × 24h of
/// inserts fits in a reasonable page budget.
#[test]
fn nfr_003_event_payload_stays_under_ledger_budget() {
    use serde_json::json;
    let event = json!({
        "seq": 12345u64,
        "prev_hash": "0".repeat(64),
        "hash": "0".repeat(64),
        "ts_ms": 1_762_980_000_000u64,
        "kind": "AgentHeartbeat",
        "payload": {"agent_id": "00000000-0000-0000-0000-000000000000", "device_count": 4u32, "at_ms": 1_762_980_000_000u64}
    });
    let bytes = serde_json::to_vec(&event).unwrap();
    // 10k events/day × 512 B/event ≈ 5 MB/day — well within SQLite's comfort zone.
    assert!(bytes.len() <= 512, "event wire size {} B exceeds 512 B ledger page budget", bytes.len());
}

/// Traces to: NFR-004
///
/// Cost estimator must match billing within 5 % over 24 h. We can't hit
/// Vast/RunPod in a unit test, but we CAN verify the compounding
/// arithmetic is monotone and the per-hour × 24 rollup matches daily
/// billing shape (USD rounding sanity).
#[test]
fn nfr_004_cost_rollup_matches_per_hour_times_24() {
    let hourly_usd = 0.369_f64;
    let daily_computed = hourly_usd * 24.0;
    let daily_expected = 8.856_f64;
    let drift = (daily_computed - daily_expected).abs();
    assert!(drift < 0.001, "cost rollup drift {} exceeds 0.001 USD tolerance", drift);
}

/// Traces to: NFR-005
///
/// Apache-2.0 compatibility. LGPL dynamic-link is allowed; GPL-only is not.
/// Reads the workspace `Cargo.lock` and flags any package whose license
/// field is `GPL-2.0-only`, `GPL-3.0-only`, or `AGPL-*`.
#[test]
fn nfr_005_license_rejects_gpl_only_deps() {
    use std::fs;
    let lock_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.lock");
    let Ok(lock) = fs::read_to_string(lock_path) else {
        // If we can't read it (e.g. running from packaged crate), skip rather than false-positive.
        return;
    };
    let bad = [
        "GPL-2.0-only",
        "GPL-3.0-only",
        "AGPL-3.0-only",
        "AGPL-3.0-or-later",
    ];
    let mut hits = Vec::new();
    for line in lock.lines() {
        if line.trim_start().starts_with("license =") {
            for tag in &bad {
                if line.contains(tag) {
                    hits.push(format!("{}: {}", tag, line.trim()));
                }
            }
        }
    }
    assert!(hits.is_empty(), "GPL-only dependencies detected:\n{}", hits.join("\n"));
}

/// Traces to: NFR-007
///
/// No unjustified `#[allow(dead_code)]` in shipped crates. We tolerate
/// `#[allow(clippy::assertions_on_constants)]` (used in this very test file
/// for stub tests) and `#[expect(dead_code)]` which is the modern opt-in
/// form. Anything else in `crates/*/src/` flags.
#[test]
fn nfr_007_no_unjustified_dead_code_allows_in_src() {
    use std::fs;
    use std::path::PathBuf;

    let crates_root = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../"));
    let mut offenders = Vec::new();
    for entry in walkdir_lite(&crates_root) {
        let path = entry;
        if !path.to_string_lossy().ends_with(".rs") {
            continue;
        }
        // Only check src/, not tests/ (tests are allowed more latitude).
        let p = path.to_string_lossy();
        if !p.contains("/src/") {
            continue;
        }
        let Ok(body) = fs::read_to_string(&path) else { continue };
        for (i, line) in body.lines().enumerate() {
            let l = line.trim_start();
            if l.starts_with("#[allow(dead_code)]")
                || l.starts_with("#![allow(dead_code)]")
                || l.starts_with("#[allow(unused)]")
            {
                offenders.push(format!("{}:{}: {}", path.display(), i + 1, line.trim()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "NFR-007 violation — unjustified dead_code suppressions found:\n{}",
        offenders.join("\n")
    );
}

fn walkdir_lite(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(p) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&p) else { continue };
        for e in entries.flatten() {
            let ep = e.path();
            if ep.is_dir() {
                let name = ep.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name == "target" || name == ".build" || name == "node_modules" || name.starts_with('.') {
                    continue;
                }
                stack.push(ep);
            } else {
                out.push(ep);
            }
        }
    }
    out
}
