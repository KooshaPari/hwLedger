//! Integration tests for phenotype-health

use chrono::{Duration, Utc};
use phenotype_health::{
    composite::CompositeHealthCheck,
    history::{HealthHistory, HistoryEntry, TrendAnalyzer},
    ComponentHealthCheck, HealthCheck, HealthRegistry, HealthStatus,
};

/// Test component health check
#[tokio::test]
async fn test_component_health_check() {
    let check = ComponentHealthCheck::new("test", HealthStatus::Healthy);
    assert_eq!(check.name(), "test");

    let result = check.check().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), HealthStatus::Healthy);
}

/// Test health registry
#[tokio::test]
async fn test_health_registry() {
    let mut registry = HealthRegistry::new();

    // Add a simple check
    registry.register(ComponentHealthCheck::new("db", HealthStatus::Healthy));

    let report = registry.check_all().await;
    assert_eq!(report.overall_status, HealthStatus::Healthy);
    assert_eq!(report.checks.len(), 1);
}

/// Test health history tracking
#[test]
fn test_health_history() {
    let mut history = HealthHistory::new(100);

    // Add some entries
    for i in 0..5 {
        history.add(HistoryEntry {
            timestamp: Utc::now() - Duration::seconds(i * 10),
            status: if i % 2 == 0 {
                HealthStatus::Healthy
            } else {
                HealthStatus::Degraded
            },
            latency_ms: Some(100 + i as u64 * 10),
            error: None,
        });
    }

    assert_eq!(history.entries().len(), 5);

    // Check uptime calculation
    let uptime = history.uptime(Duration::minutes(10));
    assert!(uptime > 0.0 && uptime <= 100.0);
}

/// Test health history ring buffer eviction
#[test]
fn test_history_ring_buffer() {
    let mut history = HealthHistory::new(5);

    // Add more entries than capacity
    for i in 0..10 {
        history.add(HistoryEntry {
            timestamp: Utc::now(),
            status: HealthStatus::Healthy,
            latency_ms: Some(i as u64),
            error: None,
        });
    }

    // Should only keep 5 entries
    assert_eq!(history.entries().len(), 5);
}

/// Test composite health check
#[test]
fn test_composite_health_check() {
    let check = CompositeHealthCheck::new(
        "api",
        ComponentHealthCheck::new("api", HealthStatus::Healthy),
    )
    .depends_on("database")
    .depends_on("cache");

    assert_eq!(check.name(), "api");
}

/// Test health status display
#[test]
fn test_health_status_display() {
    assert_eq!(format!("{}", HealthStatus::Healthy), "healthy");
    assert_eq!(format!("{}", HealthStatus::Degraded), "degraded");
    assert_eq!(format!("{}", HealthStatus::Unhealthy), "unhealthy");
}

/// Test health status default
#[test]
fn test_health_status_default() {
    assert_eq!(HealthStatus::default(), HealthStatus::Healthy);
}

/// Test trend analyzer placeholder
#[test]
fn test_trend_analyzer() {
    let history = HealthHistory::new(10);
    let analysis = TrendAnalyzer::analyze(&history);
    assert!(!analysis.is_empty());
}
