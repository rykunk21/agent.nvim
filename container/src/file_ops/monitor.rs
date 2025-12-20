use anyhow::Result;
use log::info;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Type of file operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OperationType {
    Read,
    Write,
    Delete,
    Create,
    Modify,
    Move,
}

/// Status of a file operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OperationStatus {
    Pending,
    InProgress,
    Completed,
    Failed { error: String },
}

/// Visual block for file operation display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationBlock {
    pub id: String,
    pub operation_type: OperationType,
    pub file_path: String,
    pub status: OperationStatus,
    pub progress: f32,
    pub bytes_processed: u64,
    pub total_bytes: u64,
    pub started_at: u64,
    pub completed_at: Option<u64>,
}

impl OperationBlock {
    /// Create a new operation block
    pub fn new(operation_type: OperationType, file_path: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        OperationBlock {
            id: Uuid::new_v4().to_string(),
            operation_type,
            file_path,
            status: OperationStatus::Pending,
            progress: 0.0,
            bytes_processed: 0,
            total_bytes: 0,
            started_at: now,
            completed_at: None,
        }
    }

    /// Update progress
    pub fn update_progress(&mut self, bytes_processed: u64, total_bytes: u64) {
        self.bytes_processed = bytes_processed;
        self.total_bytes = total_bytes;
        if total_bytes > 0 {
            self.progress = (bytes_processed as f32 / total_bytes as f32) * 100.0;
        }
    }

    /// Mark as completed
    pub fn complete(&mut self) {
        self.status = OperationStatus::Completed;
        self.progress = 100.0;
        self.completed_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );
    }

    /// Mark as failed
    pub fn fail(&mut self, error: String) {
        self.status = OperationStatus::Failed { error };
        self.completed_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );
    }

    /// Get duration in seconds
    pub fn duration_secs(&self) -> u64 {
        let end_time = self.completed_at.unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        });
        end_time.saturating_sub(self.started_at)
    }
}

/// File operation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOperation {
    pub id: String,
    pub operation_type: OperationType,
    pub file_path: String,
    pub status: OperationStatus,
    pub bytes_processed: u64,
    pub total_bytes: u64,
    pub started_at: u64,
    pub completed_at: Option<u64>,
    pub error: Option<String>,
}

/// File operation monitor for tracking file operations
pub struct FileOperationMonitor {
    operations: HashMap<String, OperationBlock>,
    completed_operations: Vec<FileOperation>,
    max_history: usize,
}

impl FileOperationMonitor {
    pub fn new() -> Self {
        FileOperationMonitor {
            operations: HashMap::new(),
            completed_operations: Vec::new(),
            max_history: 100,
        }
    }

    /// Start a new file operation
    pub fn start_operation(
        &mut self,
        operation_type: OperationType,
        file_path: String,
    ) -> Result<String> {
        let mut block = OperationBlock::new(operation_type, file_path);
        block.status = OperationStatus::InProgress;

        let block_id = block.id.clone();
        self.operations.insert(block_id.clone(), block);

        info!("Started file operation: {}", block_id);
        Ok(block_id)
    }

    /// Update operation progress
    pub fn update_progress(
        &mut self,
        operation_id: &str,
        bytes_processed: u64,
        total_bytes: u64,
    ) -> Result<()> {
        let block = self
            .operations
            .get_mut(operation_id)
            .ok_or_else(|| anyhow::anyhow!("Operation not found: {}", operation_id))?;

        block.update_progress(bytes_processed, total_bytes);
        info!(
            "Updated progress for operation {}: {:.1}%",
            operation_id, block.progress
        );
        Ok(())
    }

    /// Complete an operation
    pub fn complete_operation(&mut self, operation_id: &str) -> Result<()> {
        let mut block = self
            .operations
            .remove(operation_id)
            .ok_or_else(|| anyhow::anyhow!("Operation not found: {}", operation_id))?;

        block.complete();

        // Record in history
        let operation = FileOperation {
            id: block.id.clone(),
            operation_type: block.operation_type,
            file_path: block.file_path,
            status: block.status,
            bytes_processed: block.bytes_processed,
            total_bytes: block.total_bytes,
            started_at: block.started_at,
            completed_at: block.completed_at,
            error: None,
        };

        self.completed_operations.push(operation);
        if self.completed_operations.len() > self.max_history {
            self.completed_operations.remove(0);
        }

        info!("Completed operation: {}", operation_id);
        Ok(())
    }

    /// Fail an operation
    pub fn fail_operation(&mut self, operation_id: &str, error: String) -> Result<()> {
        let mut block = self
            .operations
            .remove(operation_id)
            .ok_or_else(|| anyhow::anyhow!("Operation not found: {}", operation_id))?;

        block.fail(error.clone());

        // Record in history
        let operation = FileOperation {
            id: block.id.clone(),
            operation_type: block.operation_type,
            file_path: block.file_path,
            status: block.status,
            bytes_processed: block.bytes_processed,
            total_bytes: block.total_bytes,
            started_at: block.started_at,
            completed_at: block.completed_at,
            error: Some(error),
        };

        self.completed_operations.push(operation);
        if self.completed_operations.len() > self.max_history {
            self.completed_operations.remove(0);
        }

        info!("Failed operation: {}", operation_id);
        Ok(())
    }

    /// Get operation block
    pub fn get_operation(&self, operation_id: &str) -> Result<OperationBlock> {
        self.operations
            .get(operation_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Operation not found: {}", operation_id))
    }

    /// Get all active operations
    pub fn get_active_operations(&self) -> Vec<OperationBlock> {
        self.operations.values().cloned().collect()
    }

    /// Get operation history
    pub fn get_history(&self) -> Vec<FileOperation> {
        self.completed_operations.clone()
    }

    /// Clear operation history
    pub fn clear_history(&mut self) {
        self.completed_operations.clear();
    }

    /// Get operation statistics
    pub fn get_statistics(&self) -> OperationStatistics {
        let total_operations = self.completed_operations.len();
        let successful_operations = self
            .completed_operations
            .iter()
            .filter(|op| matches!(op.status, OperationStatus::Completed))
            .count();
        let failed_operations = self
            .completed_operations
            .iter()
            .filter(|op| matches!(op.status, OperationStatus::Failed { .. }))
            .count();

        let total_bytes_processed: u64 = self
            .completed_operations
            .iter()
            .map(|op| op.bytes_processed)
            .sum();

        OperationStatistics {
            total_operations,
            successful_operations,
            failed_operations,
            total_bytes_processed,
            active_operations: self.operations.len(),
        }
    }
}

impl Default for FileOperationMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// File operation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationStatistics {
    pub total_operations: usize,
    pub successful_operations: usize,
    pub failed_operations: usize,
    pub total_bytes_processed: u64,
    pub active_operations: usize,
}
