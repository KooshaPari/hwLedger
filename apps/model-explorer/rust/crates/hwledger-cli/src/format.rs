//! Output helpers: pretty tables for humans, JSON for scripts.

use std::io::Write;

use anyhow::Result;
use comfy_table::Table;
use serde::Serialize;

/// Render a `serde::Serialize` value as pretty JSON on stdout.
pub fn print_json<T: Serialize>(value: &T) -> Result<()> {
    let s = serde_json::to_string_pretty(value)?;
    println!("{s}");
    Ok(())
}

/// Write a pre-built `comfy_table::Table` to stdout.
pub fn print_table(table: &Table) {
    // comfy-table doesn't write to a writer by default in 7.x — print via Display.
    println!("{table}");
}

/// Suppress unused-warning on `Write` when feature-gating tests later.
#[allow(dead_code)]
fn _write_marker(_w: &mut dyn Write) {}