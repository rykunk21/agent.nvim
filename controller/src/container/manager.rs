use super::config::ContainerConfig;
use super::health::{HealthMonitor, HealthStatus};
use log::{info, warn, error, debug};
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;
use serde::{Deserialize, Serialize};

/// Container state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContainerState {
    /// Container is not started
    NotStarted,
    /// Container is starting
    Starting,
    /// Container is running
    Running,
    /// Container is stopping
    Stopping,
    /// Container is stopped
    Stopped,
    /// Container encountered an error
    Error(String),
}

/// Container manager for Docker operations
pub struct ContainerManager {
    config: ContainerConfig,
    state: ContainerState,
    container_id: Option<String>,
    health_monitor: HealthMonitor,
    restart_count: u32,
    max_restart_attempts: u32,
}

impl ContainerManager {
    /// Create a new container manager
    pub fn new(config: ContainerConfig) -> Result<Self, String> {
        config.validate()?;
        
        Ok(Self {
            config,
            state: ContainerState::NotStarted,
            container_id: None,
            health_monitor: HealthMonitor::new(),
            restart_count: 0,
            max_restart_attempts: 3,
        })
    }

    /// Get current container state
    pub fn state(&self) -> &ContainerState {
        &self.state
    }

    /// Get container ID if running
    pub fn container_id(&self) -> Option<&str> {
        self.container_id.as_deref()
    }

    /// Get health status
    pub fn health_status(&self) -> HealthStatus {
        self.health_monitor.get_status()
    }

    /// Pull the Docker image
    pub async fn pull_image(&mut self) -> Result<(), String> {
        info!("Pulling Docker image: {}", self.config.full_image_name());
        
        let output = Command::new("docker")
            .args(&["pull", &self.config.full_image_name()])
            .output()
            .map_err(|e| format!("Failed to execute docker pull: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Docker pull failed: {}", stderr));
        }

