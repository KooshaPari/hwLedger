//! Phenotype Health Library
//!
//! A standardized health check framework for async services.
//! Provides a unified way to define, check, and aggregate health status.
//!
//! # Features
//!
//! - `http` (default): HTTP endpoint integration via axum
//! - `prometheus`: Prometheus metrics export
//! - `background`: Background check scheduling
//!
//! # Modules
//!
//! - [`history`](crate::history): Time-series tracking of health check results
//! - [`composite`](crate::composite): Health checks with dependency support
//! - [`background`](crate::background): Scheduled background health checks
//!
//! # Usage
//!
//! ```rust,ignore
//! use phenotype_health::{HealthCheck, HealthStatus, HealthRegistry};
//! use async_trait::async_trait;
//!
//! // Define a health check
//! #[derive(Debug)]
//! struct DatabaseCheck;
//!
//! #[async_trait]
//! impl HealthCheck for DatabaseCheck {
//!     fn name(&self) -> &str {
//!         "database"
//!     }
//!
//!     async fn check(&self) -> Result<HealthStatus, phenotype_health::HealthCheckError> {
//!         Ok(HealthStatus::Healthy)
//!     }
//! }
//!
//! // Register and run checks
//! let registry = HealthRegistry::new();
//! registry.register(DatabaseCheck).await;
//! let report = registry.check_all().await;
//! ```
//!
//! # Example: Health History
//!
//! ```rust,ignore
//! use phenotype_health::history::{HealthHistory, HistoryEntry};
//! use chrono::Utc;
//!
//! let mut history = HealthHistory::new(100);
//! history.add(HistoryEntry {
//!     timestamp: Utc::now(),
//!     status: HealthStatus::Healthy,
//!     latency_ms: Some(50),
//!     error: None,
//! });
//!
//! let uptime = history.uptime(Duration::hours(1));
//! println!("Uptime: {:.1}%", uptime);
//! ```
//!
//! # Example: Composite Checks
//!
//! ```rust,ignore
//! use phenotype_health::composite::{CompositeHealthCheck, CompositeRegistry};
//!
//! let check = CompositeHealthCheck::new("api", ApiCheck)
//!     .depends_on("database")
//!     .depends_on("cache");
//!
//! let mut registry = CompositeRegistry::new();
//! registry.register(check);
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::debug;

/// Health check history and time-series tracking
pub mod history;

/// Composite health checks with dependencies
pub mod composite;

/// Background health check scheduling
pub mod background;

#[cfg(feature = "http")]
pub mod http;

#[cfg(feature = "prometheus")]
pub mod metrics;

/// Health status enum - standardized across all services
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// Service is fully operational
    Healthy,
    /// Service is degraded but operational
    Degraded,
    /// Service is not operational
    Unhealthy,
}

impl Default for HealthStatus {
    fn default() -> Self {
        HealthStatus::Healthy
    }
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
        }
    }
}

/// Health check error types
#[derive(Debug, Error)]
pub enum HealthCheckError {
    #[error("check failed: {0}")]
    CheckFailed(String),
    #[error("timeout waiting for check")]
    Timeout,
    #[error("internal error: {0}")]
    Internal(String),
}

/// Trait for implementing health checks
#[async_trait]
pub trait HealthCheck: Send + Sync + 'static {
    /// The name of this health check
    fn name(&self) -> &str;

    /// Perform the health check
    async fn check(&self) -> Result<HealthStatus, HealthCheckError>;
}

/// A component health check with a simple status
#[derive(Debug)]
pub struct ComponentHealthCheck {
    name: String,
    status: HealthStatus,
}

impl ComponentHealthCheck {
    pub fn new(name: impl Into<String>, status: HealthStatus) -> Self {
        Self {
            name: name.into(),
            status,
        }
    }
}

#[async_trait]
impl HealthCheck for ComponentHealthCheck {
    fn name(&self) -> &str {
        &self.name
    }

    async fn check(&self) -> Result<HealthStatus, HealthCheckError> {
        Ok(self.status)
    }
}

/// Check at a specific point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSnapshot {
    /// Name of the component checked
    pub component: String,
    /// Status at check time
    pub status: HealthStatus,
    /// When the check was performed
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Optional latency information (in milliseconds)
    pub latency_ms: Option<u64>,
    /// Optional error message if check failed
    pub error: Option<String>,
}

/// Individual check result with metadata
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// Component name
    pub component: String,
    /// Check status
    pub status: HealthStatus,
    /// How long the check took
    pub duration_ms: u64,
    /// Any error message
    pub message: Option<String>,
}

/// Aggregated health report
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HealthReport {
    /// Overall status (worst of all checks)
    pub overall_status: HealthStatus,
    /// Individual check results
    pub checks: Vec<HealthSnapshot>,
    /// Summary statistics
    pub summary: ReportSummary,
}

/// Report summary statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReportSummary {
    /// Total number of checks
    pub total: usize,
    /// Number of healthy checks
    pub healthy: usize,
    /// Number of degraded checks
    pub degraded: usize,
    /// Number of unhealthy checks
    pub unhealthy: usize,
}

/// Registry for managing multiple health checks
#[derive(Default)]
pub struct HealthRegistry {
    checks: Vec<Arc<dyn HealthCheck>>,
}

impl std::fmt::Debug for HealthRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HealthRegistry")
            .field("checks", &self.checks.len())
            .finish()
    }
}

