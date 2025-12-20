use log::{info, error, debug, warn};
use std::time::Duration;
use tokio::time::timeout;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// Context data from Neovim
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeovimContext {
    pub current_buffer: Option<BufferInfo>,
    pub cursor_position: Option<CursorPosition>,
    pub open_files: Vec<FileInfo>,
    pub project_structure: Option<ProjectStructure>,
    pub recent_edits: Vec<EditInfo>,
    pub diagnostics: Vec<DiagnosticInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferInfo {
    pub path: String,
    pub content: String,
    pub filetype: String,
    pub modified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub filetype: String,
    pub modified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStructure {
    pub root_path: String,
    pub file_paths: Vec<String>,
    pub directories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditInfo {
    pub file_path: String,
    pub change_type: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticInfo {
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub severity: String,
    pub message: String,
}

/// Stream message for bidirectional communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMessage {
    pub id: String,
    pub message_type: String,
    pub payload: Vec<u8>,
    pub timestamp: i64,
}

/// gRPC client for container communication
pub struct GrpcClient {
    /// Server endpoint
    endpoint: String,
    /// Connection timeout
    timeout: Duration,
    /// Maximum retries
    max_retries: u32,
    /// Current retry count
    retry_count: Arc<RwLock<u32>>,
    /// Connection state
    connected: Arc<RwLock<bool>>,
    /// Message queue for bidirectional streaming
    message_queue: Arc<RwLock<Vec<StreamMessage>>>,
}

impl GrpcClient {
    /// Create a new gRPC client
    pub fn new(endpoint: String) -> Self {
        info!("Creating gRPC client for endpoint: {}", endpoint);
        
        Self {
            endpoint,
            timeout: Duration::from_secs(30),
            max_retries: 3,
            retry_count: Arc::new(RwLock::new(0)),
            connected: Arc::new(RwLock::new(false)),
            message_queue: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get endpoint
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Set connection timeout
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Set maximum retries
    pub fn set_max_retries(&mut self, max_retries: u32) {
        self.max_retries = max_retries;
    }

    /// Connect to the container
    pub async fn connect(&self) -> Result<(), String> {
        info!("Connecting to container at {}", self.endpoint);
        
        // Try to establish connection with retries
        let mut attempt = 0;
        loop {
            match self.try_connect().await {
                Ok(_) => {
                    *self.connected.write().await = true;
                    *self.retry_count.write().await = 0;
                    info!("Successfully connected to container");
                    return Ok(());
                }
                Err(e) => {
                    attempt += 1;
                    if attempt >= self.max_retries {
                        let error_msg = format!("Failed to connect after {} attempts: {}", attempt, e);
                        error!("{}", error_msg);
                        return Err(error_msg);
                    }
                    
                    let backoff = Duration::from_secs(2_u64.pow(attempt - 1));
                    warn!("Connection attempt {} failed, retrying in {:?}: {}", attempt, backoff, e);
                    tokio::time::sleep(backoff).await;
                }
            }
        }
    }

    /// Try to connect once
    async fn try_connect(&self) -> Result<(), String> {
        // Parse endpoint to extract host and port
        let endpoint = self.endpoint.replace("http://", "").replace("https://", "");
        
        // Try TCP connection
        match timeout(self.timeout, tokio::net::TcpStream::connect(&endpoint)).await {
            Ok(Ok(_)) => {
                debug!("TCP connection successful to {}", endpoint);
                Ok(())
            }
            Ok(Err(e)) => {
                Err(format!("TCP connection failed: {}", e))
            }
            Err(_) => {
                Err("Connection timeout".to_string())
            }
        }
    }

    /// Disconnect from the container
    pub async fn disconnect(&self) -> Result<(), String> {
        info!("Disconnecting from container");
        *self.connected.write().await = false;
        Ok(())
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// Send request to container
    pub async fn send_request(&self, request_data: Vec<u8>) -> Result<Vec<u8>, String> {
        if !self.is_connected().await {
            return Err("Not connected to container".to_string());
        }

        debug!("Sending request to container ({} bytes)", request_data.len());
        
        // In a real implementation, this would send the request via gRPC
        // For now, return a simple response
        Ok(vec![])
    }

    /// Send request with context data
    pub async fn send_request_with_context(&self, request_data: Vec<u8>, context: NeovimContext) -> Result<Vec<u8>, String> {
        if !self.is_connected().await {
            return Err("Not connected to container".to_string());
        }

        debug!("Sending request with context to container ({} bytes)", request_data.len());
        
        // Serialize context
        let context_json = serde_json::to_vec(&context)
            .map_err(|e| format!("Failed to serialize context: {}", e))?;
        
        debug!("Context serialized ({} bytes)", context_json.len());
        
        // In a real implementation, this would send both request and context via gRPC
        Ok(vec![])
    }

    /// Queue message for bidirectional streaming
    pub async fn queue_message(&self, message: StreamMessage) -> Result<(), String> {
        let mut queue = self.message_queue.write().await;
        queue.push(message);
        debug!("Message queued, queue size: {}", queue.len());
        Ok(())
    }

    /// Get next message from queue
    pub async fn dequeue_message(&self) -> Option<StreamMessage> {
        let mut queue = self.message_queue.write().await;
        if queue.is_empty() {
            None
        } else {
            Some(queue.remove(0))
        }
    }

    /// Get queue size
    pub async fn queue_size(&self) -> usize {
        let queue = self.message_queue.read().await;
        queue.len()
    }

    /// Clear message queue
    pub async fn clear_queue(&self) {
        let mut queue = self.message_queue.write().await;
        queue.clear();
    }

    /// Stream responses from container
    pub async fn stream_responses(&self) -> Result<ResponseStream, String> {
        if !self.is_connected().await {
            return Err("Not connected to container".to_string());
        }

        debug!("Starting response stream from container");
        
        Ok(ResponseStream::new())
    }

    /// Health check
    pub async fn health_check(&self) -> Result<HealthCheckResponse, String> {
        if !self.is_connected().await {
            return Err("Not connected to container".to_string());
        }

        debug!("Performing health check on container");
        
        match timeout(self.timeout, self.perform_health_check()).await {
            Ok(Ok(response)) => {
                *self.retry_count.write().await = 0;
                Ok(response)
            }
            Ok(Err(e)) => {
                let retry_count = *self.retry_count.read().await;
                if retry_count < self.max_retries {
                    *self.retry_count.write().await = retry_count + 1;
                }
                Err(e)
            }
            Err(_) => {
                Err("Health check timeout".to_string())
            }
        }
    }

    /// Perform actual health check
    async fn perform_health_check(&self) -> Result<HealthCheckResponse, String> {
        // In a real implementation, this would call the gRPC health check endpoint
        Ok(HealthCheckResponse {
            status: "healthy".to_string(),
            uptime_seconds: 0,
            version: "0.1.0".to_string(),
        })
    }

    /// Get retry count
    pub async fn retry_count(&self) -> u32 {
        *self.retry_count.read().await
    }

    /// Reset retry count
    pub async fn reset_retry_count(&self) {
        *self.retry_count.write().await = 0;
    }
}

/// Health check response
#[derive(Debug, Clone)]
pub struct HealthCheckResponse {
    pub status: String,
    pub uptime_seconds: i64,
    pub version: String,
}

/// Response stream from container
pub struct ResponseStream {
    messages: Arc<RwLock<Vec<Vec<u8>>>>,
    current_index: Arc<RwLock<usize>>,
}

impl ResponseStream {
    /// Create a new response stream
    pub fn new() -> Self {
        Self {
            messages: Arc::new(RwLock::new(Vec::new())),
            current_index: Arc::new(RwLock::new(0)),
        }
    }

    /// Add message to stream
    pub async fn add_message(&self, message: Vec<u8>) {
        let mut messages = self.messages.write().await;
        messages.push(message);
    }

    /// Get next response
    pub async fn next(&mut self) -> Option<Result<Vec<u8>, String>> {
        let mut index = self.current_index.write().await;
        let messages = self.messages.read().await;
        
        if *index < messages.len() {
            let message = messages[*index].clone();
            *index += 1;
            Some(Ok(message))
        } else {
            None
        }
    }

    /// Get all messages
    pub async fn all_messages(&self) -> Vec<Vec<u8>> {
        let messages = self.messages.read().await;
        messages.clone()
    }

    /// Clear stream
    pub async fn clear(&self) {
        let mut messages = self.messages.write().await;
        messages.clear();
        let mut index = self.current_index.write().await;
        *index = 0;
    }

    /// Get message count
    pub async fn message_count(&self) -> usize {
        let messages = self.messages.read().await;
        messages.len()
    }
}

impl Default for ResponseStream {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grpc_client_creation() {
        let client = GrpcClient::new("http://localhost:50051".to_string());
        assert_eq!(client.endpoint(), "http://localhost:50051");
    }

    #[tokio::test]
    async fn test_client_timeout_setting() {
        let mut client = GrpcClient::new("http://localhost:50051".to_string());
        client.set_timeout(Duration::from_secs(60));
        assert_eq!(client.timeout, Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_client_retry_setting() {
        let mut client = GrpcClient::new("http://localhost:50051".to_string());
        client.set_max_retries(5);
        assert_eq!(client.max_retries, 5);
    }

    #[tokio::test]
    async fn test_client_connection_state() {
        let client = GrpcClient::new("http://localhost:50051".to_string());
        assert!(!client.is_connected().await);
    }

    #[tokio::test]
    async fn test_retry_count() {
        let client = GrpcClient::new("http://localhost:50051".to_string());
        assert_eq!(client.retry_count().await, 0);
        
        client.reset_retry_count().await;
        assert_eq!(client.retry_count().await, 0);
    }

    #[tokio::test]
    async fn test_send_request_not_connected() {
        let client = GrpcClient::new("http://localhost:50051".to_string());
        let result = client.send_request(vec![1, 2, 3]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stream_responses_not_connected() {
        let client = GrpcClient::new("http://localhost:50051".to_string());
        let result = client.stream_responses().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_health_check_not_connected() {
        let client = GrpcClient::new("http://localhost:50051".to_string());
        let result = client.health_check().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_message_queueing() {
        let client = GrpcClient::new("http://localhost:50051".to_string());
        
        let message = StreamMessage {
            id: "test-1".to_string(),
            message_type: "request".to_string(),
            payload: vec![1, 2, 3],
            timestamp: 0,
        };
        
        assert!(client.queue_message(message.clone()).await.is_ok());
        assert_eq!(client.queue_size().await, 1);
        
        let dequeued = client.dequeue_message().await;
        assert!(dequeued.is_some());
        assert_eq!(client.queue_size().await, 0);
    }

    #[tokio::test]
    async fn test_context_serialization() {
        let context = NeovimContext {
            current_buffer: Some(BufferInfo {
                path: "/test/file.rs".to_string(),
                content: "fn main() {}".to_string(),
                filetype: "rust".to_string(),
                modified: false,
            }),
            cursor_position: Some(CursorPosition {
                line: 1,
                column: 5,
            }),
            open_files: vec![],
            project_structure: None,
            recent_edits: vec![],
            diagnostics: vec![],
        };
        
        let json = serde_json::to_vec(&context).unwrap();
        assert!(!json.is_empty());
        
        let deserialized: NeovimContext = serde_json::from_slice(&json).unwrap();
        assert!(deserialized.current_buffer.is_some());
    }

    #[tokio::test]
    async fn test_response_stream() {
        let stream = ResponseStream::new();
        
        stream.add_message(vec![1, 2, 3]).await;
        stream.add_message(vec![4, 5, 6]).await;
        
        assert_eq!(stream.message_count().await, 2);
        
        let messages = stream.all_messages().await;
        assert_eq!(messages.len(), 2);
    }

    #[tokio::test]
    async fn test_response_stream_iteration() {
        let mut stream = ResponseStream::new();
        
        stream.add_message(vec![1, 2, 3]).await;
        stream.add_message(vec![4, 5, 6]).await;
        
        let msg1 = stream.next().await;
        assert!(msg1.is_some());
        
        let msg2 = stream.next().await;
        assert!(msg2.is_some());
        
        let msg3 = stream.next().await;
        assert!(msg3.is_none());
    }

    #[tokio::test]
    async fn test_queue_clear() {
        let client = GrpcClient::new("http://localhost:50051".to_string());
        
        let message = StreamMessage {
            id: "test-1".to_string(),
            message_type: "request".to_string(),
            payload: vec![1, 2, 3],
            timestamp: 0,
        };
        
        client.queue_message(message).await.unwrap();
        assert_eq!(client.queue_size().await, 1);
        
        client.clear_queue().await;
        assert_eq!(client.queue_size().await, 0);
    }
}
