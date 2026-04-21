//! Tauri command errors. All commands return `Result<T, CommandError>`; the
//! webview receives a tagged JSON error object on failure.

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("classify failed: {0}")]
    Classify(String),
    /// Reserved for future `ingest`/`resolve_model` commands once they're wired
    /// through the Tauri bridge. Keeps error codes stable for the frontend.
    #[allow(dead_code)]
    #[error("ingest failed: {0}")]
    Ingest(String),
    #[error("probe failed: {0}")]
    Probe(String),
    #[error("hf search failed: {0}")]
    HfSearch(String),
    #[error("internal: {0}")]
    Internal(String),
}

impl Serialize for CommandError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Out<'a> {
            code: &'a str,
            message: String,
        }
        let code = match self {
            CommandError::InvalidInput(_) => "invalid_input",
            CommandError::Classify(_) => "classify",
            CommandError::Ingest(_) => "ingest",
            CommandError::Probe(_) => "probe",
            CommandError::HfSearch(_) => "hf_search",
            CommandError::Internal(_) => "internal",
        };
        Out { code, message: self.to_string() }.serialize(s)
    }
}
