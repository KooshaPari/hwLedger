//! MCP tool implementations exposed by `hwledger-mcp`.
//!
//! These are intentionally stub-level for v1: each tool returns a stable
//! JSON shape that downstream clients (LLM tool-using agents, the spec
//! evals, etc.) can rely on, while the real search/RAG backends are wired
//! up in subsequent phases. Stubs never `panic` and never return `Err` for
//! well-formed input — they always hand back a `serde_json::Value` so the
//! JSON-RPC layer can wrap it as a `tools/call` result.
//!
//! The six tools are the canonical MCP-2024-11-05 surface for the model
//! explorer:
//!
//! 1. `model_search`        — text + facet search over the model index
//! 2. `model_detail`        — full record for a single `(source, id)`
//! 3. `model_rag_ask`       — RAG-grounded answer to a natural-language Q
//! 4. `model_quants`        — list quantized variants for a model
//! 5. `similar_models`      — find models nearest to a seed
//! 6. `models_for_use_case` — facet-based "which fits my use case?" filter
//!
//! Each tool advertises a JSON-Schema-ish parameter block via
//! [`tool_definitions`]; the server itself is schema-agnostic and only
//! validates that `params` is a JSON object.

use serde_json::{json, Value};

use crate::error::McpError;

/// Return the list of MCP tool descriptors advertised by `tools/list`.
///
/// The shape mirrors what the MCP 2024-11-05 spec expects under
/// `result.tools[]`: each entry has a stable `name`, a human-readable
/// `description`, and an `inputSchema` describing its arguments.
pub fn tool_definitions() -> Value {
    json!([
        {
            "name": "model_search",
            "description": "Full-text + faceted search over the hwledger model index. Returns a ranked list of candidate model refs.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Natural-language or keyword query."},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 50, "default": 10},
                    "facets": {"type": "object", "description": "Optional facet filters (modality, arch, license, ...)."}
                },
                "required": ["query"],
                "additionalProperties": true
            }
        },
        {
            "name": "model_detail",
            "description": "Fetch the full record for a single model identified by (source, id).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": {"type": "string", "description": "Upstream source key (e.g. 'hf', 'internal')."},
                    "id":     {"type": "string", "description": "Source-specific model identifier."}
                },
                "required": ["source", "id"],
                "additionalProperties": false
            }
        },
        {
            "name": "model_rag_ask",
            "description": "Answer a natural-language question about models, grounded on indexed card text via RAG.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "question": {"type": "string", "description": "Free-form question."},
                    "top_k":    {"type": "integer", "minimum": 1, "maximum": 20, "default": 5}
                },
                "required": ["question"],
                "additionalProperties": true
            }
        },
        {
            "name": "model_quants",
            "description": "List known quantization variants (gguf, gptq, awq, ...) for a model.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": {"type": "string"},
                    "id":     {"type": "string"}
                },
                "required": ["source", "id"],
                "additionalProperties": false
            }
        },
        {
            "name": "similar_models",
            "description": "Return models most similar to the given seed model (cosine over embedding + tag overlap).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": {"type": "string"},
                    "id":     {"type": "string"},
                    "limit":  {"type": "integer", "minimum": 1, "maximum": 50, "default": 10}
                },
                "required": ["source", "id"],
                "additionalProperties": true
            }
        },
        {
            "name": "models_for_use_case",
            "description": "Recommend models that fit a target use case (chat, code, embedding, vision, ...).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "use_case": {"type": "string", "description": "e.g. 'chat', 'code', 'embedding', 'vision', 'tool-use'."},
                    "limit":    {"type": "integer", "minimum": 1, "maximum": 50, "default": 10},
                    "constraints": {"type": "object", "description": "Optional hard constraints (license, size, ...)."}
                },
                "required": ["use_case"],
                "additionalProperties": true
            }
        }
    ])
}

/// Lookup the registered tool name list. Used by tests and by `tools/list`.
pub fn tool_names() -> &'static [&'static str] {
    &[
        "model_search",
        "model_detail",
        "model_rag_ask",
        "model_quants",
        "similar_models",
        "models_for_use_case",
    ]
}

