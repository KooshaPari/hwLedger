//! PRD.md parser — extracts FR and NFR specifications.
//!
//! Traces to: NFR-006

use regex::Regex;
use std::fs;
use thiserror::Error;

/// Enumeration of FR/NFR kinds.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum FrKind {
    #[default]
    Fr,
    Nfr,
    NfrVerify,
}

/// User-visible journey kind extracted from an inline `[journey_kind: ...]` tag.
///
/// Traces to: FR-TRACE-001
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JourneyKind {
    Cli,
    Gui,
    Web,
}

impl JourneyKind {
    /// Parse a single kind token (case-insensitive).
    pub fn parse(tok: &str) -> Option<Self> {
        match tok.trim().to_ascii_lowercase().as_str() {
            "cli" => Some(JourneyKind::Cli),
            "gui" => Some(JourneyKind::Gui),
            "web" => Some(JourneyKind::Web),
            _ => None,
        }
    }

    /// Stable string label (matches manifest categories).
    pub fn as_str(&self) -> &'static str {
        match self {
            JourneyKind::Cli => "cli",
            JourneyKind::Gui => "gui",
            JourneyKind::Web => "web",
        }
    }
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
#[derive(Debug, Clone, Default)]
pub struct FrSpec {
    pub id: String,
    pub kind: FrKind,
    pub description: String,
    pub section: String,
    /// Journey kinds declared via inline `[journey_kind: cli,gui,web]` tag.
    /// Empty when no tag is present.
    ///
    /// Traces to: FR-TRACE-001
    pub journey_kinds: Vec<JourneyKind>,
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
        // and also: - **FR-PLAN-001** [journey_kind: cli,web]: Description text
        let fr_pattern = Regex::new(
            r"(?m)^-\s+\*\*([A-Z]+(?:-[A-Z]+)*-\d+)\*\*(?:\s*\[journey_kind:\s*([^\]]+)\])?\s*:\s*(.+?)$",
        )?;

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
                let journey_tag = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                let description = caps.get(3).map(|m| m.as_str()).unwrap_or("").to_string();

                let journey_kinds: Vec<JourneyKind> = if journey_tag.is_empty() {
                    Vec::new()
                } else {
                    journey_tag.split(',').filter_map(JourneyKind::parse).collect()
                };

                if !id.is_empty() {
                    let kind = FrKind::from_id(&id);
                    frs.push(FrSpec {
                        id,
                        kind,
                        description,
                        section: current_section.clone(),
                        journey_kinds,
                    });
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
