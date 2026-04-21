//! Types: [`SearchQuery`], [`ModelCard`], [`ModelCardDetail`]. Traces to: FR-HF-001.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Sort key for model search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortKey {
    Downloads,
    Likes,
    Recent,
    Trending,
}

impl SortKey {
    pub fn as_hf_str(&self) -> &'static str {
        match self {
            // HF's `sort` param accepts: downloads, likes, lastModified, trendingScore
            SortKey::Downloads => "downloads",
            SortKey::Likes => "likes",
            SortKey::Recent => "lastModified",
            SortKey::Trending => "trendingScore",
        }
    }
}

impl std::str::FromStr for SortKey {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "downloads" | "download" => Ok(SortKey::Downloads),
            "likes" | "like" => Ok(SortKey::Likes),
            "recent" | "lastmodified" | "last_modified" => Ok(SortKey::Recent),
            "trending" | "trendingscore" => Ok(SortKey::Trending),
            other => {
                Err(format!("unknown sort key `{}` (use downloads|likes|recent|trending)", other))
            }
        }
    }
}

/// Search filters accepted by [`crate::HfClient::search_models`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub text: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub library: Option<String>,
    pub sort: SortKey,
    pub limit: u32,
    pub min_downloads: Option<u64>,
    pub author: Option<String>,
    pub pipeline_tag: Option<String>,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            text: None,
            tags: Vec::new(),
            library: None,
            sort: SortKey::Downloads,
            limit: 20,
            min_downloads: None,
            author: None,
            pipeline_tag: None,
        }
    }
}

impl SearchQuery {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: Some(text.into()), ..Self::default() }
    }

    /// Pairs for reqwest `.query()`. Applies the server-side filters we can;
    /// client-side filters (e.g. min_downloads) are applied post-fetch.
    pub fn to_query_pairs(&self) -> Vec<(&'static str, String)> {
        let mut out: Vec<(&'static str, String)> = Vec::new();
        if let Some(s) = &self.text {
            out.push(("search", s.clone()));
        }
        for tag in &self.tags {
            out.push(("filter", tag.clone()));
        }
        if let Some(lib) = &self.library {
            out.push(("library", lib.clone()));
        }
        if let Some(author) = &self.author {
            out.push(("author", author.clone()));
        }
        if let Some(pt) = &self.pipeline_tag {
            out.push(("pipeline_tag", pt.clone()));
        }
        out.push(("sort", self.sort.as_hf_str().to_string()));
        out.push(("direction", "-1".to_string()));
        // HF caps at 100; we cap our own side too.
        let limit = self.limit.clamp(1, 100);
        out.push(("limit", limit.to_string()));
        // Ask for the fields we actually consume.
        out.push(("full", "true".to_string()));
        out
    }

    /// Stable fingerprint used as a cache key.
    pub fn cache_fingerprint(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(t) = &self.text {
            parts.push(format!("q={}", t));
        }
        if !self.tags.is_empty() {
            parts.push(format!("tags={}", self.tags.join(",")));
        }
        if let Some(l) = &self.library {
            parts.push(format!("lib={}", l));
        }
        if let Some(a) = &self.author {
            parts.push(format!("author={}", a));
        }
        if let Some(p) = &self.pipeline_tag {
            parts.push(format!("pt={}", p));
        }
        parts.push(format!("sort={}", self.sort.as_hf_str()));
        parts.push(format!("limit={}", self.limit));
        parts.join("&")
    }
}

/// Model summary returned by `/api/models` (post-normalisation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCard {
    pub id: String,
    pub downloads: u64,
    pub likes: u64,
    #[serde(default)]
    pub tags: Vec<String>,
    pub library_name: Option<String>,
    pub pipeline_tag: Option<String>,
    pub last_modified: DateTime<Utc>,
    /// Best-effort param estimate parsed from `model-index` / tags (e.g. "7B", "70b").
    pub params_estimate: Option<u64>,
}

/// Full model card with siblings + card data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCardDetail {
    #[serde(flatten)]
    pub card: ModelCard,
    pub author: Option<String>,
    pub sha: Option<String>,
    pub private: bool,
    pub gated: bool,
    #[serde(default)]
    pub siblings: Vec<SiblingFile>,
    pub card_data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiblingFile {
    pub rfilename: String,
}

// ---------------------------------------------------------------------------
// Raw shapes as returned by HF. We keep these separate so the public API can
// stay stable even when HF adds/removes fields.

