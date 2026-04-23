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
use std::io::{BufRead, BufReader};
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
}
