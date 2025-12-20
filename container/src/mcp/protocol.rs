use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCall {
    pub tool_name: String,
    pub arguments: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    pub tool_name: String,
    pub result: serde_json::Value,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServiceConfig {
    pub name: String,
    pub endpoint: String,
    pub auth: Option<McpAuthConfig>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum McpAuthConfig {
    ApiKey { key: String },
    Bearer { token: String },
    Basic { username: String, password: String },
    OAuth2 { client_id: String, client_secret: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServiceInfo {
    pub name: String,
    pub version: String,
    pub tools: Vec<McpTool>,
}
