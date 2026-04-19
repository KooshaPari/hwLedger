//! PRD.md parser — extracts FR and NFR specifications.
//!
//! Traces to: NFR-006

use regex::Regex;
use std::fs;
use thiserror::Error;

/// Enumeration of FR/NFR kinds.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FrKind {
    Fr,
    Nfr,
    NfrVerify,
}

impl FrKind {
    /// Returns the kind from an FR/NFR ID string.
    pub fn from_id(id: &str) -> Self {
        if id.starts_with("NFR-VERIFY") {
            FrKind::NfrVerify
        } else if id.starts_with("NFR-") {
            FrKind::Nfr
        } else {
            FrKind::Fr
        }
    }
}

/// Specification of a single FR/NFR extracted from PRD.md.
#[derive(Debug, Clone)]
pub struct FrSpec {
    pub id: String,
    pub kind: FrKind,
    pub description: String,
    pub section: String,
}

/// Errors from PRD parsing.
#[derive(Debug, Error)]
pub enum PrdError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("PRD format error: {0}")]
    Format(String),
}

/// Parser for PRD.md files.
pub struct PrdParser;

impl PrdParser {
    /// Parses PRD.md and returns all extracted FR/NFR specifications.
    ///
    /// Traces to: NFR-006
    pub fn parse(path: &str) -> Result<Vec<FrSpec>, PrdError> {
        let content = fs::read_to_string(path)?;
        Self::parse_content(&content)
    }

    /// Parses PRD.md content (for testing).
    ///
    /// Traces to: NFR-006
    pub fn parse_content(content: &str) -> Result<Vec<FrSpec>, PrdError> {
        let mut frs = Vec::new();
        let mut current_section = String::from("unknown");

        // Regex to match lines like: - **FR-PLAN-001**: Description text
        let fr_pattern = Regex::new(r"(?m)^-\s+\*\*([A-Z]+(?:-[A-Z]+)*-\d+)\*\*:\s*(.+?)$")?;

        // Track section headers
        let section_pattern = Regex::new(r"(?m)^###?\s+(.+?)$")?;

        for line in content.lines() {
            // Update current section
            if let Some(caps) = section_pattern.captures(line) {
                current_section = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown").to_string();
            }

            // Extract FR/NFR
            if let Some(caps) = fr_pattern.captures(line) {
                let id = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
                let description = caps.get(2).map(|m| m.as_str()).unwrap_or("").to_string();

                if !id.is_empty() {
                    let kind = FrKind::from_id(&id);
                    frs.push(FrSpec { id, kind, description, section: current_section.clone() });
                }
            }
        }

        Ok(frs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Traces to: NFR-006
    #[test]
    fn test_prd_simple_parse() {
        let content = r#"
### 2.1 Capacity planner
- **FR-PLAN-001**: Ingest model metadata from HF Hub
- **FR-PLAN-002**: Classify architecture into variants
"#;
        let frs = PrdParser::parse_content(content).unwrap();
        assert_eq!(frs.len(), 2);
        assert_eq!(frs[0].id, "FR-PLAN-001");
        assert_eq!(frs[1].id, "FR-PLAN-002");
    }

    /// Traces to: NFR-006
    #[test]
    fn test_prd_nfr_variants() {
        let content = r#"
## 3. Non-functional requirements
- **NFR-001**: Planner math ±200 MB
- **NFR-006**: All public tests reference a Functional Requirement ID
- **NFR-VERIFY-001**: Per-journey token cost shall not exceed
"#;
        let frs = PrdParser::parse_content(content).unwrap();
        assert_eq!(frs.len(), 3);
        assert_eq!(frs[0].kind, FrKind::Nfr);
        assert_eq!(frs[1].kind, FrKind::Nfr);
        assert_eq!(frs[2].kind, FrKind::NfrVerify);
    }

    /// Traces to: NFR-006
    #[test]
    fn test_prd_multi_section() {
        let content = r#"
### Section A
- **FR-PLAN-001**: First item

### Section B
- **FR-PLAN-002**: Second item
"#;
        let frs = PrdParser::parse_content(content).unwrap();
        assert_eq!(frs.len(), 2);
        assert_eq!(frs[0].section, "Section A");
        assert_eq!(frs[1].section, "Section B");
    }

    /// Traces to: NFR-006
    #[test]
    fn test_prd_empty() {
        let content = "No requirements here";
        let frs = PrdParser::parse_content(content).unwrap();
        assert_eq!(frs.len(), 0);
    }
}
