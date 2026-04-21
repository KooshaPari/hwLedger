//! Probe subcommand: GPU device discovery and telemetry.
//!
//! Traces to: FR-TEL-002

use crate::output;
use anyhow::Result;
use clap::{Parser, Subcommand};
use comfy_table::Table;
use hwledger_probe::{GpuProbe, ProbeError};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Parser)]
pub struct ProbeArgs;

#[derive(Subcommand)]
pub enum ProbeSubcommand {
    /// List all detected GPU devices.
    List(ListArgs),

    /// Watch GPU telemetry with streaming updates.
    Watch(WatchArgs),
}

#[derive(Parser)]
pub struct ListArgs {
    /// Refresh interval in milliseconds (time to wait before next sample).
    #[arg(long, default_value = "100")]
    refresh_ms: u64,

    /// Output as JSON instead of table.
    #[arg(long)]
    json: bool,
}

#[derive(Parser)]
pub struct WatchArgs {
    /// Sample interval (default: 1s). Formats: "500ms", "1s", "2.5s".
    #[arg(long, default_value = "1s")]
    interval: String,

    /// Output as NDJSON (one sample per line) instead of table updates.
    #[arg(long)]
    json: bool,
}

/// A single metric: either a numeric value or a human-readable
/// "Not supported on <chip>" marker. Serialised to JSON as either `"45.2"`
/// or `{"unsupported": "Not supported on Apple M1 Pro"}` so downstream
/// consumers can distinguish an honest-zero from an unsupported chip.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Metric<T> {
    Value(T),
    Unsupported { unsupported: String },
}

