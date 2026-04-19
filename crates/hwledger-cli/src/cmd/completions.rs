//! Completions subcommand: generate shell completions.

use anyhow::{anyhow, Result};
use clap::{CommandFactory, Parser};
use clap_complete::shells::*;

#[derive(Parser)]
pub struct CompletionsArgs {
    /// Shell to generate completions for: bash, zsh, fish, powershell.
    #[arg(value_parser = ["bash", "zsh", "fish", "powershell"])]
    shell: String,
}

pub fn run(args: CompletionsArgs) -> Result<()> {
    match args.shell.as_str() {
        "bash" => {
            use super::super::Cli;
            let mut cmd = Cli::command();
            clap_complete::generate(Bash, &mut cmd, "hwledger", &mut std::io::stdout());
            Ok(())
        }
        "zsh" => {
            use super::super::Cli;
            let mut cmd = Cli::command();
            clap_complete::generate(Zsh, &mut cmd, "hwledger", &mut std::io::stdout());
            Ok(())
        }
        "fish" => {
            use super::super::Cli;
            let mut cmd = Cli::command();
            clap_complete::generate(Fish, &mut cmd, "hwledger", &mut std::io::stdout());
            Ok(())
        }
        "powershell" => {
            use super::super::Cli;
            let mut cmd = Cli::command();
            clap_complete::generate(PowerShell, &mut cmd, "hwledger", &mut std::io::stdout());
            Ok(())
        }
        _ => Err(anyhow!("unsupported shell: {}", args.shell)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Traces to: FR-CLI-001
    #[test]
    fn test_valid_shell_args() {
        for shell in &["bash", "zsh", "fish", "powershell"] {
            let args = CompletionsArgs { shell: shell.to_string() };
            assert_eq!(args.shell, *shell);
        }
    }
}
