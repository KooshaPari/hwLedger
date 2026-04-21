// One-shot merge-conflict resolver for verified journey manifests.
// Per scripting policy (Rust first), used in place of a shell/sed pipeline.
//
// Strategy: in each manifest containing `<<<<<<< Updated upstream` ...
// `=======` ... `>>>>>>> Stashed changes` blocks, keep ONLY the upstream
// (HEAD) side. The upstream side was produced by the current pending-record
// + re-record toolchain and matches the schema the traceability crate
// expects; the "stashed" side is an older partial blobdiff and is unsafe to
// merge mechanically.
//
// Compile: rustc --edition 2021 tools/pending-record/resolve_conflict_manifests.rs -o /tmp/resolve_conflicts
// Run:     /tmp/resolve_conflicts <file1> <file2> ...

use std::fs;
use std::path::PathBuf;

fn resolve(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut in_upstream = false;
    let mut in_stashed = false;
    for line in content.lines() {
        if line.starts_with("<<<<<<<") {
            in_upstream = true;
            continue;
        }
        if line.starts_with("=======") && in_upstream {
            in_upstream = false;
            in_stashed = true;
            continue;
        }
        if line.starts_with(">>>>>>>") && in_stashed {
            in_stashed = false;
            continue;
        }
        if in_stashed {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn main() {
    let files: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    if files.is_empty() {
        eprintln!("usage: resolve_conflicts <file1> [file2 ...]");
        std::process::exit(1);
    }
    let mut changed = 0usize;
    for f in files {
        let c = fs::read_to_string(&f).unwrap_or_else(|e| panic!("read {}: {}", f.display(), e));
        if !c.contains("<<<<<<<") {
            continue;
        }
        let resolved = resolve(&c);
        fs::write(&f, &resolved).unwrap();
        println!("resolved {}", f.display());
        changed += 1;
    }
    println!("{changed} file(s) resolved");
}
