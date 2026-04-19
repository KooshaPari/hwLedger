//! Background health check scheduling
//!
//! Provides scheduled health checks that run periodically and trigger
//! callbacks when health status changes.

use crate::{HealthRegistry, HealthReport, HealthStatus};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio::time::interval;
use tracing::info;

/// Events emitted by the background scheduler
#[derive(Debug, Clone)]
pub enum HealthEvent {
    /// Health status changed for a component
    StatusChanged {
        component: String,
        old_status: HealthStatus,
        new_status: HealthStatus,
    },
    /// Overall health status changed
    OverallStatusChanged {
        old_status: HealthStatus,
        new_status: HealthStatus,
    },
    /// A check failed with an error
    CheckFailed { component: String, error: String },
    /// Periodic check completed
    CheckCompleted(HealthReport),
}

/// Background health check scheduler
#[derive(Debug)]
pub struct BackgroundScheduler {
    registry: Arc<HealthRegistry>,
    event_tx: broadcast::Sender<HealthEvent>,
    interval: Duration,
}

impl BackgroundScheduler {
    /// Create a new background scheduler
    ///
    /// # Arguments
    ///
    /// * `registry` - The health registry to check
    /// * `interval` - How often to run checks
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use phenotype_health::background::BackgroundScheduler;
    /// use phenotype_health::HealthRegistry;
    /// use std::sync::Arc;
    /// use std::time::Duration;
    ///
    /// let registry = Arc::new(HealthRegistry::new());
    /// let scheduler = BackgroundScheduler::new(registry, Duration::from_secs(30));
    /// ```
    pub fn new(registry: Arc<HealthRegistry>, interval: Duration) -> Self {
        let (event_tx, _) = broadcast::channel(100);

        Self {
            registry,
            event_tx,
            interval,
        }
    }

    /// Subscribe to health events
    ///
    /// Returns a receiver that will receive `HealthEvent` notifications
    /// when health status changes or checks complete.
    pub fn subscribe(&self) -> broadcast::Receiver<HealthEvent> {
        self.event_tx.subscribe()
    }

    /// Start the background scheduler
    ///
    /// Spawns a background task that periodically runs health checks
    /// and emits events. Returns a `JoinHandle` that can be used to
    /// await or abort the scheduler.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use phenotype_health::background::BackgroundScheduler;
    /// use phenotype_health::HealthRegistry;
    /// use std::sync::Arc;
    /// use std::time::Duration;
    ///
    /// async fn start_monitoring() {
    ///     let registry = Arc::new(HealthRegistry::new());
    ///     let scheduler = BackgroundScheduler::new(registry, Duration::from_secs(30));
    ///     
    ///     // Subscribe to events
    ///     let mut rx = scheduler.subscribe();
    ///     tokio::spawn(async move {
    ///         while let Ok(event) = rx.recv().await {
    ///             println!("Health event: {:?}", event);
    ///         }
    ///     });
    ///     
    ///     // Start the scheduler
    ///     let handle = scheduler.start();
    /// }
    /// ```
    pub fn start(self) -> JoinHandle<()> {
        let registry = self.registry;
        let event_tx = self.event_tx;
        let mut ticker = interval(self.interval);
        let mut last_report: Option<HealthReport> = None;

        tokio::spawn(async move {
            info!("Starting background health check scheduler");

            loop {
                ticker.tick().await;

                let report = registry.check_all().await;

                // Compare with last report and emit events for changes
                if let Some(ref last) = last_report {
                    // Check overall status change
                    if last.overall_status != report.overall_status {
                        let _ = event_tx.send(HealthEvent::OverallStatusChanged {
                            old_status: last.overall_status,
                            new_status: report.overall_status,
                        });
                    }

                    // Check individual component changes
                    for check in &report.checks {
                        if let Some(last_check) =
                            last.checks.iter().find(|c| c.component == check.component)
                        {
                            if last_check.status != check.status {
                                let _ = event_tx.send(HealthEvent::StatusChanged {
                                    component: check.component.clone(),
                                    old_status: last_check.status,
                                    new_status: check.status,
                                });
                            }
                        }
                    }

                    // Check for errors
                    for check in &report.checks {
                        if check.error.is_some()
                            && last
                                .checks
                                .iter()
                                .find(|c| c.component == check.component)
                                .and_then(|c| c.error.as_ref())
                                != check.error.as_ref()
                        {
                            let _ = event_tx.send(HealthEvent::CheckFailed {
                                component: check.component.clone(),
                                error: check.error.clone().unwrap(),
                            });
                        }
                    }
                }

                // Emit completion event
                let _ = event_tx.send(HealthEvent::CheckCompleted(report.clone()));

                last_report = Some(report);
            }
        })
    }

    /// Start with a callback for health events
    ///
    /// This is a convenience method that starts the scheduler and spawns
    /// a task that calls the provided callback for each event.
    pub fn start_with_callback<F>(self, mut callback: F) -> JoinHandle<()>
    where
        F: FnMut(HealthEvent) + Send + 'static,
    {
        let mut rx = self.subscribe();

        // Spawn the callback handler
        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                callback(event);
            }
        });

        // Start the scheduler
        self.start()
    }
}

/// Builder for background scheduler configuration
#[derive(Debug)]
pub struct SchedulerBuilder {
    interval: Duration,
    initial_delay: Option<Duration>,
    max_events: usize,
}

impl Default for SchedulerBuilder {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            initial_delay: None,
            max_events: 100,
        }
    }
}

impl SchedulerBuilder {
    /// Create a new scheduler builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the check interval
    pub fn interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Set an initial delay before the first check
    pub fn initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = Some(delay);
        self
    }

    /// Set the maximum number of events to buffer
    pub fn max_events(mut self, max: usize) -> Self {
        self.max_events = max;
        self
    }

    /// Build the scheduler
    pub fn build(self, registry: Arc<HealthRegistry>) -> BackgroundScheduler {
        let (event_tx, _) = broadcast::channel(self.max_events);

        BackgroundScheduler {
            registry,
            event_tx,
            interval: self.interval,
        }
    }
}

/// Convenience function to start periodic health checks
///
/// This is a simple way to start background monitoring without
/// creating a scheduler manually.
pub fn start_periodic_checks(
    registry: Arc<HealthRegistry>,
    interval: Duration,
) -> (JoinHandle<()>, broadcast::Receiver<HealthEvent>) {
    let scheduler = BackgroundScheduler::new(registry, interval);
    let rx = scheduler.subscribe();
    let handle = scheduler.start();

    (handle, rx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ComponentHealthCheck;

    #[tokio::test]
    async fn test_scheduler_builder() {
        let registry = Arc::new(HealthRegistry::new());

        let scheduler = SchedulerBuilder::new()
            .interval(Duration::from_secs(60))
            .max_events(50)
            .build(registry);

        assert_eq!(scheduler.interval, Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_start_periodic_checks() {
        let registry = Arc::new(HealthRegistry::new());

        let (handle, mut rx) = start_periodic_checks(registry, Duration::from_secs(1));

        // Give it time to run at least one check
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Clean up
        handle.abort();
    }
}
