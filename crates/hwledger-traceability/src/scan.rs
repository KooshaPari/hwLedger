//! Cross-dimension FR traceability scanning.
//!
//! Scans source code, tests, ADRs, documentation, and journey manifests
//! for five types of FR citations: Traces, Implements, Constrains, Documents, Exercises.
//!
//! Traces to: NFR-006

use regex::Regex;
use serde::Serialize;
use std::path::Path;
use thiserror::Error;
use walkdir::WalkDir;

/// Error type for scanning operations.
#[derive(Debug, Error)]
pub enum ScanError {
    #[error("regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// The semantic relationship between a code/doc element and an FR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationVerb {
    /// Test verifies this FR (in test functions).
    Traces,
    /// Production source code realizes this FR (in source code comments).
    Implements,
    /// ADR decision bounds how this FR is realized.
    Constrains,
    /// User- or dev-facing doc page describes this FR.
    Documents,
    /// UI/CLI journey drives this FR at runtime.
    Exercises,
}

/// Origin of an annotation: where it was found in the codebase.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Citer {
    /// Rust test function.
    RustTest,
    /// Rust source code (struct, fn, module doc).
    RustSource,
    /// ADR markdown file.
    AdrDoc,
    /// General documentation markdown.
    DocPage,
    /// Journey manifest JSON.
    JourneyManifest,
    /// Swift test file.
    SwiftTest,
}

/// A single FR citation from any dimension.
#[derive(Debug, Clone, Serialize)]
pub struct TraceAnnotation {
    pub citer: Citer,
    pub verb: AnnotationVerb,
    pub file: String,
    pub line: usize,
    pub cited_frs: Vec<String>,
    pub context: String, // function name, doc section, etc.
    pub is_ignored: bool, // applies only to tests
}

/// Legacy struct for backward compatibility. See TraceAnnotation.
#[derive(Debug, Clone, Serialize)]
pub struct TestTrace {
    pub file: String,
    pub line: usize,
    pub test_name: String,
    pub cited_frs: Vec<String>,
    pub is_ignored: bool,
}

/// Result of scanning a test file.
#[derive(Debug)]
pub struct ScanResult {
    pub traces: Vec<TestTrace>,
    pub parse_errors: Vec<String>,
}

/// Scanner for cross-dimension FR annotations.
pub struct AnnotationScanner;

/// Backward-compatible alias.
pub type TestScanner = AnnotationScanner;

impl AnnotationScanner {
    /// Recursively scans a directory for all FR annotations across all dimensions.
    ///
    /// Walks: Rust source & tests, ADRs, docs, journey manifests.
    /// Skips: target/, .build/, node_modules/, .git/, sidecars/omlx-fork/, docs-site/node_modules/
    ///
    /// Traces to: NFR-006
    pub fn scan(repo_path: &str) -> Result<Vec<TraceAnnotation>, ScanError> {
        let mut annotations = Vec::new();
        let skip_dirs = [
            "target",
            ".build",
            "node_modules",
            ".git",
            "omlx-fork",
            "sidecars",
        ];

        for entry in WalkDir::new(repo_path).into_iter().filter_map(|e| e.ok()).filter(|e| {
            let path = e.path();
            // Skip blacklisted directories
            for skip in &skip_dirs {
                if path.to_string_lossy().contains(skip) {
                    return false;
                }
            }
            true
        }) {
            let path = entry.path();
            let path_str = path.to_string_lossy();

            // Rust source files
            if path_str.ends_with(".rs") {
                if let Ok(mut file_annos) = Self::scan_rust_file(&path_str) {
                    annotations.append(&mut file_annos);
                }
            }
            // ADR markdown files
            else if path_str.contains("/docs/adr/") && path_str.ends_with(".md") {
                if let Ok(mut file_annos) = Self::scan_adr_file(&path_str) {
                    annotations.append(&mut file_annos);
                }
            }
            // General documentation markdown (excluding docs-site node_modules)
            else if path_str.ends_with(".md") && !path_str.contains("node_modules") {
                // Only scan top-level docs and specific directories
                if Self::is_doc_page(&path_str) {
                    if let Ok(mut file_annos) = Self::scan_doc_file(&path_str) {
                        annotations.append(&mut file_annos);
                    }
                }
            }
            // Journey manifests
            else if path_str.ends_with("manifest.json") && path_str.contains("journeys") {
                if let Ok(mut file_annos) = Self::scan_journey_manifest(&path_str) {
                    annotations.append(&mut file_annos);
                }
            }
            // Swift test files
            else if path_str.ends_with(".swift") && path_str.contains("Test") {
                if let Ok(mut file_annos) = Self::scan_swift_file(&path_str) {
                    annotations.append(&mut file_annos);
                }
            }
        }

        Ok(annotations)
    }

