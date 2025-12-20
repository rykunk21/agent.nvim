use super::types::*;
use crate::agent::ChatManager;
use crate::spec::workflow::SpecWorkflow;
use crate::utils::error_handling::PluginResult;
use log::{info, error, debug, warn};
use std::path::PathBuf;

/// Main message handler that routes requests to appropriate handlers
pub struct MessageHandler {
    chat_manager: ChatManager,
    spec_workflow: SpecWorkflow,
}

impl MessageHandler {
    pub fn new() -> PluginResult<Self> {
        Ok(Self {
            chat_manager: ChatManager::new(None)?,
            spec_workflow: SpecWorkflow::new(PathBuf::from(".kiro/specs")),
        })
    }
    
    /// Handle incoming message and return optional response
    pub fn handle_message(&mut self, message: Message) -> CommunicationResult<Option<Message>> {
        debug!("Handling message: {:?}", message.payload);
        
        let response_payload = match &message.payload {
            // System messages
            MessagePayload::Ping => Some(MessagePayload::Pong),
            MessagePayload::Startup { .. } => {
                info!("Backend startup acknowledged");
                None
            }
            MessagePayload::Shutdown => {
                info!("Shutdown requested");
                None
            }
            
            // Chat operations
            MessagePayload::ChatMessage { message: msg } => {
                self.handle_chat_message(msg.clone())?
            }
            
            // Spec operations
            MessagePayload::NewSpec { feature_name } => {
                self.handle_new_spec(feature_name.clone())?
            }
            MessagePayload::OpenSpec { spec_name } => {
                self.handle_open_spec(spec_name.clone())?
            }
            
            // Command operations
            MessagePayload::CommandApproval { approved } => {
                self.handle_command_approval(*approved)?
            }
            
            // File operations
            MessagePayload::FileOperation { operation_type, path, .. } => {
                self.handle_file_operation(operation_type.clone(), path.clone())?
            }
            
            // Window operations
            MessagePayload::OpenAgent => {
                info!("Agent interface opened");
                Some(MessagePayload::AgentOpened)
            }
            MessagePayload::CloseAgent => {
                info!("Agent interface closed");
                Some(MessagePayload::AgentClosed)
            }
            
            // State operations
            MessagePayload::SaveState => {
                self.handle_save_state()?
            }
            MessagePayload::LoadState => {
                self.handle_load_state()?
            }
            
            // Resize operations
            MessagePayload::HandleResize { width, height } => {
                self.handle_resize(*width, *height)?
            }
            
            // Error handling
            MessagePayload::Error { message: error_msg, .. } => {
                error!("Received error message: {}", error_msg);
                None
            }
            
            // Unhandled messages
            _ => {
                warn!("Unhandled message type: {:?}", message.payload);
                Some(MessagePayload::Error {
                    message: "Unhandled message type".to_string(),
                    details: Some(format!("{:?}", message.payload)),
                })
            }
        };
        
        // Create response if we have a payload
        let response = if let Some(payload) = response_payload {
            if message.expects_response() {
                Some(Message::new_response(message.id, payload))
            } else {
                Some(Message::new_notification(payload))
            }
        } else {
            None
        };
        
        Ok(response)
    }
    
    /// Handle chat message
    fn handle_chat_message(&mut self, message: String) -> CommunicationResult<Option<MessagePayload>> {
        info!("Processing chat message: {}", message);
        
        // Add message to chat history using the existing interface
        use crate::agent::chat_manager::{MessageRole, MessageContent};
        
        if let Err(e) = self.chat_manager.add_message(
            MessageRole::User, 
            MessageContent::Text(message.clone())
        ) {
            error!("Failed to add user message: {}", e);
            return Ok(Some(MessagePayload::Error {
                message: "Failed to process chat message".to_string(),
                details: Some(e.to_string()),
            }));
        }
        
        // Generate a simple response for now
        // TODO: Integrate with actual agent/LLM in future tasks
        let response_text = format!("**Agent:** I received your message: \"{}\"", message);
        
        if let Err(e) = self.chat_manager.add_message(
            MessageRole::Agent,
            MessageContent::Text(response_text.clone())
        ) {
            error!("Failed to add agent message: {}", e);
        }
        
        Ok(Some(MessagePayload::ChatResponse {
            message: response_text,
        }))
    }
    
    /// Handle new spec creation
    fn handle_new_spec(&mut self, feature_name: String) -> CommunicationResult<Option<MessagePayload>> {
        info!("Creating new spec: {}", feature_name);
        
        match self.spec_workflow.create_new_spec(feature_name.clone()) {
            Ok(_) => {
                let path = format!(".kiro/specs/{}", feature_name);
                Ok(Some(MessagePayload::SpecCreated { feature_name, path }))
            }
            Err(e) => {
                error!("Failed to create spec: {}", e);
                Ok(Some(MessagePayload::Error {
                    message: "Failed to create spec".to_string(),
                    details: Some(e.to_string()),
                }))
            }
        }
    }
    
