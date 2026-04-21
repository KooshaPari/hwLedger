//! `hwledger attest` — thin driver for the `hwledger-attest` crate.
//!
//! Delegates to the library. The standalone `hwledger-attest` binary exposes
//! the same surface for lefthook / git-wrapper use cases; this subcommand
//! just wires it into the top-level CLI so users only have one tool name to
//! remember.

use anyhow::{anyhow, Context, Result};
use clap::Subcommand;
use hwledger_attest::{
    append_to_log, build_and_sign, git_head_sha, git_tree_hash, last_attestation_hash, read_log,
    run_check, verify_attestation, verify_chain, Attestation, BuildInput, CheckResult,
};
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum AttestSubcommand {
    /// Build and sign a new attestation from local gates; append to chain.
    Build {
        #[arg(long, default_value = "default")]
        key_id: String,
        #[arg(long, hide = true)]
        synthetic: bool,
    },
    /// Verify a single attestation file or the HEAD of the chain.
    Verify { file: Option<PathBuf> },
    /// Walk the attestation chain.
    Chain,
    /// Generate a new ed25519 keypair.
    Genkey { id: String },
}

pub fn run(sub: AttestSubcommand) -> Result<()> {
    let repo = std::env::current_dir()?;
    match sub {
        AttestSubcommand::Build { key_id, synthetic } => build(&repo, &key_id, synthetic),
        AttestSubcommand::Verify { file } => verify(&repo, file),
        AttestSubcommand::Chain => chain(&repo),
        AttestSubcommand::Genkey { id } => {
            hwledger_attest::generate_keypair(&id)?;
            println!("generated key '{id}'");
            Ok(())
        }
    }
}

fn build(repo: &Path, key_id: &str, synthetic: bool) -> Result<()> {
    let commit_sha = git_head_sha(repo).context("HEAD")?;
    let tree_hash = git_tree_hash(repo).context("tree")?;
    let parent = last_attestation_hash(repo)?;
    let checks: Vec<CheckResult> = if synthetic {
        vec![CheckResult {
            name: "synthetic".into(),
            passed: true,
            duration_ms: 0,
            evidence_sha256: hwledger_attest::sha256_hex(b"synthetic"),
        }]
    } else {
        vec![
            run_check("cargo-fmt-check", "cargo", &["fmt", "--all", "--", "--check"]),
            run_check(
                "cargo-clippy",
                "cargo",
                &["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
            ),
            run_check("cargo-test", "cargo", &["test", "--workspace", "--no-fail-fast"]),
        ]
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
        return Err(anyhow!("one or more gates failed"));
    }
    Ok(())
}

fn verify(repo: &Path, file: Option<PathBuf>) -> Result<()> {
    let att: Attestation = if let Some(p) = file {
        serde_json::from_str(&std::fs::read_to_string(p)?)?
    } else {
        read_log(repo)?.into_iter().last().ok_or_else(|| anyhow!("no attestations in log"))?
    };
    verify_attestation(&att)?;
    println!("OK  commit={}  hash={}", att.payload.commit_sha, &att.hash[..16]);
    Ok(())
}

fn chain(repo: &Path) -> Result<()> {
    let rep = verify_chain(repo)?;
    let entries = read_log(repo)?;
    for (i, a) in entries.iter().enumerate() {
        println!(
            "#{:03}  commit={}  checks={}  hash={}",
            i,
            &a.payload.commit_sha[..a.payload.commit_sha.len().min(12)],
            a.payload.checks.len(),
            &a.hash[..16]
        );
    }
    if rep.ok {
        println!("\nchain: OK  ({} entries)", rep.total);
        Ok(())
    } else {
        for e in &rep.errors {
            println!("  ! {e}");
        }
        Err(anyhow!("chain broken"))
    }
}
