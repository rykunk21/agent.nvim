use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::time::Duration as StdDuration;
use log::{debug, warn};

/// Container health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    /// Container is healthy and responsive
    Healthy,
    /// Container is starting up
    Starting,
    /// Container is unhealthy
    Unhealthy { reason: String },
    /// Container is not running
    NotRunning,
    /// Health check not yet performed
    Unknown,
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub status: HealthStatus,
    pub timestamp: DateTime<Utc>,
    pub response_time_ms: u64,
    pub error_message: Option<String>,
}

/// Health monitoring for container
#[derive(Debug)]
pub struct HealthMonitor {
    /// Last health check result
    last_check: Option<HealthCheckResult>,
    /// Number of consecutive failures
    failure_count: u32,
    /// Maximum consecutive failures before marking unhealthy
    max_failures: u32,
    /// Time between health checks
    check_interval: StdDuration,
    /// Last check timestamp
    last_check_time: Option<DateTime<Utc>>,
    /// Startup grace period
    startup_grace_period: StdDuration,
    /// Container start time
    container_start_time: Option<DateTime<Utc>>,
}

impl HealthMonitor {
    /// Create a new health monitor
    pub fn new() -> Self {
        Self {
            last_check: None,
            failure_count: 0,
            max_failures: 3,
            check_interval: StdDuration::from_secs(30),
            last_check_time: None,
            startup_grace_period: StdDuration::from_secs(60),
            container_start_time: None,
        }
    }

    /// Record a successful health check
    pub fn record_success(&mut self, response_time_ms: u64) {
        self.last_check = Some(HealthCheckResult {
            status: HealthStatus::Healthy,
            timestamp: Utc::now(),
            response_time_ms,
            error_message: None,
        });
        self.failure_count = 0;
        self.last_check_time = Some(Utc::now());
        debug!("Health check successful, response time: {}ms", response_time_ms);
    }

    /// Record a failed health check
    pub fn record_failure(&mut self, error: String) {
        self.failure_count += 1;
        
        let status = if self.failure_count >= self.max_failures {
            HealthStatus::Unhealthy {
                reason: format!("Failed {} consecutive health checks", self.failure_count),
            }
        } else {
            HealthStatus::Starting
        };

        self.last_check = Some(HealthCheckResult {
            status,
            timestamp: Utc::now(),
            response_time_ms: 0,
            error_message: Some(error.clone()),
        });
        self.last_check_time = Some(Utc::now());
        warn!("Health check failed (attempt {}): {}", self.failure_count, error);
    }

    /// Record container start
    pub fn record_container_start(&mut self) {
        self.container_start_time = Some(Utc::now());
        self.failure_count = 0;
        self.last_check = Some(HealthCheckResult {
            status: HealthStatus::Starting,
            timestamp: Utc::now(),
            response_time_ms: 0,
            error_message: None,
        });
        debug!("Container started, entering grace period");
    }

    /// Get current health status
    pub fn get_status(&self) -> HealthStatus {
        // Check if we're in startup grace period
        if let Some(start_time) = self.container_start_time {
            let elapsed = Utc::now().signed_duration_since(start_time);
            if elapsed < Duration::from_std(self.startup_grace_period).unwrap_or_default() {
                return HealthStatus::Starting;
            }
        }

        // Return last check status or Unknown
        self.last_check
            .as_ref()
            .map(|check| check.status.clone())
            .unwrap_or(HealthStatus::Unknown)
    }

    /// Check if health check is due
    pub fn is_check_due(&self) -> bool {
        match self.last_check_time {
            None => true,
            Some(last_time) => {
                let elapsed = Utc::now().signed_duration_since(last_time);
                elapsed > Duration::from_std(self.check_interval).unwrap_or_default()
            }
        }
    }

    /// Get last health check result
    pub fn last_check_result(&self) -> Option<&HealthCheckResult> {
        self.last_check.as_ref()
    }

    /// Reset health monitor
    pub fn reset(&mut self) {
        self.last_check = None;
        self.failure_count = 0;
        self.last_check_time = None;
        self.container_start_time = None;
    }

    /// Set maximum consecutive failures threshold
    pub fn set_max_failures(&mut self, max_failures: u32) {
        self.max_failures = max_failures;
    }

    /// Set check interval
    pub fn set_check_interval(&mut self, interval: StdDuration) {
        self.check_interval = interval;
    }

    /// Get failure count
    pub fn failure_count(&self) -> u32 {
        self.failure_count
    }

    /// Check if container is in startup grace period
    pub fn is_in_startup_grace_period(&self) -> bool {
        if let Some(start_time) = self.container_start_time {
            let elapsed = Utc::now().signed_duration_since(start_time);
            elapsed < Duration::from_std(self.startup_grace_period).unwrap_or_default()
        } else {
            false
        }
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_health_monitor_creation() {
        let monitor = HealthMonitor::new();
        assert_eq!(monitor.get_status(), HealthStatus::Unknown);
        assert!(monitor.is_check_due());
    }

    #[test]
    fn test_record_success() {
        let mut monitor = HealthMonitor::new();
        monitor.record_success(50);
        
        assert_eq!(monitor.get_status(), HealthStatus::Healthy);
        assert_eq!(monitor.failure_count(), 0);
        assert!(monitor.last_check_result().is_some());
    }

    #[test]
    fn test_record_failure() {
        let mut monitor = HealthMonitor::new();
        monitor.set_max_failures(2);
        
        monitor.record_failure("Connection timeout".to_string());
        assert_eq!(monitor.failure_count(), 1);
        assert_eq!(monitor.get_status(), HealthStatus::Starting);
        
        monitor.record_failure("Connection timeout".to_string());
        assert_eq!(monitor.failure_count(), 2);
        assert!(matches!(monitor.get_status(), HealthStatus::Unhealthy { .. }));
    }

    #[test]
    fn test_startup_grace_period() {
        let mut monitor = HealthMonitor::new();
        monitor.set_check_interval(StdDuration::from_secs(1));
        monitor.set_max_failures(1);
        
        monitor.record_container_start();
        assert!(monitor.is_in_startup_grace_period());
        assert_eq!(monitor.get_status(), HealthStatus::Starting);
        
        // Record failure during grace period
        monitor.record_failure("Connection timeout".to_string());
        // Should still be Starting due to grace period
        assert_eq!(monitor.get_status(), HealthStatus::Starting);
    }

    #[test]
    fn test_check_due() {
        let mut monitor = HealthMonitor::new();
        monitor.set_check_interval(StdDuration::from_millis(100));
        
        assert!(monitor.is_check_due());
        
        monitor.record_success(50);
        assert!(!monitor.is_check_due());
        
        // Wait for interval to pass
        thread::sleep(StdDuration::from_millis(150));
        assert!(monitor.is_check_due());
    }

    #[test]
    fn test_reset() {
        let mut monitor = HealthMonitor::new();
        monitor.record_success(50);
        monitor.record_failure("Error".to_string());
        
        assert_eq!(monitor.failure_count(), 1);
        
        monitor.reset();
        assert_eq!(monitor.failure_count(), 0);
        assert_eq!(monitor.get_status(), HealthStatus::Unknown);
    }
}
