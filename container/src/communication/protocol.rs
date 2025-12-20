use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRequest {
    pub id: String,
    pub request_type: RequestType,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RequestType {
    Chat,
    SpecOperation,
    CommandExecution,
    FileOperation,
    HealthCheck,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub id: String,
    pub success: bool,
    pub payload: serde_json::Value,
    pub error: Option<String>,
}

/// Command execution request payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecutionRequest {
    pub command: String,
    pub working_directory: String,
    pub description: String,
}

/// Command execution response payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecutionResponse {
    pub block_id: String,
    pub approval_status: String,
    pub output: Option<CommandExecutionOutput>,
}

/// Command execution output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecutionOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub success: bool,
}

/// Command approval request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandApprovalRequest {
    pub block_id: String,
    pub approved: bool,
}

