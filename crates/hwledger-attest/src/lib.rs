//! hwledger-attest — local-CI attestation + tamper-guard.
//!
//! Each pre-push run emits a signed, hash-chained JSON `Attestation` attesting
//! that local gates (clippy, test, fmt, tape-assertions, manifests, journeys)
//! passed. The attestation references the prior attestation by hash, forming
//! an append-only event-sourced chain at `.hwledger/attestations.log`. Any
//! reorder, rewrite, or synthesised entry breaks the chain and is detected
//! by `verify_chain()`.
//!
//! Trust model: ed25519 signatures; public keys live in
//! `~/.hwledger/attest-keys/<dev-id>.pub`. A signature by a key not in the
//! registry is rejected. Key rotation is handled by appending a new key to
//! the registry; old attestations remain verifiable against their key.
//!
//! This crate is domain-agnostic — the gate commands, evidence capture, and
//! git integration all live here. See `hwledger-cli`'s `attest` subcommand
//! for the user-facing driver.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

pub const SCHEMA_VERSION: u32 = 1;
pub const LOG_PATH: &str = ".hwledger/attestations.log";
pub const KEY_DIR_ENV: &str = "HWLEDGER_ATTEST_KEY_DIR";

#[derive(Debug, Error)]
pub enum AttestError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("hex: {0}")]
    Hex(#[from] hex::FromHexError),
    #[error("signature: {0}")]
    Sig(String),
    #[error("chain broken at line {line}: {reason}")]
    Chain { line: usize, reason: String },
    #[error("tamper: {0}")]
    Tamper(String),
    #[error("no key found: {0}")]
    NoKey(String),
    #[error("check failed: {name} ({detail})")]
    CheckFailed { name: String, detail: String },
    #[error("git: {0}")]
    Git(String),
}

pub type Result<T> = std::result::Result<T, AttestError>;

/// Signature block. Detached from the payload so we can canonicalise the
/// payload, hash it, sign the hash, then stamp sig+hash into the envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Signature {
    /// Key id — filename stem in the attest-keys dir (e.g. "koosha-macbook").
    pub key_id: String,
    /// Hex-encoded ed25519 signature over the canonical payload bytes.
    pub sig_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostInfo {
    pub os: String,
    pub hostname: String,
    pub user: String,
    /// RFC3339 timestamp.
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub duration_ms: u64,
    /// sha256 of the captured stdout+stderr from the check.
    pub evidence_sha256: String,
}

/// The inner payload (without signature/hash) — what we canonicalise and sign.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttestationPayload {
    pub version: u32,
    pub commit_sha: String,
    pub tree_hash: String,
    pub parent_attestation_hash: Option<String>,
    pub checks: Vec<CheckResult>,
    pub host: HostInfo,
}

/// Full signed attestation. This is the JSON-lines entry written to the log.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Attestation {
    #[serde(flatten)]
    pub payload: AttestationPayload,
    pub signature: Signature,
    /// sha256(canonical_payload || "|" || sig_hex). Used as the next entry's
    /// parent_attestation_hash.
    pub hash: String,
}

impl AttestationPayload {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        // serde_json with sorted keys by constructing a BTreeMap-like traversal.
        // Simplest correct path: round-trip through Value and serialise with a
        // deterministic ordering.
        let v: serde_json::Value = serde_json::to_value(self)?;
        Ok(canonical_json(&v).into_bytes())
    }
}

/// Deterministic JSON serialisation: sort object keys lexicographically, no
/// whitespace. Good enough for signing — RFC8785 would be stricter but this
/// is stable across platforms for our shapes (no floats with precision
/// ambiguity).
pub fn canonical_json(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let parts: Vec<String> = keys
                .iter()
                .map(|k| {
                    format!("{}:{}", serde_json::to_string(k).unwrap(), canonical_json(&map[*k]))
                })
                .collect();
            format!("{{{}}}", parts.join(","))
        }
        serde_json::Value::Array(arr) => {
            let parts: Vec<String> = arr.iter().map(canonical_json).collect();
            format!("[{}]", parts.join(","))
        }
        other => serde_json::to_string(other).unwrap(),
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

