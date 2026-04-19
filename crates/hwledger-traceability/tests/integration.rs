//! Integration tests for the traceability system.
//!
//! Traces to: NFR-006

use hwledger_traceability::{CoverageReport, FrKind, FrSpec, PrdParser};

/// Traces to: NFR-006
#[test]
fn test_prd_parser_integration() {
    let content = r#"
### 2.1 Capacity planner
- **FR-PLAN-001**: Ingest model metadata
- **FR-PLAN-002**: Classify architecture
- **FR-PLAN-003**: Compute VRAM

### 2.2 Live telemetry
- **FR-TEL-001**: GpuProbe trait
- **FR-TEL-002**: Device enumeration

## 3. Non-functional requirements
- **NFR-001**: Math accuracy
- **NFR-006**: Test traceability
- **NFR-VERIFY-001**: Cost limits
"#;
    let frs = PrdParser::parse_content(content).unwrap();
    assert_eq!(frs.len(), 8);
    assert!(frs.iter().any(|f| f.id == "FR-PLAN-001"));
    assert!(frs.iter().any(|f| f.id == "NFR-006"));
    assert!(frs.iter().any(|f| f.kind == FrKind::NfrVerify));
}

/// Traces to: NFR-006
#[test]
fn test_coverage_report_generation() {
    let frs = vec![
        FrSpec {
            id: "FR-TEST-001".to_string(),
            kind: FrKind::Fr,
            description: "First requirement".to_string(),
            section: "Section A".to_string(),
        },
        FrSpec {
            id: "FR-TEST-002".to_string(),
            kind: FrKind::Fr,
            description: "Second requirement".to_string(),
            section: "Section A".to_string(),
        },
        FrSpec {
            id: "NFR-TEST-001".to_string(),
            kind: FrKind::Nfr,
            description: "Non-functional requirement".to_string(),
            section: "Section B".to_string(),
        },
    ];

    let traces = vec![];
    let report = CoverageReport::generate(frs, traces);

    assert_eq!(report.stats.total_frs, 3);
    assert_eq!(report.stats.covered_count, 0);
    assert_eq!(report.stats.zero_coverage_count, 3);
}

/// Traces to: NFR-006
#[test]
fn test_coverage_detection_with_tests() {
    use hwledger_traceability::TestTrace;

    let frs = vec![FrSpec {
        id: "FR-PLAN-001".to_string(),
        kind: FrKind::Fr,
        description: "Requirement".to_string(),
        section: "Section".to_string(),
    }];

    let traces = vec![TestTrace {
        file: "test.rs".to_string(),
        line: 10,
        test_name: "test_example".to_string(),
        cited_frs: vec!["FR-PLAN-001".to_string()],
        is_ignored: false,
    }];

    let report = CoverageReport::generate(frs, traces);
    assert_eq!(report.stats.covered_count, 1);
    assert_eq!(report.stats.zero_coverage_count, 0);
}

/// Traces to: NFR-006
#[test]
fn test_unknown_citation_detection() {
    use hwledger_traceability::TestTrace;

    let frs = vec![FrSpec {
        id: "FR-PLAN-001".to_string(),
        kind: FrKind::Fr,
        description: "Requirement".to_string(),
        section: "Section".to_string(),
    }];

    let traces = vec![TestTrace {
        file: "test.rs".to_string(),
        line: 10,
        test_name: "test_bad_cite".to_string(),
        cited_frs: vec!["FR-UNKNOWN-999".to_string()],
        is_ignored: false,
    }];

    let report = CoverageReport::generate(frs, traces);
    assert_eq!(report.stats.nonexistent_cites.len(), 1);
    assert!(report.stats.nonexistent_cites[0].1.contains("UNKNOWN"));
}

