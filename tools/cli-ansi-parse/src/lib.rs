//! `hwledger-cli-ansi-parse` — Tier 0 structural-capture, CLI family.
//!
//! Ingests an [asciicast v2] JSONL recording and a list of keyframe timestamps
//! (seconds from start), then emits one `<frame-id>.structural.json` file per
//! timestamp, capturing the terminal's structural state at that moment:
//!
//! ```json
//! {
//!   "family": "cli",
//!   "rows": 24,
//!   "cols": 80,
//!   "terminal_title": "hwledger",
//!   "cursor": { "row": 7, "col": 12, "visible": true },
//!   "cells": [
//!     { "row": 0, "col": 0, "ch": "$", "fg": null, "bg": null, "bold": false, "italic": false, "underline": false, "inverse": false }
//!   ]
//! }
//! ```
//!
//! The pipeline is byte-exact: each asciicast event's payload is shoved through
//! a [`vte::Parser`] that drives a [`vt100::Parser`] screen. When a keyframe
//! timestamp is reached (the first event strictly past the timestamp flushes),
//! we snapshot the screen's visible cells.
//!
//! Landing is idempotent: the same `.cast` + timestamp list produces byte-equal
//! output (no nondeterministic timestamps in payload).
//!
//! [asciicast v2]: https://docs.asciinema.org/manual/asciicast/v2/
//!
//! Traces to: Tier 0 structural-capture (CLI family).

use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

/// Asciicast v2 header (first JSON line of a `.cast`).
#[derive(Debug, Clone, Deserialize)]
pub struct CastHeader {
    pub version: u32,
    pub width: u16,
    pub height: u16,
    #[serde(default)]
    pub title: Option<String>,
}

/// A single asciicast v2 event: `[time, "o"|"i"|"r", data]`.
#[derive(Debug, Clone)]
pub struct CastEvent {
    pub time: f64,
    pub kind: String,
    pub data: String,
}

fn parse_event_line(line: &str) -> Result<CastEvent> {
    let v: serde_json::Value = serde_json::from_str(line)
        .with_context(|| format!("asciicast event JSON: {line:?}"))?;
    let arr = v
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("asciicast event must be array, got: {line:?}"))?;
    if arr.len() != 3 {
        bail!("asciicast event must have 3 elements, got {}", arr.len());
    }
    let time = arr[0]
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("event[0] must be float seconds"))?;
    let kind = arr[1]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("event[1] must be str kind"))?
        .to_string();
    let data = arr[2]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("event[2] must be str data"))?
        .to_string();
    Ok(CastEvent { time, kind, data })
}

/// Parse an asciicast v2 `.cast` file: header + events in stream order.
pub fn load_cast(path: &Path) -> Result<(CastHeader, Vec<CastEvent>)> {
    let f = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut lines = BufReader::new(f).lines();
    let header_line = lines
        .next()
        .ok_or_else(|| anyhow::anyhow!("empty cast: {}", path.display()))??;
    let header: CastHeader =
        serde_json::from_str(&header_line).with_context(|| "asciicast header JSON")?;
    if header.version != 2 {
        bail!("unsupported asciicast version: {} (need v2)", header.version);
    }
    let mut events = Vec::new();
    for line in lines {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        events.push(parse_event_line(&line)?);
    }
    Ok((header, events))
}

/// One cell in the reconstructed terminal grid.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StructuralCell {
    pub row: u16,
    pub col: u16,
    pub ch: String,
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
}

/// Cursor location + visibility.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StructuralCursor {
    pub row: u16,
    pub col: u16,
    pub visible: bool,
}

/// One structural snapshot (what gets serialized to `<frame-id>.structural.json`).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StructuralSnapshot {
    pub family: &'static str,
    pub rows: u16,
    pub cols: u16,
    pub terminal_title: Option<String>,
    pub cursor: StructuralCursor,
    /// Only non-blank cells are emitted — keeps JSON small for mostly-empty
    /// terminals and makes diffs legible.
    pub cells: Vec<StructuralCell>,
}

fn fmt_color(c: vt100::Color) -> Option<String> {
    match c {
        vt100::Color::Default => None,
        vt100::Color::Idx(i) => Some(format!("idx:{i}")),
        vt100::Color::Rgb(r, g, b) => Some(format!("#{r:02x}{g:02x}{b:02x}")),
    }
}

