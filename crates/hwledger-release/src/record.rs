//! VHS tape recording and replay orchestration.
//!
//! Records CLI journeys using `vhs` (terminal screen recorder) with parallel
//! execution and bounded concurrency.

use crate::error::ReleaseResult;
use crate::subprocess::ReleaseCommand;
use std::path::Path;
use tokio::sync::Semaphore;
use std::sync::Arc;
use tracing::info;

/// Record VHS tapes with bounded parallelism (default: 3 concurrent recordings).
///
/// Scans for `.tape` files in the base directory and spawns `vhs <tape>` for each.
pub async fn record_all_tapes(
    base_dir: &Path,
    concurrency: usize,
) -> ReleaseResult<()> {
    info!(
        "recording all tapes in: {} (concurrency={})",
        base_dir.display(),
        concurrency
    );

    let semaphore = Arc::new(Semaphore::new(concurrency));

    // Find all .tape files
    let mut tasks = vec![];

    for entry in walkdir::WalkDir::new(base_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "tape"))
    {
        let path = entry.path().to_path_buf();
        let sem = Arc::clone(&semaphore);

        let task = tokio::spawn(async move {
            let _permit = sem.acquire().await;
            record_tape(&path)
        });

        tasks.push(task);
    }

    for task in tasks {
        task.await.map_err(|e| {
            crate::error::ReleaseError::CommandFailed(format!("task join error: {}", e))
        })??;
    }

    info!("all tapes recorded");
    Ok(())
}

/// Record a single VHS tape.
pub fn record_tape(tape_path: &Path) -> ReleaseResult<()> {
    info!("recording tape: {}", tape_path.display());

    ReleaseCommand::new("vhs")
        .arg(tape_path.to_str().unwrap())
        .timeout(600)
        .run()?;

    info!("tape recorded: {}", tape_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_concurrency() -> ReleaseResult<()> {
        // Placeholder for concurrency test
        Ok(())
    }
}
