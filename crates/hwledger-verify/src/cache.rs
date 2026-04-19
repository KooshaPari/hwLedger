//! Disk-based cache for verification results keyed on screenshot + model + prompt version.

use crate::VerifyError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

/// Disk-based cache for verification results.
pub struct Cache {
    cache_dir: PathBuf,
}

impl Cache {
    /// Create a new cache at `./target/hwledger-verify-cache/`.
    pub fn new() -> Result<Self, VerifyError> {
        let cache_dir = PathBuf::from("target/hwledger-verify-cache");
        fs::create_dir_all(&cache_dir)
            .map_err(|e| VerifyError::CacheError(format!("Failed to create cache dir: {}", e)))?;

        Ok(Self { cache_dir })
    }

    /// Generate a cache key for a describe call.
    pub fn key_for_screenshot(&self, screenshot_bytes: &[u8], model: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(screenshot_bytes);
        hasher.update(model.as_bytes());
        hasher.update(b"describe-v1");

        let hash = format!("{:x}", hasher.finalize());
        format!("describe-{}", hash)
    }

    /// Generate a cache key for a judge call.
    pub fn key_for_judge(&self, intent: &str, description: &str, model: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(intent.as_bytes());
        hasher.update(description.as_bytes());
        hasher.update(model.as_bytes());
        hasher.update(b"judge-v1");

        let hash = format!("{:x}", hasher.finalize());
        format!("judge-{}", hash)
    }

    /// Get a cached Description.
    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<T, VerifyError> {
        let path = self.cache_dir.join(format!("{}.json", key));
        if !path.exists() {
            return Err(VerifyError::CacheError("Cache miss".to_string()));
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| VerifyError::CacheError(format!("Failed to read cache: {}", e)))?;

        serde_json::from_str(&content)
            .map_err(|e| VerifyError::CacheError(format!("Failed to deserialize cache: {}", e)))
    }

    /// Store a value in the cache.
    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<(), VerifyError> {
        let path = self.cache_dir.join(format!("{}.json", key));
        let json = serde_json::to_string(value)
            .map_err(|e| VerifyError::CacheError(format!("Failed to serialize: {}", e)))?;

        fs::write(&path, json)
            .map_err(|e| VerifyError::CacheError(format!("Failed to write cache: {}", e)))?;

        Ok(())
    }

    /// Clear the entire cache.
    pub fn clear(&self) -> Result<(), VerifyError> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)
                .map_err(|e| VerifyError::CacheError(format!("Failed to clear cache: {}", e)))?;
        }
        fs::create_dir_all(&self.cache_dir)
            .map_err(|e| VerifyError::CacheError(format!("Failed to recreate cache dir: {}", e)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Description;

    // Traces to: FR-UX-VERIFY-001
    #[test]
    fn test_cache_key_for_screenshot() {
        let cache = Cache::new().unwrap();
        let png_data = b"test-png-data";
        let key1 = cache.key_for_screenshot(png_data, "claude-opus-4-7");
        let key2 = cache.key_for_screenshot(png_data, "claude-opus-4-7");

        assert_eq!(key1, key2);
        assert!(key1.starts_with("describe-"));
    }

    // Traces to: FR-UX-VERIFY-002
    #[test]
    fn test_cache_key_for_judge() {
        let cache = Cache::new().unwrap();
        let key1 = cache.key_for_judge("intent1", "description1", "claude-sonnet-4-6");
        let key2 = cache.key_for_judge("intent1", "description1", "claude-sonnet-4-6");
        let key3 = cache.key_for_judge("intent2", "description1", "claude-sonnet-4-6");

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
        assert!(key1.starts_with("judge-"));
    }

    // Traces to: FR-UX-VERIFY-001
    #[test]
    fn test_cache_set_and_get() {
        let cache = Cache::new().unwrap();
        cache.clear().ok(); // Start fresh

        let desc = Description { text: "test".to_string(), structured: None, tokens_used: 10 };

        let key = "test-key";
        cache.set(key, &desc).unwrap();
        let retrieved: Description = cache.get(key).unwrap();

        assert_eq!(retrieved.text, "test");
        assert_eq!(retrieved.tokens_used, 10);
    }
}