impl<T> Metric<T> {
    fn from_result(res: Result<T, ProbeError>) -> Self {
        match res {
            Ok(v) => Metric::Value(v),
            Err(ProbeError::UnsupportedMetric { chip, metric, .. }) => Metric::Unsupported {
                unsupported: format!("Not supported on {} ({})", chip, metric),
            },
            Err(e) => Metric::Unsupported { unsupported: format!("error: {}", e) },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSnapshot {
    pub id: u32,
    pub backend: String,
    pub name: String,
    pub uuid: Option<String>,
    pub total_vram: String,
    pub free_vram: Metric<String>,
    pub utilization: Metric<f32>,
    pub temperature: Metric<f32>,
    pub power: Metric<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySample {
    pub schema: String,
    pub timestamp_ms: u64,
    pub devices: Vec<DeviceSnapshot>,
}

pub fn run(subcommand: ProbeSubcommand) -> Result<()> {
    match subcommand {
        ProbeSubcommand::List(args) => list(args),
        ProbeSubcommand::Watch(args) => tokio::runtime::Runtime::new()?.block_on(watch(args)),
    }
}

fn list(args: ListArgs) -> Result<()> {
    let probes = hwledger_probe::detect();

    if probes.is_empty() {
        eprintln!("No GPU devices detected");
        return Ok(());
    }

    // Collect snapshots from all probes
    let mut devices = Vec::new();
    for probe in probes {
        match probe.enumerate() {
            Ok(enumerated) => {
                for device in enumerated {
                    let snapshot = snapshot_device(probe.as_ref(), &device)?;
                    devices.push(snapshot);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to enumerate {}: {}", probe.backend_name(), e);
            }
        }
    }

    if args.json {
        let sample = TelemetrySample {
            schema: "hwledger.v1".to_string(),
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            devices,
        };
        println!("{}", serde_json::to_string_pretty(&sample)?);
    } else {
        print_device_table(&devices)?;
    }

    Ok(())
}

async fn watch(args: WatchArgs) -> Result<()> {
    let interval_duration = parse_duration(&args.interval)?;

    let probes = hwledger_probe::detect();
    if probes.is_empty() {
        eprintln!("No GPU devices detected");
        return Ok(());
    }

    let mut interval = tokio::time::interval(interval_duration);

    // Respond to Ctrl+C: install signal handler for graceful exit
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        let _ = tx.send(()).await;
    });

    loop {
        tokio::select! {
            _ = rx.recv() => {
                tracing::debug!("Received SIGINT, exiting watch");
                break;
            }
            _ = interval.tick() => {
                // Collect snapshots
                let mut devices = Vec::new();
                for probe in &probes {
                    match probe.enumerate() {
                        Ok(enumerated) => {
                            for device in enumerated {
                                if let Ok(snap) = snapshot_device(probe.as_ref(), &device) {
                                    devices.push(snap);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to enumerate: {}", e);
                        }
                    }
                }

                let sample = TelemetrySample {
                    schema: "hwledger.v1".to_string(),
                    timestamp_ms: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                    devices,
                };

                if args.json {
                    println!("{}", serde_json::to_string(&sample)?);
                } else {
                    print_device_table(&sample.devices)?;
                }
            }
        }
    }

    Ok(())
}

fn snapshot_device(
    probe: &dyn GpuProbe,
    device: &hwledger_probe::Device,
) -> Result<DeviceSnapshot> {
    let free_vram = Metric::from_result(probe.free_vram(device.id).map(output::format_bytes));
    let utilization = Metric::from_result(probe.utilization(device.id));
    let temperature = Metric::from_result(probe.temperature(device.id));
    let power = Metric::from_result(probe.power_draw(device.id));

    Ok(DeviceSnapshot {
        id: device.id,
        backend: device.backend.to_string(),
        name: device.name.clone(),
        uuid: device.uuid.clone(),
        total_vram: output::format_bytes(device.total_vram),
        free_vram,
        utilization,
        temperature,
        power,
    })
}

fn print_device_table(devices: &[DeviceSnapshot]) -> Result<()> {
    if devices.is_empty() {
        println!("No devices");
        return Ok(());
    }

    let mut table = Table::new();
    table.set_header(vec![
        "ID",
        "Backend",
        "Name",
        "Total VRAM",
        "Free VRAM",
        "Util %",
        "Temp C",
        "Power W",
    ]);

    fn render_str(m: &Metric<String>) -> String {
        match m {
            Metric::Value(s) => s.clone(),
            Metric::Unsupported { unsupported } => unsupported.clone(),
        }
    }
    fn render_pct(m: &Metric<f32>) -> String {
        match m {
            Metric::Value(v) => crate::output::format_percent(*v),
            Metric::Unsupported { unsupported } => unsupported.clone(),
        }
    }
    fn render_temp(m: &Metric<f32>) -> String {
        match m {
            Metric::Value(v) => crate::output::format_temp(*v),
            Metric::Unsupported { unsupported } => unsupported.clone(),
        }
    }
    fn render_power(m: &Metric<f32>) -> String {
        match m {
            Metric::Value(v) => crate::output::format_power(*v),
            Metric::Unsupported { unsupported } => unsupported.clone(),
        }
    }

    for dev in devices {
        table.add_row(vec![
            dev.id.to_string(),
            dev.backend.clone(),
            dev.name.clone(),
            dev.total_vram.clone(),
            render_str(&dev.free_vram),
            render_pct(&dev.utilization),
            render_temp(&dev.temperature),
            render_power(&dev.power),
        ]);
    }

    println!("{}", table);
    Ok(())
}

fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim();
    if let Ok(ms) = s.parse::<u64>() {
        return Ok(Duration::from_millis(ms));
    }

    if s.ends_with("ms") {
        let ms = s.trim_end_matches("ms").parse::<u64>()?;
        Ok(Duration::from_millis(ms))
    } else if s.ends_with('s') {
        let s_val: f64 = s.trim_end_matches('s').parse()?;
        Ok(Duration::from_secs_f64(s_val))
    } else {
        Err(anyhow::anyhow!("invalid duration: {}; use '1s', '500ms', '2.5s', etc.", s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Traces to: FR-TEL-002
    #[test]
    fn test_parse_duration_ms() {
        let d = parse_duration("500ms").unwrap();
        assert_eq!(d.as_millis(), 500);
    }

    // Traces to: FR-TEL-002
    #[test]
    fn test_parse_duration_s() {
        let d = parse_duration("1s").unwrap();
        assert_eq!(d.as_secs(), 1);
    }

    // Traces to: FR-TEL-002
    #[test]
    fn test_parse_duration_fractional() {
        let d = parse_duration("2.5s").unwrap();
        assert_eq!(d.as_secs_f64(), 2.5);
    }

    // Traces to: FR-TEL-002
    #[test]
    fn test_parse_duration_invalid() {
        assert!(parse_duration("invalid").is_err());
    }
}
