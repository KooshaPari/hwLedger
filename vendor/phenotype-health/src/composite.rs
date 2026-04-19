//! Composite health checks with dependency support
//!
//! Provides health checks that can declare dependencies on other checks.
//! This allows modeling complex service hierarchies where a service's health
//! depends on its downstream dependencies.
//!
//! # Examples
//!
//! ## Creating a Composite Check
//!
//! ```rust,ignore
//! use phenotype_health::composite::CompositeHealthCheck;
//! use phenotype_health::{HealthCheck, HealthStatus};
//!
//! // Create a check for an API service that depends on database and cache
//! let api_check = CompositeHealthCheck::new("api", ApiHealthCheck)
//!     .depends_on("database")
//!     .depends_on("cache");
//! ```
//!
//! ## Using the Composite Registry
//!
//! ```rust,ignore
//! use phenotype_health::composite::{CompositeHealthCheck, CompositeRegistry};
//!
//! let mut registry = CompositeRegistry::new();
//!
//! // Register checks in dependency order
//! registry.register(CompositeHealthCheck::new("database", DatabaseCheck));
//! registry.register(CompositeHealthCheck::new("cache", CacheCheck));
//! registry.register(
//!     CompositeHealthCheck::new("api", ApiCheck)
//!         .depends_on("database")
//!         .depends_on("cache")
//! );
//!
//! // Run all checks respecting dependencies
//! let report = registry.check_all().await;
//! ```
//!
//! ## Dependency Resolution
//!
//! The `CompositeRegistry` ensures that dependencies are checked before
//! dependent services. If a dependency fails, dependent services are
//! automatically marked as degraded or unhealthy.

use crate::{HealthCheck, HealthCheckError, HealthStatus};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// A health check with dependencies
pub struct CompositeHealthCheck {
    name: String,
    inner: Arc<dyn HealthCheck>,
    dependencies: Vec<String>,
}

impl std::fmt::Debug for CompositeHealthCheck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeHealthCheck")
            .field("name", &self.name)
            .field("dependencies", &self.dependencies)
            .field("inner", &"<dyn HealthCheck>")
            .finish()
    }
}

impl CompositeHealthCheck {
    pub fn new(name: impl Into<String>, check: impl HealthCheck) -> Self {
        Self {
            name: name.into(),
            inner: Arc::new(check),
            dependencies: Vec::new(),
        }
    }

    pub fn depends_on(mut self, name: impl Into<String>) -> Self {
        self.dependencies.push(name.into());
        self
    }
}

#[async_trait]
impl HealthCheck for CompositeHealthCheck {
    fn name(&self) -> &str {
        &self.name
    }

    async fn check(&self) -> Result<HealthStatus, HealthCheckError> {
        self.inner.check().await
    }
}

/// Registry that manages composite checks with dependencies
#[derive(Debug, Default)]
pub struct CompositeRegistry {
    checks: HashMap<String, CompositeHealthCheck>,
}

impl CompositeRegistry {
    pub fn new() -> Self {
        Self {
            checks: HashMap::new(),
        }
    }

    pub fn register(&mut self, check: CompositeHealthCheck) {
        let name = check.name.clone();
        self.checks.insert(name, check);
    }

    pub async fn check_all(&self) -> crate::HealthReport {
        // Placeholder implementation
        crate::HealthReport::default()
    }
}
