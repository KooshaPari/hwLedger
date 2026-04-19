//! CLI tool for GUI recording and keyframe extraction.
//!
//! Usage:
//!   hwledger-gui-recorder record <app-id> <output.mp4>
//!   hwledger-gui-recorder extract <recording.mp4> <journey-dir>
//!   hwledger-gui-recorder full <app-id> <journey-dir>

use hwledger_gui_recorder::{JourneyRecorder, ScreenRecorder};
use std::env;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

fn main() -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async_main())
}

async fn async_main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    match args[1].as_str() {
        "record" => {
            if args.len() < 4 {
                eprintln!("Usage: {} record <app-id> <output.mp4>", args[0]);
                return Ok(());
            }
            cmd_record(&args[2], &args[3]).await?;
        }
        "extract" => {
            if args.len() < 4 {
                eprintln!("Usage: {} extract <recording.mp4> <journey-dir>", args[0]);
                return Ok(());
            }
            cmd_extract(&args[2], &args[3]).await?;
        }
        "full" => {
            if args.len() < 4 {
                eprintln!("Usage: {} full <app-id> <journey-dir>", args[0]);
                return Ok(());
            }
            cmd_full(&args[2], &args[3]).await?;
        }
        _ => {
            print_usage();
        }
    }

    Ok(())
}

fn print_usage() {
    eprintln!(
        "hwledger-gui-recorder CLI

Commands:
  record <app-id> <output.mp4>       Start recording (requires user interaction to stop)
  extract <recording.mp4> <dir>      Extract keyframes + GIF from recording
  full <app-id> <journey-dir>        Record + extract in one step

Example:
  hwledger-gui-recorder record com.kooshapari.hwLedger /tmp/rec.mp4
  hwledger-gui-recorder extract /tmp/rec.mp4 /tmp/journey
"
    );
}

async fn cmd_record(app_id: &str, output_path: &str) -> anyhow::Result<()> {
    let recorder = ScreenRecorder::new(PathBuf::from(output_path));

    println!("Starting recording: {}", app_id);
    recorder.start_recording(app_id).await?;

    println!("Recording active. Press Ctrl+C to stop...");
    // Keep recording until user interrupts (Ctrl+C)
    loop {
        sleep(Duration::from_secs(1)).await;
    }
}

async fn cmd_extract(recording_path: &str, journey_dir: &str) -> anyhow::Result<()> {
    let recorder = JourneyRecorder::new(PathBuf::from(recording_path), PathBuf::from(journey_dir));

    println!("Extracting keyframes from: {}", recording_path);
    let manifest = recorder.extract_all().await?;

    println!("Journey manifest:");
    println!("  ID: {}", manifest.journey_id);
    println!("  Keyframes: {}", manifest.keyframes.len());
    println!("  Duration: {:.1}s", manifest.duration_secs);
    println!("  Output: {}", PathBuf::from(journey_dir).display());

    Ok(())
}

async fn cmd_full(app_id: &str, journey_dir: &str) -> anyhow::Result<()> {
    let recording_path = PathBuf::from(journey_dir).join("recording.mp4");

    println!("Starting full recording + extraction cycle");
    println!("App: {}", app_id);
    println!("Journey dir: {}", journey_dir);

    let recorder = ScreenRecorder::new(recording_path.clone());
    recorder.start_recording(app_id).await?;

    println!("Recording active. Press Ctrl+C when done...");
    loop {
        sleep(Duration::from_secs(1)).await;
    }
}
