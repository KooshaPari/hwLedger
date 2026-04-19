//! Shared subprocess command runner with logging and timeout support.

use crate::error::{ReleaseError, ReleaseResult};
use std::process::Command;
use std::time::Duration;
use tracing::{debug, info};
use wait_timeout::ChildExt;

/// Execute a command with timeout and logging.
pub fn run(cmd: &str, args: &[&str], timeout_secs: u64) -> ReleaseResult<String> {
    info!("exec: {} {}", cmd, args.join(" "));

    let mut child = Command::new(cmd)
        .args(args)
        .spawn()
        .map_err(|e| ReleaseError::CommandFailed(format!("{}: {}", cmd, e)))?;

    let timeout = Duration::from_secs(timeout_secs);
    let status = child
        .wait_timeout(timeout)
        .map_err(|e| ReleaseError::CommandFailed(format!("wait_timeout: {}", e)))?;

    if let Some(status) = status {
        if status.success() {
            debug!("command succeeded: {}", cmd);
            Ok(String::new())
        } else {
            Err(ReleaseError::CommandFailed(format!(
                "{} exited with code {:?}",
                cmd,
                status.code()
            )))
        }
    } else {
        let _ = child.kill();
        Err(ReleaseError::CommandTimeout(timeout_secs, cmd.to_string()))
    }
}

/// A light wrapper for conditional command execution with dry-run support.
pub struct ReleaseCommand {
    cmd: String,
    args: Vec<String>,
    timeout_secs: u64,
    dry_run: bool,
}

impl ReleaseCommand {
    pub fn new(cmd: &str) -> Self {
        Self { cmd: cmd.to_string(), args: vec![], timeout_secs: 300, dry_run: false }
    }

    pub fn arg(mut self, arg: &str) -> Self {
        self.args.push(arg.to_string());
        self
    }

    pub fn args(mut self, args: &[&str]) -> Self {
        self.args.extend(args.iter().map(|s| s.to_string()));
        self
    }

    pub fn timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    pub fn dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    pub fn run(self) -> ReleaseResult<String> {
        if self.dry_run {
            info!("[DRY-RUN] {} {}", self.cmd, self.args.join(" "));
            return Ok(String::new());
        }

        let args_ref: Vec<&str> = self.args.iter().map(|s| s.as_str()).collect();
        run(&self.cmd, &args_ref, self.timeout_secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dry_run() {
        let result = ReleaseCommand::new("false").dry_run(true).run();
        assert!(result.is_ok());
    }
}