impl HealthRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    /// Register a health check
    pub fn register(&mut self, check: impl HealthCheck) {
        debug!("Registering health check: {}", check.name());
        self.checks.push(Arc::new(check));
    }

    /// Run all health checks and return a report
    pub async fn check_all(&self) -> HealthReport {
        let mut snapshots = Vec::new();
        let mut healthy = 0;
        let mut degraded = 0;
        let mut unhealthy = 0;

        for check in &self.checks {
            let start = std::time::Instant::now();
            let result = check.check().await;
            let duration = start.elapsed();

            let (status, error) = match result {
                Ok(s) => {
                    match s {
                        HealthStatus::Healthy => healthy += 1,
                        HealthStatus::Degraded => degraded += 1,
                        HealthStatus::Unhealthy => unhealthy += 1,
                    }
                    (s, None)
                }
                Err(e) => {
                    unhealthy += 1;
                    (HealthStatus::Unhealthy, Some(e.to_string()))
                }
            };

            snapshots.push(HealthSnapshot {
                component: check.name().to_string(),
                status,
                timestamp: chrono::Utc::now(),
                latency_ms: Some(duration.as_millis() as u64),
                error,
            });
        }

        // Determine overall status (worst wins)
        let overall = if unhealthy > 0 {
            HealthStatus::Unhealthy
        } else if degraded > 0 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        HealthReport {
            overall_status: overall,
            summary: ReportSummary {
                total: snapshots.len(),
                healthy,
                degraded,
                unhealthy,
            },
            checks: snapshots,
        }
    }

    /// Get number of registered checks
    pub fn len(&self) -> usize {
        self.checks.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.checks.is_empty()
    }
}

/// Project health information - extends basic health with project-specific fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectHealth {
    /// Project name
    pub project_name: String,
    /// Overall health status
    pub status: HealthStatus,
    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
    /// Component health details
    pub components: Vec<ComponentHealth>,
    /// Health score (0-100)
    pub score: u8,
}

/// Individual component health within a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component name
    pub name: String,
    /// Component type (e.g., "database", "api", "worker")
    pub component_type: String,
    /// Health status
    pub status: HealthStatus,
    /// Optional status message
    pub message: Option<String>,
    /// Last check timestamp
    pub last_check: chrono::DateTime<chrono::Utc>,
}

/// Aggregator for project-level health
#[derive(Debug, Default)]
pub struct ProjectHealthAggregator {
    projects: Arc<RwLock<HashMap<String, ProjectHealth>>>,
}

impl ProjectHealthAggregator {
    /// Create a new aggregator
    pub fn new() -> Self {
        Self {
            projects: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Update health for a project
    pub async fn update_project(&self, health: ProjectHealth) {
        let mut projects = self.projects.write().await;
        projects.insert(health.project_name.clone(), health);
    }

    /// Get health for a specific project
    pub async fn get_project(&self, name: &str) -> Option<ProjectHealth> {
        let projects = self.projects.read().await;
        projects.get(name).cloned()
    }

    /// Get all project healths
    pub async fn all_projects(&self) -> Vec<ProjectHealth> {
        let projects = self.projects.read().await;
        projects.values().cloned().collect()
    }

    /// Calculate overall workspace health score
    pub async fn workspace_score(&self) -> u8 {
        let projects = self.projects.read().await;
        if projects.is_empty() {
            return 0;
        }

        let total_score: u32 = projects.values().map(|p| p.score as u32).sum();
        (total_score / projects.len() as u32) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_display() {
        assert_eq!(HealthStatus::Healthy.to_string(), "healthy");
        assert_eq!(HealthStatus::Degraded.to_string(), "degraded");
        assert_eq!(HealthStatus::Unhealthy.to_string(), "unhealthy");
    }

    #[test]
    fn test_component_health_check() {
        let check = ComponentHealthCheck::new("test", HealthStatus::Healthy);
        assert_eq!(check.name(), "test");
    }

    #[tokio::test]
    async fn test_registry_check_all() {
        let mut registry = HealthRegistry::new();
        registry.register(ComponentHealthCheck::new("db", HealthStatus::Healthy));
        registry.register(ComponentHealthCheck::new("cache", HealthStatus::Healthy));

        let report = registry.check_all().await;
        assert_eq!(report.summary.total, 2);
        assert_eq!(report.summary.healthy, 2);
        assert_eq!(report.overall_status, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_registry_with_degraded() {
        let mut registry = HealthRegistry::new();
        registry.register(ComponentHealthCheck::new("db", HealthStatus::Healthy));
        registry.register(ComponentHealthCheck::new("queue", HealthStatus::Degraded));

        let report = registry.check_all().await;
        assert_eq!(report.summary.healthy, 1);
        assert_eq!(report.summary.degraded, 1);
        assert_eq!(report.overall_status, HealthStatus::Degraded);
    }

    #[tokio::test]
    async fn test_project_aggregator() {
        let aggregator = ProjectHealthAggregator::new();

        let health = ProjectHealth {
            project_name: "test-project".to_string(),
            status: HealthStatus::Healthy,
            last_updated: chrono::Utc::now(),
            components: vec![],
            score: 95,
        };

        aggregator.update_project(health).await;

        let retrieved = aggregator.get_project("test-project").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().score, 95);
    }
}
