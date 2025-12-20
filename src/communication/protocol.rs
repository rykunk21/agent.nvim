use super::types::*;
use std::io::{self, BufRead, BufReader, Write};
use log::{debug, error, warn};

/// JSON-based communication protocol for Lua-Rust communication
pub struct JsonProtocol {
    correlator: MessageCorrelator,
}

impl JsonProtocol {
    pub fn new() -> Self {
        Self {
            correlator: MessageCorrelator::new(),
        }
    }
    
    /// Serialize a message to JSON string
    pub fn serialize_message(&self, message: &Message) -> CommunicationResult<String> {
        let json = serde_json::to_string(message)?;
        debug!("Serialized message: {}", json);
        Ok(json)
    }
    
    /// Deserialize a JSON string to a message
    pub fn deserialize_message(&self, json: &str) -> CommunicationResult<Message> {
        debug!("Deserializing message: {}", json);
        
        // First try to parse as our new Message format
        if let Ok(message) = serde_json::from_str::<Message>(json) {
            return Ok(message);
        }
        
        // Fallback: try to parse legacy format for backward compatibility
        if let Ok(legacy) = serde_json::from_str::<LegacyMessage>(json) {
            return Ok(self.convert_legacy_message(legacy));
        }
        
        Err(CommunicationError::InvalidFormat(
            format!("Could not parse message: {}", json)
        ))
    }
    
    /// Send a message through stdout (for Rust backend to Lua frontend)
    pub fn send_message(&mut self, message: Message) -> CommunicationResult<()> {
        // Track request for correlation if needed
        if message.expects_response() {
            self.correlator.track_request(&message);
        }
        
        let json = self.serialize_message(&message)?;
        println!("{}", json);
        io::stdout().flush()?;
        
        debug!("Sent message: {}", message.id);
        Ok(())
    }
    
