//! Health check history and time-series tracking
//!
//! Provides historical tracking of health check results for trend analysis
//! and alerting based on status changes over time.
//!
//! # Examples
//!
//! ## Basic History Tracking
//!
//! ```rust,ignore
//! use phenotype_health::history::{HealthHistory, HistoryEntry};
//! use phenotype_health::HealthStatus;
//! use chrono::Utc;
//!
//! let mut history = HealthHistory::new(100);
//!
//! // Record a healthy check
//! history.add(HistoryEntry {
//!     timestamp: Utc::now(),
//!     status: HealthStatus::Healthy,
//!     latency_ms: Some(50),
//!     error: None,
//! });
//!
//! // Calculate uptime over the last hour
//! use chrono::Duration;
//! let uptime = history.uptime(Duration::hours(1));
//! println!("Service uptime: {:.1}%", uptime);
//! ```
//!
//! ## Persistent Storage
//!
//! ```rust,ignore
//! use phenotype_health::history::persistence::HistoryStorage;
//!
//! let storage = HistoryStorage::new("/path/to/history.json");
//! storage.save_history("database", &history).unwrap();
//!
//! let loaded = storage.load_history("database").unwrap();
//! ```
//!
//! ## Trend Analysis
//!
//! ```rust,ignore
//! use phenotype_health::history::TrendAnalyzer;
//!
//! let analysis = TrendAnalyzer::analyze(&history);
//! println!("Trend: {}", analysis);
//! ```

use crate::HealthStatus;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Historical entry for a health check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub timestamp: DateTime<Utc>,
    pub status: HealthStatus,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

/// Ring buffer for storing health check history
#[derive(Debug, Clone)]
pub struct HealthHistory {
    max_size: usize,
    entries: VecDeque<HistoryEntry>,
}

impl HealthHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            entries: VecDeque::with_capacity(max_size),
        }
    }

    pub fn add(&mut self, entry: HistoryEntry) {
        if self.entries.len() >= self.max_size {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    pub fn entries(&self) -> &VecDeque<HistoryEntry> {
        &self.entries
    }

    pub fn uptime(&self, duration: Duration) -> f64 {
        let recent: Vec<_> = self
            .entries
            .iter()
            .filter(|e| e.timestamp > Utc::now() - duration)
            .collect();
        if recent.is_empty() {
            return 0.0;
        }
        let healthy_count = recent
            .iter()
            .filter(|e| e.status == HealthStatus::Healthy)
            .count();
        (healthy_count as f64 / recent.len() as f64) * 100.0
    }
}

/// Health trend analyzer
#[derive(Debug, Default)]
pub struct TrendAnalyzer;

impl TrendAnalyzer {
    pub fn analyze(_history: &HealthHistory) -> String {
        "Trend analysis placeholder".to_string()
    }
}

/// Persistent health history storage
pub mod persistence {
    use super::HistoryEntry;
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    /// Persistent storage for health history
    #[derive(Debug, Serialize, Deserialize)]
    pub struct PersistentHistory {
        /// Storage version for migrations
        version: u32,
        /// When the history was last updated
        last_updated: DateTime<Utc>,
        /// History entries by check name
        entries: HashMap<String, Vec<HistoryEntry>>,
    }

    impl Default for PersistentHistory {
        fn default() -> Self {
            Self {
                version: 1,
                last_updated: Utc::now(),
                entries: HashMap::new(),
            }
        }
    }

    /// History storage manager
    pub struct HistoryStorage {
        storage_path: PathBuf,
        max_entries_per_check: usize,
    }

    impl HistoryStorage {
        /// Create new storage manager
        pub fn new(storage_path: impl AsRef<Path>) -> Self {
            Self {
                storage_path: storage_path.as_ref().to_path_buf(),
                max_entries_per_check: 1000,
            }
        }

        /// Set max entries per check (default: 1000)
        pub fn with_max_entries(mut self, max: usize) -> Self {
            self.max_entries_per_check = max;
            self
        }

        /// Load persistent history from disk
        pub fn load(&self) -> anyhow::Result<PersistentHistory> {
            if !self.storage_path.exists() {
                return Ok(PersistentHistory::default());
            }

            let content = std::fs::read_to_string(&self.storage_path)?;
            let history: PersistentHistory = serde_json::from_str(&content)?;
            Ok(history)
        }

        /// Save history to disk
        pub fn save(&self, history: &PersistentHistory) -> anyhow::Result<()> {
            // Ensure parent directory exists
            if let Some(parent) = self.storage_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let content = serde_json::to_string_pretty(history)?;
            std::fs::write(&self.storage_path, content)?;
            Ok(())
        }

        /// Add entry for a specific check
        pub fn add_entry(&self, check_name: &str, entry: HistoryEntry) -> anyhow::Result<()> {
            let mut history = self.load()?;

            let entries = history.entries.entry(check_name.to_string()).or_default();
            entries.push(entry);

            // Trim old entries
            if entries.len() > self.max_entries_per_check {
                let to_remove = entries.len() - self.max_entries_per_check;
                entries.drain(0..to_remove);
            }

            history.last_updated = Utc::now();
            self.save(&history)?;
            Ok(())
        }

        /// Get entries for a specific check
        pub fn get_entries(&self, check_name: &str) -> anyhow::Result<Vec<HistoryEntry>> {
            let history = self.load()?;
            Ok(history.entries.get(check_name).cloned().unwrap_or_default())
        }

        /// Get all check names
        pub fn get_check_names(&self) -> anyhow::Result<Vec<String>> {
            let history = self.load()?;
            Ok(history.entries.keys().cloned().collect())
        }

        /// Clear all history
        pub fn clear(&self) -> anyhow::Result<()> {
            if self.storage_path.exists() {
                std::fs::remove_file(&self.storage_path)?;
            }
            Ok(())
        }

        /// Get storage statistics
        pub fn stats(&self) -> anyhow::Result<HistoryStats> {
            let history = self.load()?;
            let total_checks = history.entries.len();
            let total_entries: usize = history.entries.values().map(|v| v.len()).sum();

            Ok(HistoryStats {
                total_checks,
                total_entries,
                last_updated: history.last_updated,
            })
        }
    }

    /// Statistics about stored history
    #[derive(Debug, Clone)]
    pub struct HistoryStats {
        /// Number of unique checks
        pub total_checks: usize,
        /// Total number of entries
        pub total_entries: usize,
        /// When history was last updated
        pub last_updated: DateTime<Utc>,
    }

    impl HistoryStats {
        /// Average entries per check
        pub fn avg_entries_per_check(&self) -> f64 {
            if self.total_checks == 0 {
                return 0.0;
            }
            self.total_entries as f64 / self.total_checks as f64
        }
    }

    /// Default storage path
    pub fn default_storage_path() -> PathBuf {
        dirs::cache_dir()
            .unwrap_or_else(|| std::env::temp_dir())
            .join("phenotype-health")
            .join("history.json")
    }

    /// Create storage with default path
    pub fn default_storage() -> HistoryStorage {
        HistoryStorage::new(default_storage_path())
    }
}