// ---------- git helpers ----------

pub fn git_head_sha(repo: &Path) -> Result<String> {
    run_git(repo, &["rev-parse", "HEAD"])
}

pub fn git_tree_hash(repo: &Path) -> Result<String> {
    // ls-tree -r HEAD gives file-level mode/hash/path. Hash the canonicalised
    // listing → repeatable tree digest independent of working-dir mtimes.
    let ls = run_git(repo, &["ls-tree", "-r", "HEAD"])?;
    Ok(sha256_hex(ls.as_bytes()))
}

fn run_git(repo: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|e| AttestError::Git(format!("spawn git: {e}")))?;
    if !out.status.success() {
        return Err(AttestError::Git(format!(
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

// ---------- host info ----------

pub fn host_info() -> HostInfo {
    HostInfo {
        os: std::env::consts::OS.to_string(),
        hostname: hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "unknown".into()),
        user: std::env::var("USER").unwrap_or_else(|_| "unknown".into()),
        timestamp: chrono::Utc::now().to_rfc3339(),
    }
}

// ---------- keystore ----------

pub fn key_dir() -> PathBuf {
    if let Ok(p) = std::env::var(KEY_DIR_ENV) {
        return PathBuf::from(p);
    }
    let mut p = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push(".hwledger");
    p.push("attest-keys");
    p
}

pub fn load_signing_key(key_id: &str) -> Result<ed25519_dalek::SigningKey> {
    let mut p = key_dir();
    p.push(format!("{key_id}.sk"));
    let bytes = fs::read(&p)
        .map_err(|_| AttestError::NoKey(format!("missing signing key at {}", p.display())))?;
    if bytes.len() != 32 {
        return Err(AttestError::Sig(format!(
            "signing key must be 32 raw bytes, got {}",
            bytes.len()
        )));
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(ed25519_dalek::SigningKey::from_bytes(&arr))
}

pub fn load_verifying_key(key_id: &str) -> Result<ed25519_dalek::VerifyingKey> {
    let mut p = key_dir();
    p.push(format!("{key_id}.pub"));
    let bytes = fs::read(&p)
        .map_err(|_| AttestError::NoKey(format!("missing public key at {}", p.display())))?;
    if bytes.len() != 32 {
        return Err(AttestError::Sig(format!(
            "public key must be 32 raw bytes, got {}",
            bytes.len()
        )));
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    ed25519_dalek::VerifyingKey::from_bytes(&arr).map_err(|e| AttestError::Sig(e.to_string()))
}

/// Generate a new dev key pair and write to the keystore. Idempotent-ish —
/// refuses to overwrite an existing pair.
pub fn generate_keypair(key_id: &str) -> Result<()> {
    let dir = key_dir();
    fs::create_dir_all(&dir)?;
    let sk_path = dir.join(format!("{key_id}.sk"));
    let pk_path = dir.join(format!("{key_id}.pub"));
    if sk_path.exists() || pk_path.exists() {
        return Err(AttestError::Sig(format!("key {key_id} already exists; rotate via new id")));
    }
    use rand::rngs::OsRng;
    let sk = ed25519_dalek::SigningKey::generate(&mut OsRng);
    let pk = sk.verifying_key();
    fs::write(&sk_path, sk.to_bytes())?;
    fs::write(&pk_path, pk.to_bytes())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&sk_path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

// ---------- build / sign / verify ----------

pub struct BuildInput {
    pub commit_sha: String,
    pub tree_hash: String,
    pub parent_attestation_hash: Option<String>,
    pub checks: Vec<CheckResult>,
    pub key_id: String,
}

pub fn build_and_sign(input: BuildInput) -> Result<Attestation> {
    let payload = AttestationPayload {
        version: SCHEMA_VERSION,
        commit_sha: input.commit_sha,
        tree_hash: input.tree_hash,
        parent_attestation_hash: input.parent_attestation_hash,
        checks: input.checks,
        host: host_info(),
    };
    let canon = payload.canonical_bytes()?;
    let sk = load_signing_key(&input.key_id)?;
    use ed25519_dalek::Signer;
    let sig = sk.sign(&canon);
    let sig_hex = hex::encode(sig.to_bytes());
    let signature = Signature { key_id: input.key_id, sig_hex: sig_hex.clone() };
    let mut h = Sha256::new();
    h.update(&canon);
    h.update(b"|");
    h.update(sig_hex.as_bytes());
    let hash = hex::encode(h.finalize());
    Ok(Attestation { payload, signature, hash })
}

pub fn verify_attestation(a: &Attestation) -> Result<()> {
    let canon = a.payload.canonical_bytes()?;
    let vk = load_verifying_key(&a.signature.key_id)?;
    let sig_bytes = hex::decode(&a.signature.sig_hex)?;
    if sig_bytes.len() != 64 {
        return Err(AttestError::Sig("signature must be 64 bytes".into()));
    }
    let mut arr = [0u8; 64];
    arr.copy_from_slice(&sig_bytes);
    let sig = ed25519_dalek::Signature::from_bytes(&arr);
    use ed25519_dalek::Verifier;
    vk.verify(&canon, &sig).map_err(|e| AttestError::Sig(e.to_string()))?;
    // Re-derive hash and compare.
    let mut h = Sha256::new();
    h.update(&canon);
    h.update(b"|");
    h.update(a.signature.sig_hex.as_bytes());
    let expect = hex::encode(h.finalize());
    if expect != a.hash {
        return Err(AttestError::Tamper(format!(
            "hash mismatch: stored={} computed={}",
            a.hash, expect
        )));
    }
    Ok(())
}

// ---------- log (append-only chain) ----------

pub fn log_path(repo: &Path) -> PathBuf {
    repo.join(LOG_PATH)
}

pub fn append_to_log(repo: &Path, a: &Attestation) -> Result<()> {
    let p = log_path(repo);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = fs::OpenOptions::new().create(true).append(true).open(&p)?;
    let line = serde_json::to_string(a)?;
    writeln!(f, "{line}")?;
    Ok(())
}

pub fn read_log(repo: &Path) -> Result<Vec<Attestation>> {
    let p = log_path(repo);
    if !p.exists() {
        return Ok(Vec::new());
    }
    let f = fs::File::open(&p)?;
    let r = BufReader::new(f);
    let mut out = Vec::new();
    for line in r.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let a: Attestation = serde_json::from_str(&line)?;
        out.push(a);
    }
    Ok(out)
}

pub fn last_attestation_hash(repo: &Path) -> Result<Option<String>> {
    let entries = read_log(repo)?;
    Ok(entries.last().map(|a| a.hash.clone()))
}

#[derive(Debug, Clone, Serialize)]
pub struct ChainReport {
    pub total: usize,
    pub ok: bool,
    pub errors: Vec<String>,
}

pub fn verify_chain(repo: &Path) -> Result<ChainReport> {
    let entries = read_log(repo)?;
    let mut errors = Vec::new();
    let mut prev_hash: Option<String> = None;
    for (i, a) in entries.iter().enumerate() {
        // signature + self-hash
        if let Err(e) = verify_attestation(a) {
            errors.push(format!("line {i}: {e}"));
            continue;
        }
        // chain linkage
        if a.payload.parent_attestation_hash != prev_hash {
            errors.push(format!(
                "line {}: parent mismatch — expected {:?}, got {:?}",
                i, prev_hash, a.payload.parent_attestation_hash
            ));
        }
        prev_hash = Some(a.hash.clone());
    }
    Ok(ChainReport { total: entries.len(), ok: errors.is_empty(), errors })
}

// ---------- check runner ----------

/// Run a subprocess, capture stdout+stderr, hash them for evidence.
pub fn run_check(name: &str, cmd: &str, args: &[&str]) -> CheckResult {
    let start = std::time::Instant::now();
    let out = Command::new(cmd).args(args).output();
    let duration_ms = start.elapsed().as_millis() as u64;
    match out {
        Ok(o) => {
            let mut evidence = Vec::new();
            evidence.extend_from_slice(&o.stdout);
            evidence.extend_from_slice(&o.stderr);
            CheckResult {
                name: name.to_string(),
                passed: o.status.success(),
                duration_ms,
                evidence_sha256: sha256_hex(&evidence),
            }
        }
        Err(e) => CheckResult {
            name: name.to_string(),
            passed: false,
            duration_ms,
            evidence_sha256: sha256_hex(e.to_string().as_bytes()),
        },
    }
}

// ---------- tests ----------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::TempDir;

    // Serialise tests that mutate the HWLEDGER_ATTEST_KEY_DIR env var.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn setup_keys(tmp: &Path, id: &str) {
        std::env::set_var(KEY_DIR_ENV, tmp);
        generate_keypair(id).unwrap();
    }

    fn mk_input(parent: Option<String>, id: &str) -> BuildInput {
        BuildInput {
            commit_sha: "deadbeef".into(),
            tree_hash: "cafef00d".into(),
            parent_attestation_hash: parent,
            checks: vec![CheckResult {
                name: "fake".into(),
                passed: true,
                duration_ms: 1,
                evidence_sha256: sha256_hex(b""),
            }],
            key_id: id.into(),
        }
    }

    /// Traces to: FR-ATTEST-001 — round-trip build/sign/verify.
    #[test]
    fn roundtrip() {
        let _g = ENV_LOCK.lock().unwrap();
        let tmp = TempDir::new().unwrap();
        setup_keys(tmp.path(), "dev1");
        let a = build_and_sign(mk_input(None, "dev1")).unwrap();
        verify_attestation(&a).unwrap();
    }

    /// Traces to: FR-ATTEST-002 — chain with 3 entries verifies clean.
    #[test]
    fn chain_three_entries() {
        let _g = ENV_LOCK.lock().unwrap();
        let tmp = TempDir::new().unwrap();
        setup_keys(tmp.path(), "dev1");
        let repo = TempDir::new().unwrap();
        let mut parent = None;
        for _ in 0..3 {
            let a = build_and_sign(mk_input(parent.clone(), "dev1")).unwrap();
            append_to_log(repo.path(), &a).unwrap();
            parent = Some(a.hash);
        }
        let rep = verify_chain(repo.path()).unwrap();
        assert!(rep.ok, "errors: {:?}", rep.errors);
        assert_eq!(rep.total, 3);
    }

    /// Traces to: FR-ATTEST-003 — tamper detection (flip a byte).
    #[test]
    fn tamper_detected() {
        let _g = ENV_LOCK.lock().unwrap();
        let tmp = TempDir::new().unwrap();
        setup_keys(tmp.path(), "dev1");
        let mut a = build_and_sign(mk_input(None, "dev1")).unwrap();
        a.payload.commit_sha = "tampered".into(); // mutate without re-signing
        assert!(verify_attestation(&a).is_err());
    }

    /// Traces to: FR-ATTEST-004 — missing parent detection.
    #[test]
    fn missing_parent_detected() {
        let _g = ENV_LOCK.lock().unwrap();
        let tmp = TempDir::new().unwrap();
        setup_keys(tmp.path(), "dev1");
        let repo = TempDir::new().unwrap();
        let a1 = build_and_sign(mk_input(None, "dev1")).unwrap();
        append_to_log(repo.path(), &a1).unwrap();
        // second entry claims no parent even though one exists → break
        let a2 = build_and_sign(mk_input(None, "dev1")).unwrap();
        append_to_log(repo.path(), &a2).unwrap();
        let rep = verify_chain(repo.path()).unwrap();
        assert!(!rep.ok);
        assert!(rep.errors.iter().any(|e| e.contains("parent mismatch")));
    }

    /// Traces to: FR-ATTEST-005 — canonical JSON is key-order independent.
    #[test]
    fn canonical_stable() {
        let a: serde_json::Value = serde_json::from_str(r#"{"b":2,"a":1}"#).unwrap();
        let b: serde_json::Value = serde_json::from_str(r#"{"a":1,"b":2}"#).unwrap();
        assert_eq!(canonical_json(&a), canonical_json(&b));
    }
}
