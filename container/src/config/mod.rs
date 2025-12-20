use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    pub grpc_port: u16,
    pub log_level: String,
    pub llm_provider: LlmProviderConfig,
    pub mcp_services: Vec<McpServiceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    pub provider_type: String,
    pub endpoint: Option<String>,
    pub model: String,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServiceConfig {
    pub name: String,
    pub endpoint: String,
    pub enabled: bool,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        ContainerConfig {
            grpc_port: 50051,
            log_level: "info".to_string(),
            llm_provider: LlmProviderConfig {
                provider_type: "ollama".to_string(),
                endpoint: Some("http://localhost:11434".to_string()),
                model: "llama2".to_string(),
                api_key: None,
            },
            mcp_services: Vec::new(),
        }
    }
}
