//! `POST /v1/search` — BM25 hybrid search with optional facets.
//!
//! Accepts a [`SearchRequest`] body (see [`crate::service::SearchRequest`])
//! and returns `{ "query": ..., "limit": ..., "results": [FusedResult, ...] }`.

use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};

use crate::service::{self, SearchRequest, ServiceError};
use crate::AppState;

/// Mount the `search` routes.
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/v1/search", post(search))
}

/// `POST /v1/search` — run a hybrid search.
async fn search(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SearchRequest>,
) -> Result<impl IntoResponse, ServiceError> {
    let q = req.to_query();
    let results = service::run_hybrid(&state.index, &q).await?;
    Ok(Json(serde_json::json!({
        "query": q.text,
        "limit": q.limit,
        "facets": q.facets,
        "results": results,
    })))
}