//! Prometheus metrics integration for health checks
//!
//! Provides metrics export for health status and check latencies.

use metrics::{describe_gauge, gauge, histogram, Unit};
use std::sync::Arc;

use crate::{HealthRegistry, HealthReport, HealthStatus};

/// Initialize health-related metrics
///
/// Call this once at application startup to register metric descriptions.
pub fn init_health_metrics() {
    describe_gauge!(
        "health_status",
        Unit::Count,
        "Overall health status: 0=unhealthy, 1=degraded, 2=healthy"
    );
    describe_gauge!(
        "health_component_status",
        Unit::Count,
        "Individual component health status: 0=unhealthy, 1=degraded, 2=healthy"
    );
    describe_gauge!(
        "health_total_checks",
        Unit::Count,
        "Total number of registered health checks"
    );
    describe_gauge!(
        "health_healthy_count",
        Unit::Count,
        "Number of healthy components"
    );
    describe_gauge!(
        "health_degraded_count",
        Unit::Count,
        "Number of degraded components"
    );
    describe_gauge!(
        "health_unhealthy_count",
        Unit::Count,
        "Number of unhealthy components"
    );
    metrics::describe_histogram!(
        "health_check_latency_ms",
        Unit::Milliseconds,
        "Health check latency in milliseconds"
    );
}

/// Record metrics from a health report
///
/// This should be called after each health check run to update metrics.
///
/// # Example
///
/// ```rust,no_run
/// use phenotype_health::metrics::record_health_metrics;
/// use phenotype_health::{HealthRegistry, HealthReport};
///
/// async fn update_metrics(registry: &HealthRegistry) {
///     let report = registry.check_all().await;
///     record_health_metrics(&report);
/// }
/// ```
pub fn record_health_metrics(report: &HealthReport) {
    // Record overall status as numeric value
    let status_value = match report.overall_status {
        HealthStatus::Unhealthy => 0.0,
        HealthStatus::Degraded => 1.0,
        HealthStatus::Healthy => 2.0,
    };
    gauge!("health_status", status_value);

    // Record component statuses
    for check in &report.checks {
        let component_status = match check.status {
            HealthStatus::Unhealthy => 0.0,
            HealthStatus::Degraded => 1.0,
            HealthStatus::Healthy => 2.0,
        };
        gauge!(
            "health_component_status",
            component_status,
            "component" => check.component.clone()
        );

        // Record latency if available
        if let Some(latency) = check.latency_ms {
            histogram!(
                "health_check_latency_ms",
                latency as f64,
                "component" => check.component.clone()
            );
        }
    }

    // Record summary counts
    gauge!("health_total_checks", report.summary.total as f64);
    gauge!("health_healthy_count", report.summary.healthy as f64);
    gauge!("health_degraded_count", report.summary.degraded as f64);
    gauge!("health_unhealthy_count", report.summary.unhealthy as f64);
}

/// Metrics recorder that automatically records metrics after checks
#[derive(Debug, Clone)]
pub struct MetricsRecorder {
    registry: Arc<HealthRegistry>,
}

impl MetricsRecorder {
    /// Create a new metrics recorder
    pub fn new(registry: Arc<HealthRegistry>) -> Self {
        Self { registry }
    }

    /// Run all checks and record metrics
    pub async fn check_and_record(&self) -> HealthReport {
        let report = self.registry.check_all().await;
        record_health_metrics(&report);
        report
    }
}

/// Install Prometheus metrics exporter
///
/// This installs the Prometheus exporter and initializes health metrics.
/// Call this at application startup before recording any metrics.
///
/// # Example
///
/// ```rust,no_run
/// use phenotype_health::metrics::install_prometheus;
///
/// fn main() {
///     install_prometheus();
///     // ... rest of your application
/// }
/// ```
#[cfg(feature = "prometheus")]
pub fn install_prometheus() {
    use metrics_exporter_prometheus::PrometheusBuilder;

    PrometheusBuilder::new()
        .install_recorder()
        .expect("Failed to install Prometheus recorder");

    init_health_metrics();
}

/// Get a Prometheus metrics handler for HTTP endpoints
///
/// Returns a handler function that can be used with axum or other HTTP frameworks
/// to expose metrics at the `/metrics` endpoint.
#[cfg(feature = "prometheus")]
pub fn prometheus_handler() -> impl Fn() -> String + Clone {
    use metrics_exporter_prometheus::PrometheusHandle;

    let handle = metrics_exporter_prometheus::PrometheusBuilder::new()
        .install_recorder()
        .expect("Failed to install Prometheus recorder")
        .handle();

    init_health_metrics();

    move || handle.render()
}
