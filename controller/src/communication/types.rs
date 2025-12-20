use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Unique identifier for correlating requests and responses
pub type MessageId = String;

/// Base message structure for all communication between Lua and Rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique identifier for this message
    pub id: MessageId,
    /// Type of message (request, response, notification)
    pub message_type: MessageType,
    /// Timestamp when message was created
    pub timestamp: DateTime<Utc>,
    /// Message payload
    pub payload: MessagePayload,
}

/// Type of message being sent
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessageType {
    /// Request expecting a response
    Request { correlation_id: Option<MessageId> },
    /// Response to a previous request
    Response { correlation_id: MessageId },
    /// One-way notification (no response expected)
    Notification,
}

/// All possible message payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MessagePayload {
    // System messages
    Ping,
    Pong,
    Startup { status: String },
    Shutdown,
    Error { message: String, details: Option<String> },
    
    // Chat operations
    ChatMessage { message: String },
    ChatResponse { message: String },
    ChatHistory { messages: Vec<ChatHistoryEntry> },
    
    // Spec operations
    NewSpec { feature_name: String },
    OpenSpec { spec_name: String },
    SpecCreated { feature_name: String, path: String },
    SpecOpened { spec_name: String, phase: SpecPhase },
    SpecUpdate { spec_name: String, phase: SpecPhase, action: String },
    
    // Command operations
    CommandProposal { 
        command: String, 
        working_directory: String, 
        description: String,
        risk_level: RiskLevel 
    },
    CommandApproval { approved: bool },
    CommandExecution { 
        command: String, 
        output: CommandOutput 
    },
    
    // File operations
    FileRead { 
        path: String, 
        progress: f32, 
        status: OperationStatus 
    },
    FileWrite { 
        path: String, 
        content_preview: String, 
        status: OperationStatus 
    },
    FileOperation { 
        operation_type: FileOperationType,
        path: String,
        status: OperationStatus,
        progress: Option<f32>
    },
    
    // Window operations
    WindowCreate { 
        window_type: WindowType, 
        config: WindowConfig 
    },
    WindowUpdate { 
        window_type: WindowType, 
        content: Option<Vec<String>>, 
        cursor: Option<(usize, usize)> 
    },
    WindowClose { window_type: WindowType },
    
    // Agent operations
    OpenAgent,
    CloseAgent,
    AgentOpened,
    AgentClosed,
    
    // State operations
    SaveState,
    LoadState,
    StateUpdate { key: String, value: serde_json::Value },
    
    // Resize operations
    HandleResize { width: u32, height: u32 },
}

/// Chat history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatHistoryEntry {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

/// Message role in chat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Agent,
    System,
}

/// Spec development phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpecPhase {
    Requirements,
    Design,
    Tasks,
    Implementation,
}

/// Risk level for command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

/// Command execution output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub execution_time_ms: u64,
}

/// File operation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationStatus {
    Starting,
    InProgress,
    Completed,
    Failed { error: String },
}

/// File operation type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileOperationType {
    Read,
    Write,
    Create,
    Delete,
    Move,
    Copy,
}

/// Window type for UI management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowType {
    Chat,
    Input,
    CommandApproval,
    SpecNavigation,
}

/// Window configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
    pub row: u32,
    pub col: u32,
    pub zindex: u32,
    pub border: String,
    pub title: Option<String>,
}

impl Message {
    /// Create a new request message
    pub fn new_request(payload: MessagePayload) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            message_type: MessageType::Request { correlation_id: None },
            timestamp: Utc::now(),
            payload,
        }
    }
    
    /// Create a response to a request
    pub fn new_response(correlation_id: MessageId, payload: MessagePayload) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            message_type: MessageType::Response { correlation_id },
            timestamp: Utc::now(),
            payload,
        }
    }
    
    /// Create a notification message
    pub fn new_notification(payload: MessagePayload) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            message_type: MessageType::Notification,
            timestamp: Utc::now(),
            payload,
        }
    }
    
    /// Get correlation ID if this is a request or response
    pub fn correlation_id(&self) -> Option<&MessageId> {
        match &self.message_type {
            MessageType::Request { correlation_id } => correlation_id.as_ref(),
            MessageType::Response { correlation_id } => Some(correlation_id),
            MessageType::Notification => None,
        }
    }
    
    /// Check if this message expects a response
    pub fn expects_response(&self) -> bool {
        matches!(self.message_type, MessageType::Request { .. })
    }
}

/// Error types for communication
#[derive(Debug, thiserror::Error)]
pub enum CommunicationError {
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Invalid message format: {0}")]
    InvalidFormat(String),
    
    #[error("Message timeout: {0}")]
    Timeout(String),
    
    #[error("Unknown message type: {0}")]
    UnknownMessageType(String),
    
    #[error("Correlation error: {0}")]
    Correlation(String),
}

pub type CommunicationResult<T> = Result<T, CommunicationError>;

/// Message correlation tracker for async operations
#[derive(Debug, Default)]
pub struct MessageCorrelator {
    pending_requests: HashMap<MessageId, PendingRequest>,
}

#[derive(Debug)]
pub struct PendingRequest {
    pub timestamp: DateTime<Utc>,
    pub payload: MessagePayload,
}

impl MessageCorrelator {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Track a pending request
    pub fn track_request(&mut self, message: &Message) {
        if let MessageType::Request { .. } = &message.message_type {
            self.pending_requests.insert(
                message.id.clone(),
                PendingRequest {
                    timestamp: message.timestamp,
                    payload: message.payload.clone(),
                }
            );
        }
    }
    
    /// Complete a request with a response
    pub fn complete_request(&mut self, correlation_id: &MessageId) -> Option<PendingRequest> {
        self.pending_requests.remove(correlation_id)
    }
    
    /// Get all pending requests (for cleanup/timeout handling)
    pub fn pending_requests(&self) -> &HashMap<MessageId, PendingRequest> {
        &self.pending_requests
    }
    
    /// Clean up old pending requests
    pub fn cleanup_old_requests(&mut self, max_age_seconds: i64) {
        let cutoff = Utc::now() - chrono::Duration::seconds(max_age_seconds);
        self.pending_requests.retain(|_, req| req.timestamp > cutoff);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_message_creation() {
        let payload = MessagePayload::Ping;
        let message = Message::new_request(payload);
        
        assert!(message.expects_response());
        assert!(!message.id.is_empty());
        assert!(matches!(message.payload, MessagePayload::Ping));
    }
    
    #[test]
    fn test_message_correlation() {
        let request = Message::new_request(MessagePayload::Ping);
        let response = Message::new_response(request.id.clone(), MessagePayload::Pong);
        
        assert_eq!(response.correlation_id(), Some(&request.id));
        assert!(!response.expects_response());
    }
    
    #[test]
    fn test_message_correlator() {
        let mut correlator = MessageCorrelator::new();
        let request = Message::new_request(MessagePayload::Ping);
        
        correlator.track_request(&request);
        assert_eq!(correlator.pending_requests().len(), 1);
        
        let completed = correlator.complete_request(&request.id);
        assert!(completed.is_some());
        assert_eq!(correlator.pending_requests().len(), 0);
    }
    
    #[test]
    fn test_serialization() {
        let message = Message::new_notification(MessagePayload::ChatMessage {
            message: "Hello, world!".to_string(),
        });
        
        let json = serde_json::to_string(&message).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        
        assert_eq!(message.id, deserialized.id);
        if let MessagePayload::ChatMessage { message: msg } = deserialized.payload {
            assert_eq!(msg, "Hello, world!");
        } else {
            panic!("Wrong payload type");
        }
    }
}