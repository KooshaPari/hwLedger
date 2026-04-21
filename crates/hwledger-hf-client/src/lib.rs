//! Hugging Face Hub search + metadata client.
//!
//! Traces to: FR-HF-001
//!
//! Thin, focused async HTTP client over `https://huggingface.co/api/...`.
//!
//! # Anonymous vs authenticated
//!
//! Anonymous access is the DEFAULT. Public model search + `config.json` fetches
//! work without any credential (subject to HF's ~1000 req / 5 min IP-scoped
//! rate limit). A `HF_TOKEN` env var or explicit `token` parameter unlocks
//! higher rate limits (~100k req/day) and gated/private repos.
//!
//! # Caching
//!
//! Responses are cached under `~/.cache/hwledger/hf/<repo-id>/*.json` with a
//! 24 h TTL. On network failure, cached data is used transparently with a
//! tracing warning. `offline: true` forces cache-only.

pub mod cache;
pub mod error;
pub mod model;

pub use cache::{CacheKind, HfCache};
pub use error::{HfError, Result};
pub use model::{ModelCard, ModelCardDetail, SearchQuery, SiblingFile, SortKey};

use reqwest::{header, Client, StatusCode};
use std::time::Duration;
use tracing::{debug, warn};

/// Default HF Hub API base.
pub const HF_API_BASE: &str = "https://huggingface.co";

/// Hugging Face Hub client.
///
/// Construct with [`HfClient::new`] (anonymous) or [`HfClient::with_token`].
#[derive(Debug, Clone)]
pub struct HfClient {
    http: Client,
    token: Option<String>,
    base: String,
    cache: Option<HfCache>,
    offline: bool,
}

impl HfClient {
    /// New client. Pass `None` for anonymous access (recommended for public models).
    /// Set `token` or rely on the `HF_TOKEN` env var for higher rate limits + gated repos.
    pub fn new(token: Option<String>) -> Self {
        let token = token.or_else(|| std::env::var("HF_TOKEN").ok());
        let http = Client::builder()
            .user_agent(concat!("hwledger-hf-client/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(30))
            .build()
            .expect("reqwest client should build");
        Self {
            http,
            token,
            base: HF_API_BASE.to_string(),
            cache: HfCache::default_path().ok(),
            offline: false,
        }
    }

    /// Convenience constructor with explicit token.
    pub fn with_token(token: impl Into<String>) -> Self {
        Self::new(Some(token.into()))
    }

    /// Override the API base (used by `wiremock` tests).
    pub fn with_base(mut self, base: impl Into<String>) -> Self {
        self.base = base.into();
        self
    }

    /// Disable caching entirely.
    pub fn without_cache(mut self) -> Self {
        self.cache = None;
        self
    }

    /// Override the cache path.
    pub fn with_cache(mut self, cache: HfCache) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Force offline mode: cache-only. Network calls are skipped.
    pub fn offline(mut self, offline: bool) -> Self {
        self.offline = offline;
        self
    }

    /// True if a token is configured (via constructor or `HF_TOKEN`).
    pub fn has_token(&self) -> bool {
        self.token.is_some()
    }

    fn apply_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(t) = &self.token {
            req.header(header::AUTHORIZATION, format!("Bearer {}", t))
        } else {
            req
        }
    }

    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
        cache_kind: CacheKind,
    ) -> Result<T> {
        let cache_key = cache_kind.key_with_query(query);

        if self.offline {
            debug!("offline mode: serving {} from cache", cache_key);
            return self
                .cache
                .as_ref()
                .and_then(|c| c.read::<T>(&cache_key).ok().flatten())
                .ok_or(HfError::OfflineCacheMiss(cache_key));
        }

        let url = format!("{}{}", self.base, path);
        debug!(%url, "hf GET");
        let req = self.http.get(&url).query(query);
        let req = self.apply_auth(req);

        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                warn!(%url, error = %e, "network error; trying cache");
                if let Some(c) = &self.cache {
                    if let Ok(Some(v)) = c.read::<T>(&cache_key) {
                        return Ok(v);
                    }
                }
                return Err(HfError::Network(e.to_string()));
            }
        };

        match resp.status() {
            StatusCode::OK => {
                let text = resp.text().await.map_err(|e| HfError::Network(e.to_string()))?;
                let parsed: T = serde_json::from_str(&text).map_err(|e| HfError::Parse {
                    context: path.to_string(),
                    message: e.to_string(),
                })?;
                if let Some(c) = &self.cache {
                    let _ = c.write(&cache_key, &text);
                }
                Ok(parsed)
            }
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                Err(HfError::AuthRequired { path: path.to_string(), has_token: self.has_token() })
            }
            StatusCode::NOT_FOUND => Err(HfError::NotFound(path.to_string())),
            StatusCode::TOO_MANY_REQUESTS => {
                let retry_after = resp
                    .headers()
                    .get(header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u32>().ok());
                Err(HfError::RateLimited {
                    retry_after_secs: retry_after,
                    has_token: self.has_token(),
                })
            }
            s => Err(HfError::Http { status: s.as_u16(), path: path.to_string() }),
        }
    }

    /// Search public models. Anonymous-friendly.
    pub async fn search_models(&self, q: &SearchQuery) -> Result<Vec<ModelCard>> {
        let query = q.to_query_pairs();
        let raw: Vec<model::RawModelCard> =
            self.get_json("/api/models", &query, CacheKind::Search(q.cache_fingerprint())).await?;
        Ok(raw.into_iter().map(ModelCard::from).collect())
    }

    /// Fetch full card for a single repo.
    pub async fn get_model(&self, repo_id: &str) -> Result<ModelCardDetail> {
        let path = format!("/api/models/{}", repo_id);
        let raw: model::RawModelCardDetail =
            self.get_json(&path, &[], CacheKind::Card(repo_id.to_string())).await?;
        Ok(ModelCardDetail::from(raw))
    }

    /// Fetch `config.json` for a model (raw JSON value).
    pub async fn fetch_config(
        &self,
        repo_id: &str,
        revision: Option<&str>,
    ) -> Result<serde_json::Value> {
        let rev = revision.unwrap_or("main");
        let path = format!("/{}/resolve/{}/config.json", repo_id, rev);
        self.get_json(
            &path,
            &[],
            CacheKind::Config { repo_id: repo_id.to_string(), rev: rev.to_string() },
        )
        .await
    }
}
