use log::{info, error, debug, warn};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use chrono::Utc;

/// gRPC server for Lua frontend communication
pub struct GrpcServer {
    /// Server address
    address: String,
    /// Active connections
    connections: Arc<RwLock<HashMap<String, ConnectionState>>>,
    /// Server start time for uptime calculation
    start_time: chrono::DateTime<chrono::Utc>,
    /// Health status
    healthy: Arc<RwLock<bool>>,
}

/// Connection state tracking
#[derive(Debug, Clone)]
pub struct ConnectionState {
    pub connection_id: String,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub message_count: u64,
}

impl GrpcServer {
    /// Create a new gRPC server
    pub fn new(address: String) -> Self {
        info!("Creating gRPC server on {}", address);
        
        Self {
            address,
            connections: Arc::new(RwLock::new(HashMap::new())),
            start_time: Utc::now(),
            healthy: Arc::new(RwLock::new(true)),
        }
    }

    /// Get server address
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Start the gRPC server
    pub async fn start(&self) -> Result<(), String> {
        info!("Starting gRPC server on {}", self.address);
        
        // In a real implementation, this would start the actual gRPC server
        // For now, we'll set up the basic infrastructure
        
        *self.healthy.write().await = true;
        info!("gRPC server started successfully");
        
        Ok(())
    }

    /// Stop the gRPC server
    pub async fn stop(&self) -> Result<(), String> {
        info!("Stopping gRPC server");
        
        *self.healthy.write().await = false;
        
        // Close all connections
        let mut connections = self.connections.write().await;
        connections.clear();
        
        info!("gRPC server stopped");
        Ok(())
    }

    /// Register a new connection
    pub async fn register_connection(&self, connection_id: String) -> Result<(), String> {
        let state = ConnectionState {
            connection_id: connection_id.clone(),
            connected_at: Utc::now(),
            last_activity: Utc::now(),
            message_count: 0,
        };

        let mut connections = self.connections.write().await;
        connections.insert(connection_id.clone(), state);
        
        debug!("Connection registered: {}", connection_id);
        Ok(())
    }

    /// Unregister a connection
    pub async fn unregister_connection(&self, connection_id: &str) -> Result<(), String> {
        let mut connections = self.connections.write().await;
        connections.remove(connection_id);
        
        debug!("Connection unregistered: {}", connection_id);
        Ok(())
    }

    /// Record message activity
    pub async fn record_message(&self, connection_id: &str) -> Result<(), String> {
        let mut connections = self.connections.write().await;
        
        if let Some(state) = connections.get_mut(connection_id) {
            state.last_activity = Utc::now();
            state.message_count += 1;
            debug!("Message recorded for connection: {}", connection_id);
            Ok(())
        } else {
            warn!("Message recorded for unknown connection: {}", connection_id);
            Err(format!("Unknown connection: {}", connection_id))
        }
    }

    /// Get connection state
    pub async fn get_connection_state(&self, connection_id: &str) -> Option<ConnectionState> {
        let connections = self.connections.read().await;
        connections.get(connection_id).cloned()
    }

    /// Get all active connections
    pub async fn get_active_connections(&self) -> Vec<ConnectionState> {
        let connections = self.connections.read().await;
        connections.values().cloned().collect()
    }

    /// Get server health status
    pub async fn is_healthy(&self) -> bool {
        *self.healthy.read().await
    }

    /// Get server uptime in seconds
    pub fn uptime_seconds(&self) -> i64 {
        let elapsed = Utc::now().signed_duration_since(self.start_time);
        elapsed.num_seconds()
    }

    /// Get connection count
    pub async fn connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    /// Clean up stale connections (no activity for specified duration)
    pub async fn cleanup_stale_connections(&self, max_idle_seconds: i64) -> usize {
        let mut connections = self.connections.write().await;
        let cutoff = Utc::now() - chrono::Duration::seconds(max_idle_seconds);
        
        let initial_count = connections.len();
        connections.retain(|_, state| state.last_activity > cutoff);
        let removed_count = initial_count - connections.len();
        
        if removed_count > 0 {
            info!("Cleaned up {} stale connections", removed_count);
        }
        
        removed_count
    }

    /// Handle incoming request from Lua frontend
    pub async fn handle_request(&self, connection_id: &str, request_data: Vec<u8>) -> Result<Vec<u8>, String> {
        // Record activity
        self.record_message(connection_id).await?;
        
        // In a real implementation, this would deserialize the request,
        // route it to the container via gRPC client, and return the response
        debug!("Handling request from connection: {}", connection_id);
        
        // For now, return a simple acknowledgment
        Ok(vec![])
    }

    /// Send response to Lua frontend
    pub async fn send_response(&self, connection_id: &str, response_data: Vec<u8>) -> Result<(), String> {
        // Record activity
        self.record_message(connection_id).await?;
        
        debug!("Sending response to connection: {}", connection_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_grpc_server_creation() {
        let server = GrpcServer::new("127.0.0.1:50051".to_string());
        assert_eq!(server.address(), "127.0.0.1:50051");
    }

    #[tokio::test]
    async fn test_server_start_stop() {
        let server = GrpcServer::new("127.0.0.1:50051".to_string());
        
        assert!(server.start().await.is_ok());
        assert!(server.is_healthy().await);
        
        assert!(server.stop().await.is_ok());
        assert!(!server.is_healthy().await);
    }

    #[tokio::test]
    async fn test_connection_registration() {
        let server = GrpcServer::new("127.0.0.1:50051".to_string());
        
        let conn_id = "test-connection-1".to_string();
        assert!(server.register_connection(conn_id.clone()).await.is_ok());
        assert_eq!(server.connection_count().await, 1);
        
        let state = server.get_connection_state(&conn_id).await;
        assert!(state.is_some());
        
        assert!(server.unregister_connection(&conn_id).await.is_ok());
        assert_eq!(server.connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_message_recording() {
        let server = GrpcServer::new("127.0.0.1:50051".to_string());
        let conn_id = "test-connection-1".to_string();
        
        server.register_connection(conn_id.clone()).await.unwrap();
        
        let initial_state = server.get_connection_state(&conn_id).await.unwrap();
        assert_eq!(initial_state.message_count, 0);
        
        server.record_message(&conn_id).await.unwrap();
        
        let updated_state = server.get_connection_state(&conn_id).await.unwrap();
        assert_eq!(updated_state.message_count, 1);
    }

    #[tokio::test]
    async fn test_uptime() {
        let server = GrpcServer::new("127.0.0.1:50051".to_string());
        
        let uptime = server.uptime_seconds();
        assert!(uptime >= 0);
    }

    #[tokio::test]
    async fn test_cleanup_stale_connections() {
        let server = GrpcServer::new("127.0.0.1:50051".to_string());
        
        let conn_id = "test-connection-1".to_string();
        server.register_connection(conn_id.clone()).await.unwrap();
        
        // Manually set last_activity to old time
        {
            let mut connections = server.connections.write().await;
            if let Some(state) = connections.get_mut(&conn_id) {
                state.last_activity = Utc::now() - chrono::Duration::seconds(100);
            }
        }
        
        let removed = server.cleanup_stale_connections(60).await;
        assert_eq!(removed, 1);
        assert_eq!(server.connection_count().await, 0);
    }
}