    /// Handle opening existing spec
    fn handle_open_spec(&mut self, spec_name: String) -> CommunicationResult<Option<MessagePayload>> {
        info!("Opening spec: {}", spec_name);
        
        match self.spec_workflow.open_spec(spec_name.clone()) {
            Ok(_) => {
                // TODO: Determine current phase from spec state
                let phase = SpecPhase::Requirements; // Default for now
                Ok(Some(MessagePayload::SpecOpened { spec_name, phase }))
            }
            Err(e) => {
                error!("Failed to open spec: {}", e);
                Ok(Some(MessagePayload::Error {
                    message: "Failed to open spec".to_string(),
                    details: Some(e.to_string()),
                }))
            }
        }
    }
    
    /// Handle command approval
    fn handle_command_approval(&mut self, approved: bool) -> CommunicationResult<Option<MessagePayload>> {
        info!("Command approval: {}", approved);
        
        if approved {
            // TODO: Execute the pending command
            // For now, just acknowledge
            Ok(Some(MessagePayload::CommandExecution {
                command: "echo 'Command execution not yet implemented'".to_string(),
                output: CommandOutput {
                    stdout: "Command execution not yet implemented".to_string(),
                    stderr: String::new(),
                    exit_code: 0,
                    execution_time_ms: 0,
                },
            }))
        } else {
            info!("Command rejected by user");
            Ok(None)
        }
    }
    
    /// Handle file operations
    fn handle_file_operation(&mut self, operation_type: FileOperationType, path: String) -> CommunicationResult<Option<MessagePayload>> {
        info!("File operation: {:?} on {}", operation_type, path);
        
        // TODO: Implement actual file operations
        // For now, just acknowledge
        Ok(Some(MessagePayload::FileOperation {
            operation_type,
            path,
            status: OperationStatus::Completed,
            progress: Some(1.0),
        }))
    }
    
    /// Handle state saving
    fn handle_save_state(&mut self) -> CommunicationResult<Option<MessagePayload>> {
        info!("Saving state");
        
        // TODO: Implement state persistence
        // For now, just acknowledge
        Ok(None)
    }
    
    /// Handle state loading
    fn handle_load_state(&mut self) -> CommunicationResult<Option<MessagePayload>> {
        info!("Loading state");
        
        // TODO: Implement state loading
        // For now, just acknowledge
        Ok(None)
    }
    
    /// Handle window resize
    fn handle_resize(&mut self, width: u32, height: u32) -> CommunicationResult<Option<MessagePayload>> {
        info!("Handling resize: {}x{}", width, height);
        
        // TODO: Implement resize handling
        // For now, just acknowledge
        Ok(None)
    }
    
    /// Get chat history
    pub fn get_chat_history(&self) -> Vec<ChatHistoryEntry> {
        // TODO: Convert from internal chat history format
        // For now, return empty
        vec![]
    }
}

impl Default for MessageHandler {
    fn default() -> Self {
        Self::new().expect("Failed to create default MessageHandler")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ping_pong() {
        let mut handler = MessageHandler::new().unwrap();
        let ping = Message::new_request(MessagePayload::Ping);
        
        let response = handler.handle_message(ping).unwrap();
        assert!(response.is_some());
        
        if let Some(msg) = response {
            assert!(matches!(msg.payload, MessagePayload::Pong));
        }
    }
    
    #[test]
    fn test_chat_message() {
        let mut handler = MessageHandler::new().unwrap();
        let chat_msg = Message::new_request(MessagePayload::ChatMessage {
            message: "Hello".to_string(),
        });
        
        let response = handler.handle_message(chat_msg).unwrap();
        assert!(response.is_some());
        
        if let Some(msg) = response {
            if let MessagePayload::ChatResponse { message } = msg.payload {
                assert!(message.contains("Hello"));
            } else {
                panic!("Expected ChatResponse");
            }
        }
    }
    
    #[test]
    fn test_agent_open_close() {
        let mut handler = MessageHandler::new().unwrap();
        
        let open_msg = Message::new_notification(MessagePayload::OpenAgent);
        let response = handler.handle_message(open_msg).unwrap();
        assert!(response.is_some());
        
        if let Some(msg) = response {
            assert!(matches!(msg.payload, MessagePayload::AgentOpened));
        }
        
        let close_msg = Message::new_notification(MessagePayload::CloseAgent);
        let response = handler.handle_message(close_msg).unwrap();
        assert!(response.is_some());
        
        if let Some(msg) = response {
            assert!(matches!(msg.payload, MessagePayload::AgentClosed));
        }
    }
}