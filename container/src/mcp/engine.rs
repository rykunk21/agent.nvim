use super::protocol::{McpTool, McpToolCall, McpToolResult};
use crate::llm::{LlmProviderManager, manager::{LlmRequest, LlmResponse}};
use anyhow::Result;
use log::{debug, info};
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;
use futures::StreamExt;

pub struct McpOrchestrationEngine {
    llm_manager: LlmProviderManager,
    available_tools: HashMap<String, McpTool>,
    sessions: HashMap<String, SessionContext>,
}

pub struct SessionContext {
    pub id: String,
    pub messages: Vec<Message>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl McpOrchestrationEngine {
    pub async fn new(llm_manager: LlmProviderManager) -> Result<Self> {
        info!("Initializing MCP orchestration engine");

        Ok(McpOrchestrationEngine {
            llm_manager,
            available_tools: HashMap::new(),
            sessions: HashMap::new(),
        })
    }

    /// Register a tool with the orchestration engine
    pub fn register_tool(&mut self, tool: McpTool) {
        debug!("Registering tool: {}", tool.name);
        self.available_tools.insert(tool.name.clone(), tool);
    }

    /// Get all available tools
    pub fn get_available_tools(&self) -> Vec<McpTool> {
        self.available_tools.values().cloned().collect()
    }

    /// Create a new session
    pub fn create_session(&mut self) -> String {
        let session_id = Uuid::new_v4().to_string();
        let context = SessionContext {
            id: session_id.clone(),
            messages: Vec::new(),
            created_at: chrono::Utc::now(),
        };
        self.sessions.insert(session_id.clone(), context);
        info!("Created session: {}", session_id);
        session_id
    }

    /// Process a user message and generate a response
    pub async fn process_message(
        &mut self,
        session_id: &str,
        user_message: &str,
    ) -> Result<String> {
        debug!("Processing message for session: {}", session_id);

        // Build system prompt first (before borrowing session)
        let system_prompt = self.build_system_prompt();

        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

        // Add user message to history
        session.messages.push(Message {
            role: "user".to_string(),
            content: user_message.to_string(),
        });

        // Prepare messages for LLM
        let mut messages_text = String::new();
        for msg in &session.messages {
            messages_text.push_str(&format!("{}: {}\n", msg.role, msg.content));
        }

        // Send to LLM
        let llm_request = LlmRequest {
            prompt: messages_text,
            max_tokens: Some(2048),
            temperature: Some(0.7),
            system_prompt: Some(system_prompt),
        };

        let response = self.llm_manager.request(llm_request).await?;

        // Add assistant response to history
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
        
        session.messages.push(Message {
            role: "assistant".to_string(),
            content: response.content.clone(),
        });

        Ok(response.content)
    }

    /// Stream a user message and generate a response
    pub async fn stream_message(
        &mut self,
        session_id: &str,
        user_message: &str,
    ) -> Result<Box<dyn futures::Stream<Item = Result<String>> + Unpin + Send>> {
        debug!("Streaming message for session: {}", session_id);

        // Build system prompt first (before borrowing session)
        let system_prompt = self.build_system_prompt();

        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

        // Add user message to history
        session.messages.push(Message {
            role: "user".to_string(),
            content: user_message.to_string(),
        });

        // Prepare messages for LLM
        let mut messages_text = String::new();
        for msg in &session.messages {
            messages_text.push_str(&format!("{}: {}\n", msg.role, msg.content));
        }

        // Send to LLM with streaming
        let llm_request = LlmRequest {
            prompt: messages_text,
            max_tokens: Some(2048),
            temperature: Some(0.7),
            system_prompt: Some(system_prompt),
        };

        let stream = self.llm_manager.stream_request(llm_request).await?;

        Ok(Box::new(stream.map(|result| {
            result.map_err(|e| anyhow::anyhow!("Stream error: {}", e))
        })))
    }

    /// Extract tool calls from LLM response
    pub fn extract_tool_calls(&self, response: &str) -> Vec<McpToolCall> {
        // This is a simplified implementation
        // In production, you'd parse the LLM response more carefully
        let mut tool_calls = Vec::new();

        // Look for tool call patterns in the response
        if let Ok(json) = serde_json::from_str::<Value>(response) {
            if let Some(calls) = json.get("tool_calls").and_then(|v| v.as_array()) {
                for call in calls {
                    if let (Some(name), Some(args)) = (
                        call.get("name").and_then(|v| v.as_str()),
                        call.get("arguments").and_then(|v| v.as_object()),
                    ) {
                        tool_calls.push(McpToolCall {
                            tool_name: name.to_string(),
                            arguments: args
                                .iter()
                                .map(|(k, v)| (k.clone(), v.clone()))
                                .collect(),
                        });
                    }
                }
            }
        }

        tool_calls
    }

    /// Execute a tool call
    pub async fn execute_tool_call(&self, tool_call: &McpToolCall) -> Result<McpToolResult> {
        debug!("Executing tool: {}", tool_call.tool_name);

        let _tool = self
            .available_tools
            .get(&tool_call.tool_name)
            .ok_or_else(|| anyhow::anyhow!("Tool not found: {}", tool_call.tool_name))?;

        // In a real implementation, this would execute the actual tool
        // For now, we'll return a placeholder result
        Ok(McpToolResult {
            tool_name: tool_call.tool_name.clone(),
            result: json!({
                "status": "executed",
                "message": format!("Tool {} executed successfully", tool_call.tool_name)
            }),
            is_error: false,
        })
    }

    /// Get session context
    pub fn get_session(&self, session_id: &str) -> Option<&SessionContext> {
        self.sessions.get(session_id)
    }

    /// Clear session
    pub fn clear_session(&mut self, session_id: &str) -> Result<()> {
        self.sessions
            .remove(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
        info!("Cleared session: {}", session_id);
        Ok(())
    }

    fn build_system_prompt(&self) -> String {
        let mut prompt = "You are a helpful AI assistant with access to the following tools:\n\n".to_string();

        for tool in self.available_tools.values() {
            prompt.push_str(&format!("- {}: {}\n", tool.name, tool.description));
        }

        prompt.push_str("\nWhen you need to use a tool, respond with a JSON object containing the tool_calls array.");

        prompt
    }
}