/// Dispatch a single `tools/call` invocation to the matching stub.
///
/// `params` is the raw JSON object from the `tools/call` request. The
/// function is responsible for the per-tool "is this structurally valid?"
/// check and returns [`McpError::InvalidParams`] on failure; a
/// well-formed call always returns a stub JSON value.
pub fn call_tool(name: &str, params: &Value) -> Result<Value, McpError> {
    let obj = params
        .as_object()
        .ok_or_else(|| McpError::invalid_params("params must be a JSON object"))?;

    match name {
        "model_search" => model_search(obj),
        "model_detail" => model_detail(obj),
        "model_rag_ask" => model_rag_ask(obj),
        "model_quants" => model_quants(obj),
        "similar_models" => similar_models(obj),
        "models_for_use_case" => models_for_use_case(obj),
        other => Err(McpError::method_not_found(other)),
    }
}

// ---------------------------------------------------------------------------
// individual tool stubs
// ---------------------------------------------------------------------------

fn require_string<'a>(
    obj: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Result<&'a str, McpError> {
    obj.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params(format!("missing or non-string `{key}`")))
}

fn optional_u64(
    obj: &serde_json::Map<String, Value>,
    key: &str,
    default: u64,
) -> Result<u64, McpError> {
    match obj.get(key) {
        None => Ok(default),
        Some(v) => v.as_u64().ok_or_else(|| {
            McpError::invalid_params(format!("`{key}` must be a non-negative integer"))
        }),
    }
}

fn model_search(obj: &serde_json::Map<String, Value>) -> Result<Value, McpError> {
    let query = require_string(obj, "query")?;
    let limit = optional_u64(obj, "limit", 10)?;
    Ok(json!({
        "query": query,
        "limit": limit,
        "results": [
            {"source": "hf", "id": "example-org/example-llm-7b", "score": 0.92},
            {"source": "hf", "id": "example-org/example-llm-13b", "score": 0.81}
        ],
        "total": 2,
        "stub": true
    }))
}

fn model_detail(obj: &serde_json::Map<String, Value>) -> Result<Value, McpError> {
    let source = require_string(obj, "source")?;
    let id = require_string(obj, "id")?;
    Ok(json!({
        "source": source,
        "id": id,
        "name": id,
        "kind": "base",
        "license": "stub",
        "parameters_b": 7,
        "stub": true
    }))
}

fn model_rag_ask(obj: &serde_json::Map<String, Value>) -> Result<Value, McpError> {
    let question = require_string(obj, "question")?;
    let top_k = optional_u64(obj, "top_k", 5)?;
    Ok(json!({
        "question": question,
        "top_k": top_k,
        "answer": "[stub] RAG answer will be generated in a later phase.",
        "chunks": [],
        "stub": true
    }))
}

fn model_quants(obj: &serde_json::Map<String, Value>) -> Result<Value, McpError> {
    let source = require_string(obj, "source")?;
    let id = require_string(obj, "id")?;
    Ok(json!({
        "source": source,
        "id": id,
        "quants": [
            {"format": "gguf", "bits": 4, "uri": "stub://example/q4_k_m.gguf"},
            {"format": "gptq", "bits": 4, "uri": "stub://example/gptq-4bit.safetensors"}
        ],
        "stub": true
    }))
}

fn similar_models(obj: &serde_json::Map<String, Value>) -> Result<Value, McpError> {
    let source = require_string(obj, "source")?;
    let id = require_string(obj, "id")?;
    let limit = optional_u64(obj, "limit", 10)?;
    Ok(json!({
        "seed": {"source": source, "id": id},
        "limit": limit,
        "similar": [
            {"source": "hf", "id": "example-org/example-llm-7b-instruct", "score": 0.95},
            {"source": "hf", "id": "example-org/example-llm-7b-chat",    "score": 0.88}
        ],
        "stub": true
    }))
}

fn models_for_use_case(obj: &serde_json::Map<String, Value>) -> Result<Value, McpError> {
    let use_case = require_string(obj, "use_case")?;
    let limit = optional_u64(obj, "limit", 10)?;
    Ok(json!({
        "use_case": use_case,
        "limit": limit,
        "models": [
            {"source": "hf", "id": "example-org/example-llm-7b", "fit": 0.90},
            {"source": "hf", "id": "example-org/example-llm-13b", "fit": 0.84}
        ],
        "stub": true
    }))
}
