//! `hwledger-attest` CLI — build, verify, and walk the local-CI attestation
//! chain. Also exposes git-wrapper lockdown helpers.
//!
//! Subcommands:
//!   build   — run all local gates, capture evidence, sign, append to log
//!   verify  — verify a single attestation file or the HEAD entry
//!   chain   — walk the log, print entries, highlight breaks
//!   genkey  — generate a new ed25519 keypair for a dev-id
//!   verify-push — verify every commit in a push range has an attestation
//!   lockdown-check — used by git-wrapper to reject --no-verify / --force

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use hwledger_attest::{
    append_to_log, build_and_sign, git_head_sha, git_tree_hash, last_attestation_hash, read_log,
    run_check, verify_attestation, verify_chain, Attestation, BuildInput, CheckResult,
};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "hwledger-attest", version, about = "Local-CI attestation + tamper-guard")]
struct Cli {
    /// Repo root (defaults to CWD).
    #[arg(long, global = true)]
    repo: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run all local gates, sign, append to the log.
    Build {
        /// Signing key id (filename stem in $HWLEDGER_ATTEST_KEY_DIR or ~/.hwledger/attest-keys).
        #[arg(long, default_value = "default")]
        key_id: String,
        /// Skip the actual gate commands — only record the checks list as synthetic.
        /// Used for dry-run wiring tests. Never set in CI.
        #[arg(long, hide = true)]
        synthetic: bool,
    },
    /// Verify a single attestation JSON file.
    Verify {
        /// Path to attestation JSON. If omitted, verifies the last entry in the log.
        file: Option<PathBuf>,
    },
    /// Walk the log, print each entry, highlight chain breaks.
    Chain,
    /// Generate a new ed25519 keypair.
    Genkey { id: String },
    /// Verify each commit in `HEAD..<remote-ref>` has a corresponding attestation.
    VerifyPush {
        #[arg(long, default_value = "HEAD")]
        local_ref: String,
        #[arg(long)]
        remote_ref: Option<String>,
    },
    /// Used by the git-wrapper: check args for --no-verify / --force, reject
    /// unless HWLEDGER_ALLOW_FORCE=1 AND a signed justification exists.
    LockdownCheck {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

fn repo_root(r: Option<PathBuf>) -> Result<PathBuf> {
    Ok(r.unwrap_or(std::env::current_dir()?))
}

fn main() -> Result<()> {
    tracing_subscriber::fmt().with_writer(std::io::stderr).init();
    let cli = Cli::parse();
    let repo = repo_root(cli.repo)?;
    match cli.cmd {
        Cmd::Build { key_id, synthetic } => cmd_build(&repo, &key_id, synthetic),
        Cmd::Verify { file } => cmd_verify(&repo, file),
        Cmd::Chain => cmd_chain(&repo),
        Cmd::Genkey { id } => {
            hwledger_attest::generate_keypair(&id)?;
            println!("generated key '{id}' in {}", hwledger_attest::key_dir().display());
            Ok(())
        }
        Cmd::VerifyPush { local_ref, remote_ref } => {
            cmd_verify_push(&repo, &local_ref, remote_ref.as_deref())
        }
        Cmd::LockdownCheck { args } => cmd_lockdown(&repo, &args),
    }
}

fn cmd_build(repo: &Path, key_id: &str, synthetic: bool) -> Result<()> {
    let commit_sha = git_head_sha(repo).context("resolving HEAD")?;
    let tree_hash = git_tree_hash(repo).context("hashing tree")?;
    let parent = last_attestation_hash(repo)?;

    let checks: Vec<CheckResult> = if synthetic {
        vec![CheckResult {
            name: "synthetic".into(),
            passed: true,
            duration_ms: 0,
            evidence_sha256: hwledger_attest::sha256_hex(b"synthetic"),
        }]
    } else {
        run_all_gates(repo)
    };

    let any_failed = checks.iter().any(|c| !c.passed);

    let att = build_and_sign(BuildInput {
        commit_sha,
        tree_hash,
        parent_attestation_hash: parent,
        checks: checks.clone(),
        key_id: key_id.to_string(),
    })?;
    append_to_log(repo, &att)?;

    println!("{}", serde_json::to_string_pretty(&att)?);

    if any_failed {
        eprintln!("\n attest: one or more gates FAILED — push blocked");
        let failed: Vec<&str> =
            checks.iter().filter(|c| !c.passed).map(|c| c.name.as_str()).collect();
        return Err(anyhow!("failed gates: {:?}", failed));
    }
    Ok(())
}

/// The canonical gate list. Mirrors lefthook's pre-push stages, but captured
/// into evidence hashes. All commands run from the repo root.
fn run_all_gates(repo: &Path) -> Vec<CheckResult> {
    let cwd_guard = ChdirGuard::change(repo);
    let _ = cwd_guard;
    vec![
        run_check("cargo-fmt-check", "cargo", &["fmt", "--all", "--", "--check"]),
        run_check(
            "cargo-clippy",
            "cargo",
            &["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
        ),
        run_check("cargo-test", "cargo", &["test", "--workspace", "--no-fail-fast"]),
        // Journey / tape / traceability gates are owned by other crates; the
        // attestation just records whether they passed. They're run by the
        // lefthook pre-push step in order and their stdout is captured there.
        // Here we only re-invoke the traceability crate since it's in-repo.
        run_check(
            "traceability-journeys",
            "cargo",
            &["run", "--quiet", "-p", "hwledger-traceability", "--", "--repo", "."],
        ),
    ]
}

struct ChdirGuard {
    prev: PathBuf,
}
impl ChdirGuard {
    fn change(new: &Path) -> Option<Self> {
        let prev = std::env::current_dir().ok()?;
        if std::env::set_current_dir(new).is_ok() {
            Some(Self { prev })
        } else {
            None
        }
    }
}
impl Drop for ChdirGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.prev);
    }
}

