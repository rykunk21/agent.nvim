use anyhow::Result;
use log::info;
use serde_json::json;
use crate::command::CommandExecutor;
use crate::communication::protocol::{
    AgentRequest, AgentResponse, RequestType, CommandExecutionRequest, CommandApprovalRequest,
};

/// Handles incoming requests from the Rust controller
pub struct RequestHandler {
    command_executor: CommandExecutor,
}

impl RequestHandler {
    pub fn new() -> Self {
        RequestHandler {
            command_executor: CommandExecutor::new(),
        }
    }

    /// Handle an incoming agent request
    pub async fn handle_request(&mut self, request: AgentRequest) -> Result<AgentResponse> {
        info!("Handling request: {} (type: {:?})", request.id, request.request_type);

        let response = match request.request_type {
            RequestType::Chat => self.handle_chat_request(&request).await,
            RequestType::SpecOperation => self.handle_spec_operation(&request).await,
            RequestType::CommandExecution => self.handle_command_execution(&request).await,
            RequestType::FileOperation => self.handle_file_operation(&request).await,
            RequestType::HealthCheck => self.handle_health_check(&request).await,
        };

        match response {
            Ok(payload) => Ok(AgentResponse {
                id: request.id,
                success: true,
                payload,
                error: None,
            }),
            Err(e) => Ok(AgentResponse {
                id: request.id,
                success: false,
                payload: json!({}),
                error: Some(e.to_string()),
            }),
        }
    }

    /// Handle chat requests
    async fn handle_chat_request(&mut self, request: &AgentRequest) -> Result<serde_json::Value> {
        info!("Processing chat request");
        // TODO: Implement chat handling with LLM provider
        Ok(json!({
            "status": "chat_received",
            "message": "Chat request received and queued for processing"
        }))
    }

    /// Handle spec operations
    async fn handle_spec_operation(&mut self, request: &AgentRequest) -> Result<serde_json::Value> {
        info!("Processing spec operation");
        // TODO: Implement spec operation handling
        Ok(json!({
            "status": "spec_operation_received",
            "message": "Spec operation received and queued for processing"
        }))
    }

    /// Handle command execution requests
    async fn handle_command_execution(&mut self, request: &AgentRequest) -> Result<serde_json::Value> {
        info!("Processing command execution request");

        // Parse the command execution request
        let cmd_request: CommandExecutionRequest = serde_json::from_value(request.payload.clone())?;

        // Create a command block for approval
        let block_id = self.command_executor.create_command_block(
            cmd_request.command,
            cmd_request.working_directory,
            cmd_request.description,
        )?;

        info!("Created command block for approval: {}", block_id);

        // Return the command block ID for the controller to present to the user
        Ok(json!({
            "status": "command_pending_approval",
            "block_id": block_id,
            "message": "Command created and awaiting user approval"
        }))
    }

    /// Handle file operations
    async fn handle_file_operation(&mut self, request: &AgentRequest) -> Result<serde_json::Value> {
        info!("Processing file operation");
        // TODO: Implement file operation handling
        Ok(json!({
            "status": "file_operation_received",
            "message": "File operation received and queued for processing"
        }))
    }

    /// Handle health check requests
    async fn handle_health_check(&mut self, _request: &AgentRequest) -> Result<serde_json::Value> {
        info!("Processing health check");
        Ok(json!({
            "status": "healthy",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "services": {
                "command_executor": "ready",
                "spec_engine": "ready",
                "llm_provider": "ready"
            }
        }))
    }

    /// Approve a command for execution
    pub async fn approve_command(&mut self, block_id: &str) -> Result<serde_json::Value> {
        info!("Approving command: {}", block_id);
        self.command_executor.approve_command(block_id)?;

        // Execute the command
        let output = self.command_executor.execute_command(block_id)?;

        Ok(json!({
            "status": "command_executed",
            "block_id": block_id,
            "output": {
                "stdout": output.stdout,
                "stderr": output.stderr,
                "exit_code": output.exit_code,
                "success": output.success
            }
        }))
    }

    /// Reject a command
    pub async fn reject_command(&mut self, block_id: &str) -> Result<serde_json::Value> {
        info!("Rejecting command: {}", block_id);
        self.command_executor.reject_command(block_id)?;

        Ok(json!({
            "status": "command_rejected",
            "block_id": block_id,
            "message": "Command has been rejected and will not be executed"
        }))
    }

    /// Get command block details
    pub fn get_command_block(&self, block_id: &str) -> Result<serde_json::Value> {
        let command_block = self.command_executor.get_command_block(block_id)?;

        Ok(json!({
            "id": command_block.id,
            "command": command_block.command,
            "working_directory": command_block.working_directory,
            "description": command_block.description,
            "risk_level": format!("{:?}", command_block.risk_level),
            "approval_status": format!("{:?}", command_block.approval_status)
        }))
    }
}

impl Default for RequestHandler {
    fn default() -> Self {
        Self::new()
    }
}