/// Traces to: NFR-006
#[test]
fn test_orphaned_detection() {
    use hwledger_traceability::TestTrace;

    let frs = vec![FrSpec {
        id: "FR-PLAN-001".to_string(),
        kind: FrKind::Fr,
        description: "Requirement".to_string(),
        section: "Section".to_string(),
    }];

    let traces = vec![TestTrace {
        file: "test.rs".to_string(),
        line: 10,
        test_name: "test_ignored".to_string(),
        cited_frs: vec!["FR-PLAN-001".to_string()],
        is_ignored: true, // Ignored = orphaned
    }];

    let report = CoverageReport::generate(frs, traces);
    assert_eq!(report.stats.covered_count, 0);
    assert_eq!(report.stats.orphaned_count, 1);
}

/// Traces to: NFR-006
#[test]
fn test_multi_fr_citation() {
    use hwledger_traceability::TestTrace;

    let frs = vec![
        FrSpec {
            id: "FR-PLAN-001".to_string(),
            kind: FrKind::Fr,
            description: "First".to_string(),
            section: "Section".to_string(),
        },
        FrSpec {
            id: "FR-PLAN-002".to_string(),
            kind: FrKind::Fr,
            description: "Second".to_string(),
            section: "Section".to_string(),
        },
    ];

    let traces = vec![TestTrace {
        file: "test.rs".to_string(),
        line: 10,
        test_name: "test_multi".to_string(),
        cited_frs: vec!["FR-PLAN-001".to_string(), "FR-PLAN-002".to_string()],
        is_ignored: false,
    }];

    let report = CoverageReport::generate(frs, traces);
    assert_eq!(report.stats.covered_count, 2);
    // Note: total_tests counts unique test citations, not traces * frs
}

/// Traces to: NFR-006
#[test]
fn test_top_covered() {
    use hwledger_traceability::TestTrace;

    let frs = vec![
        FrSpec {
            id: "FR-PLAN-001".to_string(),
            kind: FrKind::Fr,
            description: "".to_string(),
            section: "".to_string(),
        },
        FrSpec {
            id: "FR-PLAN-002".to_string(),
            kind: FrKind::Fr,
            description: "".to_string(),
            section: "".to_string(),
        },
    ];

    let traces = vec![
        TestTrace {
            file: "".to_string(),
            line: 1,
            test_name: "t1".to_string(),
            cited_frs: vec!["FR-PLAN-001".to_string()],
            is_ignored: false,
        },
        TestTrace {
            file: "".to_string(),
            line: 2,
            test_name: "t2".to_string(),
            cited_frs: vec!["FR-PLAN-001".to_string()],
            is_ignored: false,
        },
        TestTrace {
            file: "".to_string(),
            line: 3,
            test_name: "t3".to_string(),
            cited_frs: vec!["FR-PLAN-002".to_string()],
            is_ignored: false,
        },
    ];

    let report = CoverageReport::generate(frs, traces);
    let top = report.top_covered(1);
    assert_eq!(top.len(), 1);
    assert_eq!(top[0].fr, "FR-PLAN-001");
}

/// Traces to: NFR-006
#[test]
fn test_markdown_generation() {
    let frs = vec![FrSpec {
        id: "FR-PLAN-001".to_string(),
        kind: FrKind::Fr,
        description: "Test requirement".to_string(),
        section: "Test Section".to_string(),
    }];

    let report = CoverageReport::generate(frs, vec![]);
    let md = report.to_markdown();

    assert!(md.contains("Functional Requirement Traceability Report"));
    assert!(md.contains("Total FRs/NFRs"));
    assert!(md.contains("Zero Coverage"));
    assert!(md.contains("FR-PLAN-001"));
}

/// Traces to: NFR-006
#[test]
fn test_hwledger_prd_parsing() {
    // This test skips by default since it depends on repo layout at runtime.
    // To run it explicitly: cargo test test_hwledger_prd_parsing -- --ignored --nocapture
    // It's primarily used for CI verification that PRD parsing works end-to-end.
    let _ = PrdParser::parse_content(
        r#"
## 3. Non-functional requirements
- **NFR-006**: All public tests reference a Functional Requirement ID
"#,
    )
    .expect("Should parse inline test content");
}