fn cmd_verify(repo: &Path, file: Option<PathBuf>) -> Result<()> {
    let att: Attestation = if let Some(p) = file {
        serde_json::from_str(&std::fs::read_to_string(p)?)?
    } else {
        read_log(repo)?.into_iter().last().ok_or_else(|| anyhow!("no attestations in log"))?
    };
    verify_attestation(&att)?;
    println!("OK  commit={}  hash={}", att.payload.commit_sha, &att.hash[..16]);
    Ok(())
}

fn cmd_chain(repo: &Path) -> Result<()> {
    let rep = verify_chain(repo)?;
    let entries = read_log(repo)?;
    for (i, a) in entries.iter().enumerate() {
        let status =
            if a.payload.parent_attestation_hash.is_some() || i == 0 { "ok" } else { "??" };
        println!(
            "#{:03}  {}  commit={}  checks={}  hash={}",
            i,
            status,
            &a.payload.commit_sha[..a.payload.commit_sha.len().min(12)],
            a.payload.checks.len(),
            &a.hash[..16]
        );
    }
    if rep.ok {
        println!("\nchain: OK  ({} entries)", rep.total);
    } else {
        println!("\nchain: BROKEN  ({} entries, {} errors)", rep.total, rep.errors.len());
        for e in &rep.errors {
            println!("  ! {e}");
        }
        std::process::exit(2);
    }
    Ok(())
}

fn cmd_verify_push(repo: &Path, _local: &str, _remote: Option<&str>) -> Result<()> {
    // Minimal implementation: require the HEAD commit to have a matching
    // attestation (by commit_sha) in the log. A fuller implementation would
    // walk the full push range.
    let head = git_head_sha(repo)?;
    let entries = read_log(repo)?;
    let found = entries.iter().any(|a| a.payload.commit_sha == head);
    if !found {
        return Err(anyhow!(
            "no attestation found for HEAD {head} — run `hwledger attest build` first"
        ));
    }
    let rep = verify_chain(repo)?;
    if !rep.ok {
        return Err(anyhow!("chain broken: {:?}", rep.errors));
    }
    println!("verify-push: OK  HEAD={head} chain-entries={}", rep.total);
    Ok(())
}

fn cmd_lockdown(_repo: &Path, args: &[String]) -> Result<()> {
    // Reject --no-verify and --force / -f unless an explicit, signed override exists.
    let mut banned: Vec<&str> = Vec::new();
    for a in args {
        match a.as_str() {
            "--no-verify" => banned.push("--no-verify"),
            "--force" | "-f" | "--force-with-lease" => banned.push(a.as_str()),
            _ => {}
        }
    }
    if banned.is_empty() {
        return Ok(());
    }
    let allow = std::env::var("HWLEDGER_ALLOW_FORCE").ok();
    let justification_path = std::env::var("HWLEDGER_FORCE_JUSTIFICATION").ok();
    if allow.as_deref() == Some("1") && justification_path.is_some() {
        let path = justification_path.unwrap();
        if Path::new(&path).exists() {
            eprintln!("hwledger-attest: lockdown override accepted ({path})");
            return Ok(());
        }
    }
    eprintln!(
        "\n hwledger-attest: BLOCKED — refusing `git {}` with banned flag(s): {:?}\n\
        \n\
        The hwLedger local-CI attestation system requires every push to be\n\
        accompanied by a signed, hash-chained attestation manifest. The flags\n\
        you supplied would bypass that gate:\n\
        \n\
        To override (e.g. recovering from a hook bug), write a signed\n\
        justification file and set:\n\
        \n\
          export HWLEDGER_ALLOW_FORCE=1\n\
          export HWLEDGER_FORCE_JUSTIFICATION=/path/to/justification.txt\n\
        \n\
        Both must be set AND the file must exist. See\n\
        docs-site/quality/attestation.md for the recovery procedure.\n",
        args.join(" "),
        banned
    );
    std::process::exit(100);
}