/// Snapshot the current visible screen of the given `vt100::Parser` into a
/// [`StructuralSnapshot`]. Only non-blank cells (non-space char or any style)
/// are included in `cells`.
pub fn snapshot_screen(
    parser: &vt100::Parser,
    terminal_title: Option<String>,
) -> StructuralSnapshot {
    let screen = parser.screen();
    let (rows, cols) = screen.size();
    let (cur_row, cur_col) = screen.cursor_position();
    let visible = !screen.hide_cursor();

    let mut cells = Vec::new();
    for row in 0..rows {
        for col in 0..cols {
            if let Some(cell) = screen.cell(row, col) {
                let contents = cell.contents();
                let bold = cell.bold();
                let italic = cell.italic();
                let underline = cell.underline();
                let inverse = cell.inverse();
                let fg = fmt_color(cell.fgcolor());
                let bg = fmt_color(cell.bgcolor());
                let is_blank =
                    contents.chars().all(|c| c == ' ' || c == '\0') && contents.len() <= 1;
                let has_style = bold || italic || underline || inverse || fg.is_some() || bg.is_some();
                if is_blank && !has_style {
                    continue;
                }
                cells.push(StructuralCell {
                    row,
                    col,
                    ch: contents.to_string(),
                    fg,
                    bg,
                    bold,
                    italic,
                    underline,
                    inverse,
                });
            }
        }
    }

    StructuralSnapshot {
        family: "cli",
        rows,
        cols,
        terminal_title,
        cursor: StructuralCursor {
            row: cur_row,
            col: cur_col,
            visible,
        },
        cells,
    }
}

/// Replay events and capture a snapshot at each timestamp in `timestamps`
/// (seconds since cast start, ascending). A timestamp triggers capture as
/// soon as all events with `event.time <= timestamp` have been applied.
///
/// Returns one snapshot per timestamp, in the same order.
pub fn replay_and_snapshot(
    header: &CastHeader,
    events: &[CastEvent],
    timestamps: &[f64],
) -> Vec<StructuralSnapshot> {
    let mut parser = vt100::Parser::new(header.height, header.width, 0);
    let mut ts_iter = timestamps.iter().copied().enumerate().peekable();
    let mut out: Vec<Option<StructuralSnapshot>> = vec![None; timestamps.len()];
    let title = header.title.clone();

    for ev in events {
        // Before applying this event, drain any timestamps that are strictly
        // less than the event's time — those are "past" relative to the
        // screen state we're about to mutate, so snapshot *before* the
        // mutation. This keeps behavior intuitive: a keyframe at t=1.5s
        // reflects all events with time <= 1.5s.
        while let Some(&(idx, t)) = ts_iter.peek() {
            if t < ev.time {
                out[idx] = Some(snapshot_screen(&parser, title.clone()));
                ts_iter.next();
            } else {
                break;
            }
        }

        if ev.kind == "o" {
            parser.process(ev.data.as_bytes());
        }
        // "i" (input) and "r" (resize) events: input doesn't affect screen
        // reconstruction (it's the user's keystrokes, not output). Resize
        // would, but asciinema rarely emits it; skip for now (TODO).
    }

    // After all events: any remaining timestamps snapshot the final screen.
    for (idx, _t) in ts_iter {
        out[idx] = Some(snapshot_screen(&parser, title.clone()));
    }

    out.into_iter()
        .map(|s| s.expect("every timestamp produces a snapshot"))
        .collect()
}

/// Emit one `<frame-id>.structural.json` file per timestamp.
///
/// `frame_ids` is aligned with `timestamps` (same length, same order).
/// Each snapshot is written to `out_dir.join(format!("{frame_id}.structural.json"))`.
pub fn emit_snapshots(
    out_dir: &Path,
    frame_ids: &[String],
    snapshots: &[StructuralSnapshot],
) -> Result<Vec<PathBuf>> {
    if frame_ids.len() != snapshots.len() {
        bail!(
            "frame_ids ({}) and snapshots ({}) length mismatch",
            frame_ids.len(),
            snapshots.len()
        );
    }
    std::fs::create_dir_all(out_dir)
        .with_context(|| format!("create out dir {}", out_dir.display()))?;
    let mut paths = Vec::with_capacity(snapshots.len());
    for (frame_id, snap) in frame_ids.iter().zip(snapshots) {
        let path = out_dir.join(format!("{frame_id}.structural.json"));
        let json = serde_json::to_string_pretty(snap)?;
        std::fs::write(&path, json).with_context(|| format!("write {}", path.display()))?;
        paths.push(path);
    }
    Ok(paths)
}