        info!("Successfully pulled Docker image");
        Ok(())
    }

    /// Start the container
    pub async fn start_container(&mut self) -> Result<(), String> {
        if self.state == ContainerState::Running {
            warn!("Container is already running");
            return Ok(());
        }

        self.state = ContainerState::Starting;
        info!("Starting container: {}", self.config.container_name);

        // Build docker run command
        let mut cmd = Command::new("docker");
        cmd.arg("run");
        cmd.arg("-d"); // Detached mode
        cmd.arg("--name").arg(&self.config.container_name);

        // Add port mappings
        for port in &self.config.ports {
            cmd.arg("-p")
                .arg(format!("{}:{}/{}", port.host_port, port.container_port, port.protocol));
        }

        // Add environment variables
        for (key, value) in &self.config.environment {
            cmd.arg("-e").arg(format!("{}={}", key, value));
        }

        // Add resource limits
        cmd.arg("-m").arg(format!("{}m", self.config.resource_limits.memory_mb));
        cmd.arg("--cpus").arg(format!("{}", self.config.resource_limits.cpu_shares as f64 / 1024.0));

        // Add network configuration
        cmd.arg("--network").arg(&self.config.network.network_mode);

        // Add DNS servers if configured
        for dns in &self.config.network.dns_servers {
            cmd.arg("--dns").arg(dns);
        }

        // Add extra hosts if configured
        for host in &self.config.network.extra_hosts {
            cmd.arg("--add-host").arg(host);
        }

        // Add image name
        cmd.arg(&self.config.full_image_name());

        // Execute the command
        let output = cmd
            .output()
            .map_err(|e| format!("Failed to execute docker run: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.state = ContainerState::Error(format!("Failed to start container: {}", stderr));
            return Err(format!("Docker run failed: {}", stderr));
        }

        // Extract container ID from output
        let container_id = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();

        if container_id.is_empty() {
            self.state = ContainerState::Error("No container ID returned".to_string());
            return Err("No container ID returned from docker run".to_string());
        }

        self.container_id = Some(container_id.clone());
        self.state = ContainerState::Running;
        self.health_monitor.record_container_start();
        self.restart_count = 0;

        info!("Container started successfully: {}", container_id);
        Ok(())
    }

    /// Stop the container
    pub async fn stop_container(&mut self) -> Result<(), String> {
        if self.state == ContainerState::Stopped || self.state == ContainerState::NotStarted {
            return Ok(());
        }

        self.state = ContainerState::Stopping;
        info!("Stopping container: {}", self.config.container_name);

        let output = Command::new("docker")
            .args(&["stop", &self.config.container_name])
            .output()
            .map_err(|e| format!("Failed to execute docker stop: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Docker stop returned non-zero exit code: {}", stderr);
        }

        self.state = ContainerState::Stopped;
        self.container_id = None;
        self.health_monitor.reset();

        info!("Container stopped");
        Ok(())
    }

    /// Remove the container
    pub async fn remove_container(&mut self) -> Result<(), String> {
        // Stop first if running
        if self.state == ContainerState::Running {
            self.stop_container().await?;
        }

        info!("Removing container: {}", self.config.container_name);

        let output = Command::new("docker")
            .args(&["rm", "-f", &self.config.container_name])
            .output()
            .map_err(|e| format!("Failed to execute docker rm: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Docker rm returned non-zero exit code: {}", stderr);
        }

        self.state = ContainerState::NotStarted;
        self.container_id = None;

        info!("Container removed");
        Ok(())
    }

    /// Check if container is running
    pub async fn is_running(&self) -> Result<bool, String> {
        let output = Command::new("docker")
            .args(&["ps", "-q", "-f", &format!("name={}", self.config.container_name)])
            .output()
            .map_err(|e| format!("Failed to execute docker ps: {}", e))?;

        if !output.status.success() {
            return Err("Failed to check container status".to_string());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(!stdout.trim().is_empty())
    }

    /// Perform health check on container
    pub async fn health_check(&mut self) -> Result<(), String> {
        if !self.health_monitor.is_check_due() {
            return Ok(());
        }

        debug!("Performing health check on container");

        // Check if container is running
        match self.is_running().await {
            Ok(true) => {
                // Try to connect to gRPC endpoint
                let endpoint = self.config.grpc_endpoint();
                match self.check_grpc_endpoint(&endpoint).await {
                    Ok(response_time) => {
                        self.health_monitor.record_success(response_time);
                        Ok(())
                    }
                    Err(e) => {
                        self.health_monitor.record_failure(e.clone());
                        Err(e)
                    }
                }
            }
            Ok(false) => {
                let error = "Container is not running".to_string();
                self.health_monitor.record_failure(error.clone());
                Err(error)
            }
            Err(e) => {
                self.health_monitor.record_failure(e.clone());
                Err(e)
            }
        }
    }

    /// Check gRPC endpoint connectivity
    async fn check_grpc_endpoint(&self, endpoint: &str) -> Result<u64, String> {
        let start = std::time::Instant::now();
        
        // Simple TCP connection check to gRPC port
        let url = endpoint.replace("http://", "");
        match tokio::net::TcpStream::connect(&url).await {
            Ok(_) => {
                let elapsed = start.elapsed().as_millis() as u64;
                debug!("gRPC endpoint check successful: {}ms", elapsed);
                Ok(elapsed)
            }
            Err(e) => {
                Err(format!("Failed to connect to gRPC endpoint {}: {}", endpoint, e))
            }
        }
    }

    /// Restart container with exponential backoff
    pub async fn restart_with_backoff(&mut self) -> Result<(), String> {
        if self.restart_count >= self.max_restart_attempts {
            let error = format!(
                "Max restart attempts ({}) exceeded",
                self.max_restart_attempts
            );
            self.state = ContainerState::Error(error.clone());
            return Err(error);
        }

        let backoff_secs = 2_u64.pow(self.restart_count);
        warn!(
            "Restarting container after {}s (attempt {}/{})",
            backoff_secs, self.restart_count + 1, self.max_restart_attempts
        );

        sleep(Duration::from_secs(backoff_secs)).await;

        self.restart_count += 1;
        self.remove_container().await?;
        self.start_container().await?;

        Ok(())
    }

    /// Get container logs
    pub async fn get_logs(&self, lines: Option<u32>) -> Result<String, String> {
        let mut cmd = Command::new("docker");
        cmd.args(&["logs", &self.config.container_name]);

        if let Some(n) = lines {
            cmd.arg("--tail").arg(n.to_string());
        }

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to execute docker logs: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Docker logs failed: {}", stderr));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Get container stats
    pub async fn get_stats(&self) -> Result<ContainerStats, String> {
        let output = Command::new("docker")
            .args(&["stats", "--no-stream", "--format", "json", &self.config.container_name])
            .output()
            .map_err(|e| format!("Failed to execute docker stats: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Docker stats failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stats: ContainerStats = serde_json::from_str(&stdout)
            .map_err(|e| format!("Failed to parse docker stats: {}", e))?;

        Ok(stats)
    }

    /// Set maximum restart attempts
    pub fn set_max_restart_attempts(&mut self, max_attempts: u32) {
        self.max_restart_attempts = max_attempts;
    }

    /// Get restart count
    pub fn restart_count(&self) -> u32 {
        self.restart_count
    }

    /// Get configuration
    pub fn config(&self) -> &ContainerConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: ContainerConfig) -> Result<(), String> {
        config.validate()?;
        self.config = config;
        Ok(())
    }
}

/// Container statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStats {
    #[serde(rename = "Container")]
    pub container: String,
    #[serde(rename = "CPUPerc")]
    pub cpu_percent: String,
    #[serde(rename = "MemUsage")]
    pub mem_usage: String,
    #[serde(rename = "MemPerc")]
    pub mem_percent: String,
    #[serde(rename = "NetIO")]
    pub net_io: String,
    #[serde(rename = "BlockIO")]
    pub block_io: String,
    #[serde(rename = "PIDs")]
    pub pids: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_manager_creation() {
        let config = ContainerConfig::default();
        let manager = ContainerManager::new(config);
        assert!(manager.is_ok());
    }

    #[test]
    fn test_container_state_transitions() {
        let config = ContainerConfig::default();
        let manager = ContainerManager::new(config).unwrap();
        
        assert_eq!(manager.state(), &ContainerState::NotStarted);
    }

    #[test]
    fn test_invalid_config() {
        let mut config = ContainerConfig::default();
        config.image = String::new();
        
        let result = ContainerManager::new(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_restart_count() {
        let config = ContainerConfig::default();
        let mut manager = ContainerManager::new(config).unwrap();
        
        assert_eq!(manager.restart_count(), 0);
        manager.set_max_restart_attempts(5);
        assert_eq!(manager.restart_count(), 0);
    }
}
