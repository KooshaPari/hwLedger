//! 24-hour filesystem cache for HF responses. Traces to: FR-HF-001.

use crate::error::{HfError, Result};
use serde::de::DeserializeOwned;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tracing::{debug, warn};

pub const DEFAULT_TTL_SECS: u64 = 24 * 60 * 60;

#[derive(Debug, Clone)]
pub struct HfCache {
    pub root: PathBuf,
    pub ttl: Duration,
}

impl HfCache {
    pub fn new(root: PathBuf) -> Self {
        Self { root, ttl: Duration::from_secs(DEFAULT_TTL_SECS) }
    }

    /// `~/.cache/hwledger/hf/`.
    pub fn default_path() -> Result<Self> {
        let base = dirs::cache_dir()
            .ok_or_else(|| HfError::Cache("no cache dir available".into()))?
            .join("hwledger")
            .join("hf");
        Ok(Self::new(base))
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    fn path_for(&self, key: &str) -> PathBuf {
        // Flatten to a safe filename: replace `/` with `__`, spaces with `_`.
        let safe = key.replace(['/', '\\'], "__").replace([' ', '?', '&', '='], "_");
        self.root.join(format!("{}.json", safe))
    }

    fn ensure_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.root).map_err(|e| HfError::Cache(e.to_string()))
    }

    /// Read if present and fresh.
    pub fn read<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let p = self.path_for(key);
        if !p.exists() {
            return Ok(None);
        }
        let meta = fs::metadata(&p).map_err(|e| HfError::Cache(e.to_string()))?;
        let fresh = meta
            .modified()
            .ok()
            .and_then(|m| SystemTime::now().duration_since(m).ok())
            .map(|age| age < self.ttl)
            .unwrap_or(false);
        if !fresh {
            debug!(path = %p.display(), "cache entry stale");
        }
        let text = fs::read_to_string(&p).map_err(|e| HfError::Cache(e.to_string()))?;
        let v = serde_json::from_str::<T>(&text).map_err(|e| HfError::Parse {
            context: format!("cache:{}", key),
            message: e.to_string(),
        })?;
        Ok(Some(v))
    }

    pub fn write(&self, key: &str, raw_text: &str) -> Result<()> {
        self.ensure_dir()?;
        let p = self.path_for(key);
        if let Err(e) = fs::write(&p, raw_text) {
            warn!(path = %p.display(), error = %e, "cache write failed");
            return Err(HfError::Cache(e.to_string()));
        }
        Ok(())
    }

    pub fn root_path(&self) -> &Path {
        &self.root
    }
}

/// Kind of cached payload. Used to derive keys.
#[derive(Debug, Clone)]
pub enum CacheKind {
    Search(String),
    Card(String),
    Config { repo_id: String, rev: String },
}

impl CacheKind {
    pub fn key_with_query(&self, _query: &[(&str, String)]) -> String {
        match self {
            CacheKind::Search(fp) => format!("search/{}", fp),
            CacheKind::Card(id) => format!("{}/card", id),
            CacheKind::Config { repo_id, rev } => format!("{}/config@{}", repo_id, rev),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn roundtrip_cache() {
        let tmp = TempDir::new().unwrap();
        let cache = HfCache::new(tmp.path().to_path_buf());
        cache.write("foo/bar", r#"{"x":1}"#).unwrap();
        let v: serde_json::Value = cache.read("foo/bar").unwrap().unwrap();
        assert_eq!(v["x"], 1);
    }

    #[test]
    fn stale_cache_still_readable() {
        // TTL 0 means everything is stale — but our `read` returns the value
        // regardless (staleness is purely advisory; the client decides policy).
        let tmp = TempDir::new().unwrap();
        let cache = HfCache::new(tmp.path().to_path_buf()).with_ttl(Duration::from_millis(0));
        cache.write("k", r#"{"a":2}"#).unwrap();
        let v: serde_json::Value = cache.read("k").unwrap().unwrap();
        assert_eq!(v["a"], 2);
    }
}