// ---------------------------------------------------------------------------
// D4 — cursor-track.jsonl synthesis from asciicast.
//
// Mirror of the schema emitted by the Playwright harness (D3) and the
// XCUITest CGEventTap harness (D2):
//
//     {"ts_ms": u64, "x": f64, "y": f64, "action": "move"|"down"|"up"|...}
//
// Asciicasts carry no click events, so every emitted event is "move".
// Cursor position is sampled on a fixed-rate virtual clock (default 30 Hz)
// while the asciicast stream is replayed through vt100::Parser; cell
// coordinates are converted to pixels via CellMetrics, and consecutive
// samples with identical (x, y) (rounded to 1 decimal) are deduplicated.
// ---------------------------------------------------------------------------

/// One cursor-track event — mirrors Playwright (D3) / XCUITest (D2) schema.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CursorTrackEvent {
    pub ts_ms: u64,
    /// Pixel x, center of the cell.
    pub x: f64,
    /// Pixel y, center of the cell.
    pub y: f64,
    /// Always `"move"` for asciicast-synthesized tracks (no click stream).
    pub action: String,
}

/// Terminal cell → pixel conversion metrics.
///
/// `rows` / `cols` may be passed as `0` and will be auto-populated from the
/// asciicast header's `height` / `width`.
#[derive(Debug, Clone, Copy)]
pub struct CellMetrics {
    pub cell_w: f64,
    pub cell_h: f64,
    pub rows: u16,
    pub cols: u16,
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

/// Replay an asciicast v2 `.cast` through `vt100::Parser`, sampling cursor
/// position at `sample_hz` into pixel coordinates via `metrics`.
///
/// Returns chronologically-ordered, deduplicated `"move"` events.
///
/// If `metrics.rows` or `metrics.cols` is 0, they're auto-populated from the
/// asciicast header. `sample_hz` of 0 is treated as 30.
pub fn synthesize_cursor_track(
    cast_path: &Path,
    metrics: &CellMetrics,
    sample_hz: u32,
) -> Result<Vec<CursorTrackEvent>> {
    let (header, events) = load_cast(cast_path)?;
    let mut m = *metrics;
    if m.rows == 0 {
        m.rows = header.height;
    }
    if m.cols == 0 {
        m.cols = header.width;
    }
    let hz = if sample_hz == 0 { 30 } else { sample_hz };
    let step_ms: f64 = 1000.0 / (hz as f64);

    let mut parser = vt100::Parser::new(m.rows, m.cols, 0);
    let mut out: Vec<CursorTrackEvent> = Vec::new();
    let mut last: Option<(f64, f64)> = None;
    // Virtual clock (ms since cast start). We emit a sample whenever the
    // next-sample-due timestamp (`next_emit_ms`) is strictly <= the current
    // event's time. After emitting, we bump `next_emit_ms` by `step_ms`.
    let mut next_emit_ms: f64 = 0.0;

    let push_sample = |out: &mut Vec<CursorTrackEvent>,
                           last: &mut Option<(f64, f64)>,
                           parser: &vt100::Parser,
                           ts_ms: u64| {
        let (row, col) = parser.screen().cursor_position();
        let x = round1((col as f64) * m.cell_w + m.cell_w / 2.0);
        let y = round1((row as f64) * m.cell_h + m.cell_h / 2.0);
        if let Some((px, py)) = *last {
            if (px - x).abs() < f64::EPSILON && (py - y).abs() < f64::EPSILON {
                return;
            }
        }
        *last = Some((x, y));
        out.push(CursorTrackEvent {
            ts_ms,
            x,
            y,
            action: "move".to_string(),
        });
    };

    for ev in &events {
        let ev_ms = ev.time * 1000.0;
        // Emit all pending samples whose scheduled time is <= this event's
        // wall-clock. This reflects the cursor state *before* the event is
        // applied, which is consistent with a fixed-rate display sampler.
        while next_emit_ms <= ev_ms {
            let ts_ms = next_emit_ms.max(0.0).round() as u64;
            push_sample(&mut out, &mut last, &parser, ts_ms);
            next_emit_ms += step_ms;
        }
        if ev.kind == "o" {
            parser.process(ev.data.as_bytes());
        }
    }

    // After the last event: emit one final sample reflecting the terminal
    // state. Use the last event time (or 0) as the stamp.
    if let Some(final_ev) = events.last() {
        let ts_ms = (final_ev.time * 1000.0).round() as u64;
        push_sample(&mut out, &mut last, &parser, ts_ms);
    } else if out.is_empty() {
        push_sample(&mut out, &mut last, &parser, 0);
    }

    Ok(out)
}

/// Write events as newline-delimited JSON to `out_path`. Parent dir is
/// created if needed.
pub fn write_cursor_track(events: &[CursorTrackEvent], out_path: &Path) -> Result<()> {
    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create parent {}", parent.display()))?;
        }
    }
    let mut f = File::create(out_path)
        .with_context(|| format!("create {}", out_path.display()))?;
    for ev in events {
        let line = serde_json::to_string(ev)?;
        f.write_all(line.as_bytes())?;
        f.write_all(b"\n")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(parser: &mut vt100::Parser, bytes: &[u8]) {
        parser.process(bytes);
    }

    /// "hello world" plain output lands at row 0, cols 0..11.
    #[test]
    fn test_hello_world_cells() {
        let mut p = vt100::Parser::new(5, 20, 0);
        apply(&mut p, b"hello world");
        let snap = snapshot_screen(&p, None);
        assert_eq!(snap.rows, 5);
        assert_eq!(snap.cols, 20);
        assert_eq!(snap.family, "cli");
        let text: String = snap
            .cells
            .iter()
            .filter(|c| c.row == 0)
            .map(|c| c.ch.as_str())
            .collect();
        assert_eq!(text, "helloworld"); // spaces are blank-skipped
        // Cursor advanced to col 11.
        assert_eq!(snap.cursor.col, 11);
        assert_eq!(snap.cursor.row, 0);
    }

    /// 24-bit color sequence is captured as `#rrggbb`.
    #[test]
    fn test_true_color_captured() {
        let mut p = vt100::Parser::new(3, 10, 0);
        // ESC[38;2;255;100;50m X ESC[0m
        apply(&mut p, b"\x1b[38;2;255;100;50mX\x1b[0m");
        let snap = snapshot_screen(&p, None);
        let x = snap
            .cells
            .iter()
            .find(|c| c.ch == "X")
            .expect("X cell present");
        assert_eq!(x.fg.as_deref(), Some("#ff6432"));
    }

    /// Cursor-position CSI: ESC[3;5H moves to row 2, col 4 (1-indexed → 0-indexed).
    #[test]
    fn test_cursor_move_and_bold() {
        let mut p = vt100::Parser::new(5, 20, 0);
        apply(&mut p, b"\x1b[3;5H\x1b[1mBOLD\x1b[0m");
        let snap = snapshot_screen(&p, Some("hwledger".into()));
        assert_eq!(snap.terminal_title.as_deref(), Some("hwledger"));
        // Cursor now at row 2, col 8 (moved to 2,4 then wrote 4 bold chars).
        assert_eq!(snap.cursor.row, 2);
        assert_eq!(snap.cursor.col, 8);
        let bold_b = snap
            .cells
            .iter()
            .find(|c| c.row == 2 && c.col == 4)
            .expect("B cell");
        assert_eq!(bold_b.ch, "B");
        assert!(bold_b.bold, "B must carry bold attribute");
    }

    /// Asciicast event line parses into time / kind / data.
    #[test]
    fn test_parse_event_line() {
        let ev = parse_event_line(r#"[1.25, "o", "hello"]"#).unwrap();
        assert_eq!(ev.time, 1.25);
        assert_eq!(ev.kind, "o");
        assert_eq!(ev.data, "hello");
    }

    /// End-to-end replay: two output events, one keyframe between them.
    #[test]
    fn test_replay_midpoint_snapshot() {
        let header = CastHeader {
            version: 2,
            width: 10,
            height: 3,
            title: None,
        };
        let events = vec![
            CastEvent {
                time: 0.1,
                kind: "o".into(),
                data: "hi".into(),
            },
            CastEvent {
                time: 2.0,
                kind: "o".into(),
                data: "!".into(),
            },
        ];
        // Snapshot at t=1.0 — should see "hi" but NOT "!".
        let snaps = replay_and_snapshot(&header, &events, &[1.0, 3.0]);
        assert_eq!(snaps.len(), 2);
        let first_text: String = snaps[0].cells.iter().map(|c| c.ch.as_str()).collect();
        let second_text: String = snaps[1].cells.iter().map(|c| c.ch.as_str()).collect();
        assert_eq!(first_text, "hi");
        assert_eq!(second_text, "hi!");
    }

    // ---- D4 cursor-track synthesis tests ----

    fn write_cast(dir: &Path, name: &str, header: &str, lines: &[&str]) -> PathBuf {
        let p = dir.join(name);
        let mut s = String::new();
        s.push_str(header);
        s.push('\n');
        for l in lines {
            s.push_str(l);
            s.push('\n');
        }
        std::fs::write(&p, s).unwrap();
        p
    }

    fn tmp_dir() -> PathBuf {
        let d = std::env::temp_dir()
            .join(format!("hwledger-cursor-track-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&d);
        // Per-test subdir to avoid collision when tests run in parallel.
        let sub = d.join(format!(
            "t-{:?}-{}",
            std::thread::current().id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&sub).unwrap();
        sub
    }

    #[test]
    fn test_d4_fixture_produces_events() {
        let dir = tmp_dir();
        let cast = write_cast(
            &dir,
            "hello.cast",
            r#"{"version":2,"width":40,"height":10}"#,
            &[
                r#"[0.0, "o", "hello"]"#,
                r#"[0.5, "o", " world"]"#,
                r#"[1.0, "o", "!"]"#,
            ],
        );
        let m = CellMetrics {
            cell_w: 8.0,
            cell_h: 16.0,
            rows: 0,
            cols: 0,
        };
        let events = synthesize_cursor_track(&cast, &m, 30).unwrap();
        assert!(!events.is_empty(), "expected at least one event");
        for e in &events {
            assert_eq!(e.action, "move");
        }
    }

    #[test]
    fn test_d4_chronological_order() {
        let dir = tmp_dir();
        let cast = write_cast(
            &dir,
            "chrono.cast",
            r#"{"version":2,"width":20,"height":5}"#,
            &[
                r#"[0.0, "o", "a"]"#,
                r#"[0.2, "o", "b"]"#,
                r#"[0.4, "o", "c"]"#,
                r#"[0.6, "o", "d"]"#,
            ],
        );
        let m = CellMetrics {
            cell_w: 8.0,
            cell_h: 16.0,
            rows: 0,
            cols: 0,
        };
        let events = synthesize_cursor_track(&cast, &m, 30).unwrap();
        for w in events.windows(2) {
            assert!(w[0].ts_ms <= w[1].ts_ms, "ts must be non-decreasing");
        }
    }

    #[test]
    fn test_d4_dedupe_same_cell() {
        let dir = tmp_dir();
        // 10 frames where the cursor never moves (empty "" outputs). Sampler
        // at 30 Hz would otherwise emit many samples; dedupe must collapse
        // them to a single emitted event.
        let mut lines = Vec::new();
        let mut stored = Vec::new();
        for i in 0..10 {
            stored.push(format!(r#"[{:.3}, "o", ""]"#, 0.05 * (i as f64)));
        }
        for s in &stored {
            lines.push(s.as_str());
        }
        let cast = write_cast(
            &dir,
            "static.cast",
            r#"{"version":2,"width":20,"height":5}"#,
            &lines,
        );
        let m = CellMetrics {
            cell_w: 8.0,
            cell_h: 16.0,
            rows: 0,
            cols: 0,
        };
        let events = synthesize_cursor_track(&cast, &m, 30).unwrap();
        assert_eq!(
            events.len(),
            1,
            "cursor never moved → dedupe → single event, got {events:?}"
        );
        // Cursor at (0,0) → x = 0*8 + 4 = 4.0, y = 0*16 + 8 = 8.0.
        assert!((events[0].x - 4.0).abs() < 1e-9);
        assert!((events[0].y - 8.0).abs() < 1e-9);
    }

    #[test]
    fn test_d4_jsonl_round_trip() {
        let dir = tmp_dir();
        let cast = write_cast(
            &dir,
            "rt.cast",
            r#"{"version":2,"width":20,"height":5}"#,
            &[
                r#"[0.0, "o", "ab"]"#,
                r#"[0.3, "o", "cd"]"#,
            ],
        );
        let m = CellMetrics {
            cell_w: 10.0,
            cell_h: 20.0,
            rows: 0,
            cols: 0,
        };
        let events = synthesize_cursor_track(&cast, &m, 30).unwrap();
        let out = dir.join("cursor-track.jsonl");
        write_cursor_track(&events, &out).unwrap();
        let txt = std::fs::read_to_string(&out).unwrap();
        let mut parsed: Vec<CursorTrackEvent> = Vec::new();
        for line in txt.lines() {
            if line.trim().is_empty() {
                continue;
            }
            parsed.push(serde_json::from_str(line).unwrap());
        }
        assert_eq!(parsed, events, "JSONL round-trip must be lossless");
    }

    #[test]
    fn test_d4_sample_hz_scales_rate() {
        let dir = tmp_dir();
        // Cast where the cursor moves across many columns, producing distinct
        // sample positions. Higher sample_hz should yield >= events than
        // lower sample_hz (can be equal if dedupe collapses both to same
        // unique-position count, but never fewer at the higher rate).
        let lines: Vec<String> = (0..20)
            .map(|i| format!(r#"[{:.3}, "o", "x"]"#, 0.05 * (i as f64)))
            .collect();
        let line_refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let cast = write_cast(
            &dir,
            "scale.cast",
            r#"{"version":2,"width":40,"height":5}"#,
            &line_refs,
        );
        let m = CellMetrics {
            cell_w: 8.0,
            cell_h: 16.0,
            rows: 0,
            cols: 0,
        };
        let ev_30 = synthesize_cursor_track(&cast, &m, 30).unwrap();
        let ev_10 = synthesize_cursor_track(&cast, &m, 10).unwrap();
        assert!(
            ev_30.len() >= ev_10.len(),
            "30Hz ({}) must emit >= 10Hz ({}) events",
            ev_30.len(),
            ev_10.len()
        );
    }

    #[test]
    fn test_d4_header_malformed_errs() {
        let dir = tmp_dir();
        let cast = write_cast(
            &dir,
            "bad.cast",
            r#"{"version":"not-a-number"}"#,
            &[r#"[0.0, "o", "x"]"#],
        );
        let m = CellMetrics {
            cell_w: 8.0,
            cell_h: 16.0,
            rows: 24,
            cols: 80,
        };
        let res = synthesize_cursor_track(&cast, &m, 30);
        assert!(res.is_err(), "malformed header must return Err");
    }

    #[test]
    fn test_d4_header_populates_dimensions() {
        let dir = tmp_dir();
        let cast = write_cast(
            &dir,
            "dims.cast",
            r#"{"version":2,"width":77,"height":19}"#,
            &[r#"[0.0, "o", "abc"]"#],
        );
        let m = CellMetrics {
            cell_w: 8.0,
            cell_h: 16.0,
            rows: 0,
            cols: 0,
        };
        // If zero dims weren't patched from header, vt100 would choke or
        // cursor would land at (0,0) but with wrong wraparound. Easiest
        // check: events produced, first event within bounds.
        let events = synthesize_cursor_track(&cast, &m, 30).unwrap();
        assert!(!events.is_empty());
        // After writing "abc", cursor col = 3 → x = 3*8 + 4 = 28.0.
        // The final (post-last-event) sample should reflect that.
        let last = events.last().unwrap();
        assert!((last.x - 28.0).abs() < 1e-9, "got {last:?}");
    }
}