#[derive(Debug, Deserialize)]
pub(crate) struct RawModelCard {
    // HF returns both `id` and (sometimes) `modelId` with the same value. Serde
    // rejects duplicates when we alias, so accept either by declaring both and
    // coalescing in the From impl.
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default, rename = "modelId")]
    pub model_id: Option<String>,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub likes: u64,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, rename = "library_name")]
    pub library_name: Option<String>,
    #[serde(default, rename = "pipeline_tag")]
    pub pipeline_tag: Option<String>,
    #[serde(default, rename = "lastModified", alias = "last_modified")]
    pub last_modified: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawModelCardDetail {
    #[serde(flatten)]
    pub base: RawModelCard,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub sha: Option<String>,
    #[serde(default)]
    pub private: bool,
    #[serde(default)]
    pub gated: serde_json::Value,
    #[serde(default)]
    pub siblings: Vec<SiblingFile>,
    #[serde(default, rename = "cardData")]
    pub card_data: Option<serde_json::Value>,
}

impl From<RawModelCard> for ModelCard {
    fn from(r: RawModelCard) -> Self {
        let id = r.id.clone().or(r.model_id.clone()).unwrap_or_default();
        let params = estimate_params_from_tags(&r.tags, &id);
        ModelCard {
            id,
            downloads: r.downloads,
            likes: r.likes,
            library_name: r.library_name,
            pipeline_tag: r.pipeline_tag,
            last_modified: r.last_modified.unwrap_or_else(Utc::now),
            tags: r.tags,
            params_estimate: params,
        }
    }
}

impl From<RawModelCardDetail> for ModelCardDetail {
    fn from(r: RawModelCardDetail) -> Self {
        let gated = match &r.gated {
            serde_json::Value::Bool(b) => *b,
            serde_json::Value::String(s) => !s.eq_ignore_ascii_case("false"),
            _ => false,
        };
        ModelCardDetail {
            card: ModelCard::from(r.base),
            author: r.author,
            sha: r.sha,
            private: r.private,
            gated,
            siblings: r.siblings,
            card_data: r.card_data,
        }
    }
}

/// Parse "7B", "70b", "8x7B", etc. out of tags + id.
fn estimate_params_from_tags(tags: &[String], id: &str) -> Option<u64> {
    static RE: once_cell_regex::Lazy = once_cell_regex::Lazy::new();
    let re = RE.get();

    let mut best: Option<u64> = None;
    let mut try_str = |s: &str| {
        for cap in re.captures_iter(s) {
            if let Some(n_str) = cap.get(1) {
                if let Ok(n) = n_str.as_str().parse::<f64>() {
                    let unit =
                        cap.get(2).map(|m| m.as_str().to_ascii_uppercase()).unwrap_or_default();
                    let mult = match unit.as_str() {
                        "B" => 1_000_000_000.0,
                        "M" => 1_000_000.0,
                        _ => continue,
                    };
                    let v = (n * mult) as u64;
                    best = Some(best.map_or(v, |b| b.max(v)));
                }
            }
        }
    };
    for t in tags {
        try_str(t);
    }
    try_str(id);
    best
}

// Minimal local regex cache to avoid pulling `once_cell` into the public API.
mod once_cell_regex {
    use regex::Regex;
    use std::sync::OnceLock;

    pub struct Lazy(OnceLock<Regex>);

    impl Lazy {
        pub const fn new() -> Self {
            Self(OnceLock::new())
        }
        pub fn get(&self) -> &Regex {
            self.0.get_or_init(|| {
                // Matches things like "7B", "70B", "8x7B" (captures last number), "1.5B", "125M".
                Regex::new(r"(?i)(\d+(?:\.\d+)?)\s*([BM])\b").expect("params regex")
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sort_key() {
        use std::str::FromStr;
        assert_eq!(SortKey::from_str("downloads").unwrap(), SortKey::Downloads);
        assert_eq!(SortKey::from_str("trending").unwrap(), SortKey::Trending);
        assert!(SortKey::from_str("nope").is_err());
    }

    #[test]
    fn query_pairs_basic() {
        let q = SearchQuery {
            text: Some("llama".into()),
            tags: vec!["text-generation".into()],
            library: Some("gguf".into()),
            sort: SortKey::Downloads,
            limit: 5,
            ..Default::default()
        };
        let pairs = q.to_query_pairs();
        let map: std::collections::HashMap<_, _> =
            pairs.iter().map(|(k, v)| (*k, v.clone())).collect();
        assert_eq!(map.get("search").unwrap(), "llama");
        assert_eq!(map.get("library").unwrap(), "gguf");
        assert_eq!(map.get("limit").unwrap(), "5");
        assert_eq!(map.get("sort").unwrap(), "downloads");
    }

    #[test]
    fn params_from_tags() {
        let n =
            estimate_params_from_tags(&["llama".into(), "7B".into()], "meta-llama/Llama-2-7b-hf");
        assert_eq!(n, Some(7_000_000_000));
        let n2 = estimate_params_from_tags(&[], "mistralai/Mixtral-8x7B-v0.1");
        assert_eq!(n2, Some(7_000_000_000));
        let n3 = estimate_params_from_tags(&["125M".into()], "gpt2");
        assert_eq!(n3, Some(125_000_000));
    }
}
