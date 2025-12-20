use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Container configuration for MCP orchestration layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    /// Docker image name
    pub image: String,
    /// Docker image tag
    pub tag: String,
    /// Container name
    pub container_name: String,
    /// Port mappings (host:container)
    pub ports: Vec<PortMapping>,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// LLM provider configuration
    pub llm_provider: LlmProviderConfig,
    /// MCP services configuration
    pub mcp_services: Vec<McpServiceConfig>,
    /// Resource limits
    pub resource_limits: ResourceLimits,
    /// Network configuration
    pub network: NetworkConfig,
}

/// Port mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub host_port: u16,
    pub container_port: u16,
    pub protocol: String, // "tcp" or "udp"
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    pub provider_type: LlmProviderType,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

/// Supported LLM providers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LlmProviderType {
    /// Local Ollama instance
    Ollama {
        endpoint: String,
        model: String,
    },
    /// OpenAI API
    OpenAI {
        api_key: String,
        model: String,
        base_url: Option<String>,
    },
    /// Anthropic API
    Anthropic {
        api_key: String,
        model: String,
    },
}

/// MCP service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServiceConfig {
    pub name: String,
    pub endpoint: String,
    pub auth: Option<AuthConfig>,
    pub enabled: bool,
    pub timeout_seconds: u64,
}

/// Authentication configuration for MCP services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub auth_type: String, // "bearer", "api_key", "basic", etc.
    pub credentials: String, // Encrypted or stored securely
}

/// Resource limits for container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub memory_mb: u32,
    pub cpu_shares: u32,
    pub max_open_files: u32,
}

/// Network configuration for container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub network_mode: String, // "bridge", "host", "container", etc.
    pub dns_servers: Vec<String>,
    pub extra_hosts: Vec<String>,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            image: "nvim-spec-agent".to_string(),
            tag: "latest".to_string(),
            container_name: "nvim-spec-agent-container".to_string(),
            ports: vec![
                PortMapping {
                    host_port: 50051,
                    container_port: 50051,
                    protocol: "tcp".to_string(),
                }
            ],
            environment: HashMap::new(),
            llm_provider: LlmProviderConfig {
                provider_type: LlmProviderType::Ollama {
                    endpoint: "http://localhost:11434".to_string(),
                    model: "llama2".to_string(),
                },
                timeout_seconds: 300,
                max_retries: 3,
            },
            mcp_services: vec![],
            resource_limits: ResourceLimits {
                memory_mb: 2048,
                cpu_shares: 1024,
                max_open_files: 1024,
            },
            network: NetworkConfig {
                network_mode: "bridge".to_string(),
                dns_servers: vec![],
                extra_hosts: vec![],
            },
        }
    }
}

impl ContainerConfig {
    /// Load configuration from file
    pub fn load_from_file(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save_to_file(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.image.is_empty() {
            return Err("Image name cannot be empty".to_string());
        }
        
        if self.tag.is_empty() {
            return Err("Image tag cannot be empty".to_string());
        }
        
        if self.container_name.is_empty() {
            return Err("Container name cannot be empty".to_string());
        }
        
        if self.ports.is_empty() {
            return Err("At least one port mapping is required".to_string());
        }
        
        if self.resource_limits.memory_mb < 256 {
            return Err("Memory limit must be at least 256 MB".to_string());
        }
        
        Ok(())
    }

    /// Get full image name with tag
    pub fn full_image_name(&self) -> String {
        format!("{}:{}", self.image, self.tag)
    }

    /// Get gRPC endpoint
    pub fn grpc_endpoint(&self) -> String {
        if let Some(port_mapping) = self.ports.first() {
            format!("http://localhost:{}", port_mapping.host_port)
        } else {
            "http://localhost:50051".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ContainerConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_full_image_name() {
        let config = ContainerConfig::default();
        assert_eq!(config.full_image_name(), "nvim-spec-agent:latest");
    }

    #[test]
    fn test_grpc_endpoint() {
        let config = ContainerConfig::default();
        assert_eq!(config.grpc_endpoint(), "http://localhost:50051");
    }

    #[test]
    fn test_validation() {
        let mut config = ContainerConfig::default();
        assert!(config.validate().is_ok());

        config.image = String::new();
        assert!(config.validate().is_err());

        config.image = "nvim-spec-agent".to_string();
        config.resource_limits.memory_mb = 100;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_serialization() {
        let config = ContainerConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ContainerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.image, deserialized.image);
    }
}