    /// Read messages from stdin (for Rust backend from Lua frontend)
    pub fn read_messages<F>(&mut self, mut handler: F) -> CommunicationResult<()>
    where
        F: FnMut(Message) -> CommunicationResult<Option<Message>>,
    {
        let stdin = io::stdin();
        let reader = BufReader::new(stdin.lock());
        
        for line in reader.lines() {
            match line {
                Ok(json_line) => {
                    if json_line.trim().is_empty() {
                        continue;
                    }
                    
                    match self.deserialize_message(&json_line) {
                        Ok(message) => {
                            debug!("Received message: {}", message.id);
                            
                            // Handle response correlation
                            if let MessageType::Response { correlation_id } = &message.message_type {
                                if let Some(_pending) = self.correlator.complete_request(correlation_id) {
                                    debug!("Completed request: {}", correlation_id);
                                } else {
                                    warn!("Received response for unknown request: {}", correlation_id);
                                }
                            }
                            
                            // Call the handler
                            match handler(message) {
                                Ok(Some(response)) => {
                                    if let Err(e) = self.send_message(response) {
                                        error!("Failed to send response: {}", e);
                                    }
                                }
                                Ok(None) => {
                                    // No response needed
                                }
                                Err(e) => {
                                    error!("Handler error: {}", e);
                                    
                                    // Send error response if this was a request
                                    // Note: We'd need the original message ID here for proper correlation
                                    let error_message = Message::new_notification(
                                        MessagePayload::Error {
                                            message: format!("Handler error: {}", e),
                                            details: None,
                                        }
                                    );
                                    
                                    if let Err(send_err) = self.send_message(error_message) {
                                        error!("Failed to send error response: {}", send_err);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to deserialize message: {}", e);
                            
                            let error_message = Message::new_notification(
                                MessagePayload::Error {
                                    message: format!("Invalid message format: {}", e),
                                    details: Some(json_line),
                                }
                            );
                            
                            if let Err(send_err) = self.send_message(error_message) {
                                error!("Failed to send error response: {}", send_err);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Error reading from stdin: {}", e);
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    /// Clean up old pending requests to prevent memory leaks
    pub fn cleanup_old_requests(&mut self) {
        self.correlator.cleanup_old_requests(300); // 5 minutes
    }
    
    /// Get pending request count (for monitoring)
    pub fn pending_request_count(&self) -> usize {
        self.correlator.pending_requests().len()
    }
}

/// Legacy message format for backward compatibility
#[derive(serde::Deserialize)]
struct LegacyMessage {
    #[serde(rename = "type")]
    msg_type: String,
    data: Option<serde_json::Value>,
}

impl JsonProtocol {
    /// Convert legacy message format to new format
    fn convert_legacy_message(&self, legacy: LegacyMessage) -> Message {
        let payload = match legacy.msg_type.as_str() {
            "ping" => MessagePayload::Ping,
            "pong" => MessagePayload::Pong,
            "startup" => {
                let status = legacy.data
                    .as_ref()
                    .and_then(|d| d.get("status"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("ready")
                    .to_string();
                MessagePayload::Startup { status }
            },
            "open_agent" => MessagePayload::OpenAgent,
            "close_agent" => MessagePayload::CloseAgent,
            "agent_opened" => MessagePayload::AgentOpened,
            "agent_closed" => MessagePayload::AgentClosed,
            "chat_message" => {
                let message = legacy.data
                    .as_ref()
                    .and_then(|d| d.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("")
                    .to_string();
                MessagePayload::ChatMessage { message }
            }
            "chat_response" => {
                let message = legacy.data
                    .as_ref()
                    .and_then(|d| d.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("")
                    .to_string();
                MessagePayload::ChatResponse { message }
            }
            "new_spec" => {
                let feature_name = legacy.data
                    .as_ref()
                    .and_then(|d| d.get("feature_name"))
                    .and_then(|f| f.as_str())
                    .unwrap_or("new-feature")
                    .to_string();
                MessagePayload::NewSpec { feature_name }
            }
            "open_spec" => {
                let spec_name = legacy.data
                    .as_ref()
                    .and_then(|d| d.get("spec_name"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                MessagePayload::OpenSpec { spec_name }
            }
            "spec_created" => {
                let feature_name = legacy.data
                    .as_ref()
                    .and_then(|d| d.get("feature_name"))
                    .and_then(|f| f.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let path = format!(".kiro/specs/{}", feature_name);
                MessagePayload::SpecCreated { feature_name, path }
            }
            "spec_opened" => {
                let spec_name = legacy.data
                    .as_ref()
                    .and_then(|d| d.get("spec_name"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                MessagePayload::SpecOpened {
                    spec_name,
                    phase: SpecPhase::Requirements, // Default phase
                }
            }
            "save_state" => MessagePayload::SaveState,
            "handle_resize" => {
                let width = legacy.data
                    .as_ref()
                    .and_then(|d| d.get("width"))
                    .and_then(|w| w.as_u64())
                    .unwrap_or(80) as u32;
                let height = legacy.data
                    .as_ref()
                    .and_then(|d| d.get("height"))
                    .and_then(|h| h.as_u64())
                    .unwrap_or(24) as u32;
                MessagePayload::HandleResize { width, height }
            }
            _ => MessagePayload::Error {
                message: format!("Unknown legacy message type: {}", legacy.msg_type),
                details: None,
            },
        };
        
        Message::new_notification(payload)
    }
}

impl Default for JsonProtocol {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_message_serialization() {
        let protocol = JsonProtocol::new();
        let message = Message::new_request(MessagePayload::Ping);
        
        let json = protocol.serialize_message(&message).unwrap();
        let deserialized = protocol.deserialize_message(&json).unwrap();
        
        assert_eq!(message.id, deserialized.id);
        assert!(matches!(deserialized.payload, MessagePayload::Ping));
    }
    
    #[test]
    fn test_legacy_message_conversion() {
        let protocol = JsonProtocol::new();
        let legacy_json = r#"{"type": "ping"}"#;
        
        let message = protocol.deserialize_message(legacy_json).unwrap();
        assert!(matches!(message.payload, MessagePayload::Ping));
    }
    
    #[test]
    fn test_chat_message_conversion() {
        let protocol = JsonProtocol::new();
        let legacy_json = r#"{"type": "chat_message", "data": {"message": "Hello"}}"#;
        
        let message = protocol.deserialize_message(legacy_json).unwrap();
        if let MessagePayload::ChatMessage { message: msg } = message.payload {
            assert_eq!(msg, "Hello");
        } else {
            panic!("Wrong payload type");
        }
    }
}