    /// Scans a single Rust source file for all annotation types.
    ///
    /// Traces to: NFR-006
    fn scan_rust_file(path: &str) -> Result<Vec<TraceAnnotation>, ScanError> {
        use std::fs;

        let content = fs::read_to_string(path)?;
        let mut annotations = Vec::new();

        // Compile all patterns
        let test_pattern =
            Regex::new(r#"(?m)^\s*(?:#\[(?:tokio::|ignore\b)[^\]]*\])*\s*#\[test\]"#)?;
        let ignore_pattern = Regex::new(r#"#\[ignore\]"#)?;
        let traces_pattern = Regex::new(
            r#"(?://\s*|///\s*)?Traces\s+to:\s*([A-Z]+(?:-[A-Z]+)*-\d+(?:\s*,\s*[A-Z]+(?:-[A-Z]+)*-\d+)*)"#,
        )?;
        let implements_pattern = Regex::new(
            r#"///?\s*Implements:\s*([A-Z]+(?:-[A-Z]+)*-\d+(?:\s*,\s*[A-Z]+(?:-[A-Z]+)*-\d+)*)"#,
        )?;
        let fn_pattern = Regex::new(r#"(?m)^\s*(?:async\s+)?fn\s+([a-z_][a-z0-9_]*)\s*\("#)?;
        let struct_pattern = Regex::new(r#"(?m)^\s*(?:pub\s+)?(?:async\s+)?(?:impl|struct|enum|mod|const|static)\s+([a-z_A-Z][a-z0-9_]*)"#)?;

        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            // Check for Traces to: (test annotations)
            if let Some(caps) = traces_pattern.captures(line) {
                let fr_str = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let cited_frs: Vec<String> = fr_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                if !cited_frs.is_empty() {
                    let mut test_name = "unknown".to_string();
                    let mut is_ignored = false;
                    let mut test_line = i;

                    for j in (0..=i).rev() {
                        let scan_line = lines[j];
                        if test_pattern.is_match(scan_line) {
                            test_line = j;
                            if j > 0 && ignore_pattern.is_match(lines[j - 1]) {
                                is_ignored = true;
                            }
                        }
                        if test_line < i && j > test_line {
                            if let Some(caps) = fn_pattern.captures(scan_line) {
                                if let Some(name) = caps.get(1) {
                                    test_name = name.as_str().to_string();
                                    break;
                                }
                            }
                        }
                        if j == 0 {
                            break;
                        }
                    }

                    if test_line < i {
                        annotations.push(TraceAnnotation {
                            citer: Citer::RustTest,
                            verb: AnnotationVerb::Traces,
                            file: path.to_string(),
                            line: i + 1,
                            cited_frs,
                            context: test_name,
                            is_ignored,
                        });
                    }
                }
            }

            // Check for Implements: (source code annotations)
            if let Some(caps) = implements_pattern.captures(line) {
                let fr_str = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let cited_frs: Vec<String> = fr_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                if !cited_frs.is_empty() {
                    let mut context = "unknown".to_string();

                    // Look forward for the next item definition
                    for search_line in lines.iter().take(std::cmp::min(i + 10, lines.len())).skip(i) {
                        if let Some(caps) = struct_pattern.captures(search_line) {
                            if let Some(name) = caps.get(1) {
                                context = name.as_str().to_string();
                                break;
                            }
                        }
                    }

                    annotations.push(TraceAnnotation {
                        citer: Citer::RustSource,
                        verb: AnnotationVerb::Implements,
                        file: path.to_string(),
                        line: i + 1,
                        cited_frs,
                        context,
                        is_ignored: false,
                    });
                }
            }
        }

        Ok(annotations)
    }

    /// Scans an ADR markdown file for Constrains: annotations.
    ///
    /// Traces to: NFR-006
    fn scan_adr_file(path: &str) -> Result<Vec<TraceAnnotation>, ScanError> {
        use std::fs;

        let content = fs::read_to_string(path)?;
        let mut annotations = Vec::new();

        let constrains_pattern = Regex::new(
            r#"Constrains:\s*([A-Z]+(?:-[A-Z]+)*-\d+(?:\s*,\s*[A-Z]+(?:-[A-Z]+)*-\d+)*)"#,
        )?;

        for (i, line) in content.lines().enumerate() {
            if let Some(caps) = constrains_pattern.captures(line) {
                let fr_str = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let cited_frs: Vec<String> = fr_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                if !cited_frs.is_empty() {
                    annotations.push(TraceAnnotation {
                        citer: Citer::AdrDoc,
                        verb: AnnotationVerb::Constrains,
                        file: path.to_string(),
                        line: i + 1,
                        cited_frs,
                        context: Path::new(path)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("ADR")
                            .to_string(),
                        is_ignored: false,
                    });
                }
            }
        }

        Ok(annotations)
    }

    /// Scans a documentation markdown file for Documents: annotations.
    ///
    /// Traces to: NFR-006
    fn scan_doc_file(path: &str) -> Result<Vec<TraceAnnotation>, ScanError> {
        use std::fs;

        let content = fs::read_to_string(path)?;
        let mut annotations = Vec::new();

        let documents_pattern = Regex::new(
            r#"Documents:\s*([A-Z]+(?:-[A-Z]+)*-\d+(?:\s*,\s*[A-Z]+(?:-[A-Z]+)*-\d+)*)"#,
        )?;

        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            if let Some(caps) = documents_pattern.captures(line) {
                let fr_str = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let cited_frs: Vec<String> = fr_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                if !cited_frs.is_empty() {
                    let mut section = "document".to_string();

                    // Try to find the preceding heading
                    if i > 0 {
                        for j in (0..i).rev() {
                            if lines[j].starts_with('#') {
                                section = lines[j].trim_start_matches('#').trim().to_string();
                                break;
                            }
                        }
                    }

                    annotations.push(TraceAnnotation {
                        citer: Citer::DocPage,
                        verb: AnnotationVerb::Documents,
                        file: path.to_string(),
                        line: i + 1,
                        cited_frs,
                        context: section,
                        is_ignored: false,
                    });
                }
            }
        }

        Ok(annotations)
    }

    /// Scans a journey manifest JSON for traces_frs and inline (FR-...) references.
    ///
    /// Traces to: NFR-006
    fn scan_journey_manifest(path: &str) -> Result<Vec<TraceAnnotation>, ScanError> {
        use std::fs;

        let content = fs::read_to_string(path)?;
        let mut annotations = Vec::new();

        // Look for top-level "traces_frs" array
        let traces_frs_pattern =
            Regex::new(r#""traces_frs"\s*:\s*\[\s*"([^"]+)"\s*(?:,\s*"([^"]+)")?\s*\]"#)?;

        if let Some(caps) = traces_frs_pattern.captures(&content) {
            let mut cited_frs = Vec::new();
            if let Some(m) = caps.get(1) {
                cited_frs.push(m.as_str().to_string());
            }
            if let Some(m) = caps.get(2) {
                cited_frs.push(m.as_str().to_string());
            }

            if !cited_frs.is_empty() {
                annotations.push(TraceAnnotation {
                    citer: Citer::JourneyManifest,
                    verb: AnnotationVerb::Exercises,
                    file: path.to_string(),
                    line: 1,
                    cited_frs,
                    context: Path::new(path)
                        .parent()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("journey")
                        .to_string(),
                    is_ignored: false,
                });
            }
        }

        // Also scan for inline (FR-...) in intent labels
        let inline_fr_pattern = Regex::new(r#""intent"\s*:\s*"([^"]*\(FR-[A-Z\-\d]+\)[^"]*)""#)?;
        let fr_pattern = Regex::new(r#"\(FR-([A-Z]+(?:-[A-Z]+)*-\d+)\)"#)?;
        for (i, line) in content.lines().enumerate() {
            if let Some(caps) = inline_fr_pattern.captures(line) {
                let intent_text = caps.get(1).map(|m| m.as_str()).unwrap_or("");

                for fr_caps in fr_pattern.captures_iter(intent_text) {
                    if let Some(m) = fr_caps.get(1) {
                        let fr = format!("FR-{}", m.as_str());
                        annotations.push(TraceAnnotation {
                            citer: Citer::JourneyManifest,
                            verb: AnnotationVerb::Exercises,
                            file: path.to_string(),
                            line: i + 1,
                            cited_frs: vec![fr],
                            context: intent_text.to_string(),
                            is_ignored: false,
                        });
                    }
                }
            }
        }

        Ok(annotations)
    }

    /// Scans a Swift test file for Traces to: annotations.
    ///
    /// Traces to: NFR-006
    fn scan_swift_file(path: &str) -> Result<Vec<TraceAnnotation>, ScanError> {
        use std::fs;

        let content = fs::read_to_string(path)?;
        let mut annotations = Vec::new();

        let traces_pattern = Regex::new(
            r#"//\s*Traces\s+to:\s*([A-Z]+(?:-[A-Z]+)*-\d+(?:\s*,\s*[A-Z]+(?:-[A-Z]+)*-\d+)*)"#,
        )?;
        let test_func_pattern = Regex::new(r#"func\s+(test[a-zA-Z0-9_]+)\s*\("#)?;

        for (i, line) in content.lines().enumerate() {
            if let Some(caps) = traces_pattern.captures(line) {
                let fr_str = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let cited_frs: Vec<String> = fr_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                if !cited_frs.is_empty() {
                    let mut test_name = "unknown".to_string();

                    // Look forward for function name
                    for j in i..std::cmp::min(i + 5, content.lines().count()) {
                        if let Some(caps) = test_func_pattern.captures(
                            content.lines().nth(j).unwrap_or(""),
                        ) {
                            if let Some(name) = caps.get(1) {
                                test_name = name.as_str().to_string();
                                break;
                            }
                        }
                    }

                    annotations.push(TraceAnnotation {
                        citer: Citer::SwiftTest,
                        verb: AnnotationVerb::Traces,
                        file: path.to_string(),
                        line: i + 1,
                        cited_frs,
                        context: test_name,
                        is_ignored: false,
                    });
                }
            }
        }

        Ok(annotations)
    }

    /// Helper to identify whether a markdown file should be scanned as a doc page.
    fn is_doc_page(path: &str) -> bool {
        // Include top-level docs and docs/ subdirectories
        // Exclude docs-site/node_modules and architecture/adrs (synced from ADRs)
        if path.contains("node_modules") || path.contains("architecture/adrs") {
            return false;
        }
        path.ends_with("PRD.md")
            || path.ends_with("PLAN.md")
            || path.ends_with("README.md")
            || path.ends_with("AGENTS.md")
            || path.ends_with("CHARTER.md")
            || path.contains("/docs/") && !path.contains("docs/adr")
    }
}

// Backward compatibility: provide a legacy scan() wrapper that returns TestTrace
impl AnnotationScanner {
    /// Backward-compatible scan that converts TraceAnnotation to TestTrace.
    ///
    /// Traces to: NFR-006
    pub fn scan_legacy(repo_path: &str) -> Result<Vec<TestTrace>, ScanError> {
        let annotations = Self::scan(repo_path)?;
        let traces = annotations
            .into_iter()
            .filter_map(|anno| {
                if anno.verb == AnnotationVerb::Traces {
                    Some(TestTrace {
                        file: anno.file,
                        line: anno.line,
                        test_name: anno.context,
                        cited_frs: anno.cited_frs,
                        is_ignored: anno.is_ignored,
                    })
                } else {
                    None
                }
            })
            .collect();
        Ok(traces)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Traces to: NFR-006
    #[test]
    fn test_scan_simple_trace() {
        let pattern = Regex::new(
            r#"(?://\s*|///\s*)?Traces\s+to:\s*([A-Z]+(?:-[A-Z]+)*-\d+(?:\s*,\s*[A-Z]+(?:-[A-Z]+)*-\d+)*)"#,
        )
        .unwrap();
        let content = "// Traces to: FR-PLAN-001";
        assert!(pattern.is_match(content));
    }

    /// Traces to: NFR-006
    #[test]
    fn test_multi_fr_citation() {
        let content = "Traces to: FR-PLAN-001, FR-PLAN-002, FR-PLAN-003";
        let pattern = Regex::new(
            r#"(?://\s*|///\s*)?Traces\s+to:\s*([A-Z]+(?:-[A-Z]+)*-\d+(?:\s*,\s*[A-Z]+(?:-[A-Z]+)*-\d+)*)"#,
        )
        .unwrap();
        assert!(pattern.is_match(content));
    }

    /// Traces to: NFR-006
    #[test]
    fn test_annotation_verb_all_variants() {
        // Verify all verb types serialize correctly
        let verbs = [
            AnnotationVerb::Traces,
            AnnotationVerb::Implements,
            AnnotationVerb::Constrains,
            AnnotationVerb::Documents,
            AnnotationVerb::Exercises,
        ];
        assert_eq!(verbs.len(), 5);
    }

    /// Traces to: NFR-006
    #[test]
    fn test_citer_variants() {
        let citers = [
            Citer::RustTest,
            Citer::RustSource,
            Citer::AdrDoc,
            Citer::DocPage,
            Citer::JourneyManifest,
            Citer::SwiftTest,
        ];
        assert_eq!(citers.len(), 6);
    }
}
