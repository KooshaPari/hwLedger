//! `git-wrapped` — transparent wrapper for `git` that rejects `--no-verify`
//! and `--force` unless an explicit, signed override is present. Install via:
//!
//!   cargo install --path crates/hwledger-attest --bin git-wrapped
//!   alias git=git-wrapped   # in ~/.zshrc
//!
//! All other `git` invocations pass through untouched. Unix-only.

#[cfg(unix)]
fn main() {
    use std::os::unix::process::CommandExt;
    use std::path::Path;
    use std::process::{exit, Command};

    let args: Vec<String> = std::env::args().skip(1).collect();

    let banned: Vec<&str> = args
        .iter()
        .filter_map(|a| match a.as_str() {
            "--no-verify" | "--force" | "-f" | "--force-with-lease" => Some(a.as_str()),
            _ => None,
        })
        .collect();

    if !banned.is_empty() {
        let allow = std::env::var("HWLEDGER_ALLOW_FORCE").ok();
        let just = std::env::var("HWLEDGER_FORCE_JUSTIFICATION").ok();
        let ok = allow.as_deref() == Some("1")
            && just.as_deref().map(|p| Path::new(p).exists()).unwrap_or(false);
        if !ok {
            eprintln!(
                "\n git-wrapped: BLOCKED — refusing `git {}` with banned flag(s): {:?}\n\n\
                The hwLedger local-CI attestation system requires every push to carry\n\
                a signed, hash-chained attestation. These flags would bypass that gate.\n\n\
                To override (e.g. recovering from a hook bug):\n\
                  export HWLEDGER_ALLOW_FORCE=1\n\
                  export HWLEDGER_FORCE_JUSTIFICATION=/path/to/justification.txt\n\n\
                Both must be set AND the justification file must exist. See\n\
                docs-site/quality/attestation.md for the recovery procedure.\n",
                args.join(" "),
                banned
            );
            exit(100);
        }
        eprintln!("git-wrapped: lockdown override accepted");
    }

    let real = std::env::var("HWLEDGER_REAL_GIT").unwrap_or_else(|_| "/usr/bin/git".to_string());
    let err = Command::new(real).args(&args).exec();
    eprintln!("git-wrapped: failed to exec git: {err}");
    exit(127);
}

#[cfg(not(unix))]
fn main() {
    eprintln!("git-wrapped is unix-only");
    std::process::exit(1);
}
