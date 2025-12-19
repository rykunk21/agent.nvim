use crate::ui::VisualBlockManager;
use crate::utils::{PluginResult, PluginError, NeovimApiWrapper};
use neovim_lib::Neovim;
use std::time::{Instant, Duration};
use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;
use tokio::sync::mpsc;
use serde::{Serialize, Deserialize};

/// File operation types for tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileOperationType {
    Read,
    Write,
    Create,
    Delete,
    Move,
    Copy,
}

/// File operation metadata
#[derive(Debug, Clone)]
pub struct FileOperationMetadata {
    pub operation_type: FileOperationType,
    pub file_path: PathBuf,
    pub start_time: Instant,
    pub estimated_duration: Option<Duration>,
    pub file_size: Option<u64>,
    pub bytes_processed: u64,
    pub buffer_id: Option<i32>,
    pub requires_backup: bool,
}

/// File permission information
#[derive(Debug, Clone)]
pub struct FilePermissionInfo {
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
    pub exists: bool,
    pub is_directory: bool,
    pub parent_writable: bool,
}

/// Progress callback for file operations
pub type ProgressCallback = Box<dyn Fn(f32, u64, Option<u64>) + Send + Sync>;

/// Manages file operation tracking and visualization with Neovim integration
pub struct FileOperationsManager {
    pub active_operations: HashMap<String, FileOperationMetadata>,
    pub visual_manager: VisualBlockManager,
    pub progress_sender: Option<mpsc::UnboundedSender<FileProgressUpdate>>,
    pub auto_backup_enabled: bool,
    pub backup_directory: PathBuf,
    pub max_concurrent_operations: usize,
    pub buffer_integration_enabled: bool,
}

/// Progress update messages for async processing
#[derive(Debug, Clone)]
pub struct FileProgressUpdate {
    pub operation_id: String,
    pub progress: f32,
    pub bytes_processed: u64,
    pub total_bytes: Option<u64>,
    pub error: Option<String>,
}

impl FileOperationsManager {
    pub fn new() -> Self {
        FileOperationsManager {
            active_operations: HashMap::new(),
            visual_manager: VisualBlockManager::new(),
            progress_sender: None,
            auto_backup_enabled: true,
            backup_directory: PathBuf::from(".nvim-spec-agent-backups"),
            max_concurrent_operations: 5,
            buffer_integration_enabled: true,
        }
    }

    /// Initialize the file operations manager with Neovim integration
    pub fn initialize(&mut self, _neovim: &mut Neovim) -> PluginResult<()> {
        // Create backup directory if it doesn't exist
        if self.auto_backup_enabled && !self.backup_directory.exists() {
            fs::create_dir_all(&self.backup_directory)
                .map_err(|e| PluginError::filesystem(&format!("Failed to create backup directory: {}", e)))?;
        }

        // Initialize progress monitoring channel
        let (sender, mut receiver) = mpsc::unbounded_channel();
        self.progress_sender = Some(sender);

        // Start async progress processing
        tokio::spawn(async move {
            while let Some(update) = receiver.recv().await {
                // Process progress updates in background
                // This would integrate with the visual manager in a real implementation
                if let Some(error) = update.error {
                    eprintln!("File operation error: {}", error);
                }
            }
        });

        Ok(())
    }

    /// Check file permissions and accessibility
    pub fn check_file_permissions(&self, file_path: &Path) -> PluginResult<FilePermissionInfo> {
        let exists = file_path.exists();
        let is_directory = file_path.is_dir();
        
        let (readable, writable, executable) = if exists {
            let metadata = fs::metadata(file_path)
                .map_err(|e| PluginError::filesystem(&format!("Failed to read metadata for {}: {}", file_path.display(), e)))?;
            
            let permissions = metadata.permissions();
            
            // On Unix systems, check actual permissions
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = permissions.mode();
                (
                    mode & 0o400 != 0, // Owner read
                    mode & 0o200 != 0, // Owner write
                    mode & 0o100 != 0, // Owner execute
                )
            }
            
            // On Windows, use basic checks
            #[cfg(windows)]
            {
                (
                    !permissions.readonly(),
                    !permissions.readonly(),
                    file_path.extension().map_or(false, |ext| {
                        matches!(ext.to_str(), Some("exe") | Some("bat") | Some("cmd"))
                    }),
                )
            }
        } else {
            (false, false, false)
        };

        // Check if parent directory is writable for file creation
        let parent_writable = if let Some(parent) = file_path.parent() {
            if parent.exists() {
                let parent_metadata = fs::metadata(parent)
                    .map_err(|e| PluginError::filesystem(&format!("Failed to read parent metadata: {}", e)))?;
                
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    parent_metadata.permissions().mode() & 0o200 != 0
                }
                
                #[cfg(windows)]
                {
                    !parent_metadata.permissions().readonly()
                }
            } else {
                false
            }
        } else {
            false
        };

        Ok(FilePermissionInfo {
            readable,
            writable,
            executable,
            exists,
            is_directory,
            parent_writable,
        })
    }

    /// Start tracking a read operation with progress monitoring
    pub fn start_read_operation(&mut self, neovim: &mut Neovim, file_path: PathBuf) -> PluginResult<String> {
        // Check permissions first
        let permissions = self.check_file_permissions(&file_path)?;
        if !permissions.exists {
            return Err(PluginError::filesystem(&format!("File does not exist: {}", file_path.display())));
        }
        if !permissions.readable {
            return Err(PluginError::filesystem(&format!("File is not readable: {}", file_path.display())));
        }

        let operation_id = format!("read_{}_{}", 
            file_path.file_name().unwrap_or_default().to_string_lossy(),
            Instant::now().elapsed().as_millis()
        );

        // Get file size for progress calculation
        let file_size = fs::metadata(&file_path)
            .map(|m| m.len())
            .ok();

        let metadata = FileOperationMetadata {
            operation_type: FileOperationType::Read,
            file_path: file_path.clone(),
            start_time: Instant::now(),
            estimated_duration: file_size.map(|size| Duration::from_millis(size / 1024 + 100)), // Rough estimate
            file_size,
            bytes_processed: 0,
            buffer_id: None,
            requires_backup: false,
        };

        self.active_operations.insert(operation_id.clone(), metadata);

        // Create visual block
        let _block_id = self.visual_manager.show_read_operation(neovim, file_path.to_string_lossy().to_string())?;

        Ok(operation_id)
    }

    /// Update read operation progress
    pub fn update_read_progress(&mut self, _neovim: &mut Neovim, operation_id: &str, bytes_read: u64) -> PluginResult<()> {
        if let Some(metadata) = self.active_operations.get_mut(operation_id) {
            metadata.bytes_processed = bytes_read;
            
            let progress = if let Some(total_size) = metadata.file_size {
                if total_size > 0 {
                    (bytes_read as f32 / total_size as f32).min(1.0)
                } else {
                    1.0
                }
            } else {
                0.5 // Unknown size, show indeterminate progress
            };

            // Calculate estimated completion time
            let elapsed = metadata.start_time.elapsed();
            let _estimated_completion = if progress > 0.1 && progress < 1.0 {
                let estimated_total = elapsed.as_secs_f32() / progress;
                Some(Duration::from_secs_f32(estimated_total - elapsed.as_secs_f32()))
            } else {
                None
            };

            // Update visual block (this would need the actual block ID from visual manager)
            // For now, we'll create a new progress update
            if let Some(sender) = &self.progress_sender {
                let update = FileProgressUpdate {
                    operation_id: operation_id.to_string(),
                    progress,
                    bytes_processed: bytes_read,
                    total_bytes: metadata.file_size,
                    error: None,
                };
                let _ = sender.send(update);
            }
        }

        Ok(())
    }

    /// Complete a read operation
    pub fn complete_read_operation(&mut self, neovim: &mut Neovim, operation_id: &str) -> PluginResult<()> {
        if let Some(metadata) = self.active_operations.remove(operation_id) {
            // Update visual block to completed state
            if let Some(sender) = &self.progress_sender {
                let update = FileProgressUpdate {
                    operation_id: operation_id.to_string(),
                    progress: 1.0,
                    bytes_processed: metadata.bytes_processed,
                    total_bytes: metadata.file_size,
                    error: None,
                };
                let _ = sender.send(update);
            }

            // Integrate with Neovim buffer if enabled
            if self.buffer_integration_enabled {
                self.sync_with_neovim_buffer(neovim, &metadata.file_path)?;
            }
        }

        Ok(())
    }

    /// Start tracking a write operation with backup and progress monitoring
    pub fn start_write_operation(&mut self, neovim: &mut Neovim, file_path: PathBuf, content_preview: String, total_bytes: Option<u64>) -> PluginResult<String> {
        // Check permissions
        let permissions = self.check_file_permissions(&file_path)?;
        
        if permissions.exists && !permissions.writable {
            return Err(PluginError::filesystem(&format!("File is not writable: {}", file_path.display())));
        }
        
        if !permissions.exists && !permissions.parent_writable {
            return Err(PluginError::filesystem(&format!("Cannot create file, parent directory not writable: {}", file_path.display())));
        }

        let operation_id = format!("write_{}_{}", 
            file_path.file_name().unwrap_or_default().to_string_lossy(),
            Instant::now().elapsed().as_millis()
        );

        // Create backup if file exists and backup is enabled
        let requires_backup = self.auto_backup_enabled && permissions.exists;
        if requires_backup {
            self.create_backup(&file_path)?;
        }

        let metadata = FileOperationMetadata {
            operation_type: FileOperationType::Write,
            file_path: file_path.clone(),
            start_time: Instant::now(),
            estimated_duration: total_bytes.map(|size| Duration::from_millis(size / 512 + 200)), // Write estimate
            file_size: total_bytes,
            bytes_processed: 0,
            buffer_id: None,
            requires_backup,
        };

        self.active_operations.insert(operation_id.clone(), metadata);

        // Create visual block
        let _block_id = self.visual_manager.show_write_operation(neovim, file_path.to_string_lossy().to_string(), content_preview)?;

        Ok(operation_id)
    }

    /// Update write operation progress
    pub fn update_write_progress(&mut self, _neovim: &mut Neovim, operation_id: &str, bytes_written: u64) -> PluginResult<()> {
        if let Some(metadata) = self.active_operations.get_mut(operation_id) {
            metadata.bytes_processed = bytes_written;
            
            let progress = if let Some(total_size) = metadata.file_size {
                if total_size > 0 {
                    (bytes_written as f32 / total_size as f32).min(1.0)
                } else {
                    1.0
                }
            } else {
                0.5 // Unknown size
            };

            // Send progress update
            if let Some(sender) = &self.progress_sender {
                let update = FileProgressUpdate {
                    operation_id: operation_id.to_string(),
                    progress,
                    bytes_processed: bytes_written,
                    total_bytes: metadata.file_size,
                    error: None,
                };
                let _ = sender.send(update);
            }
        }

        Ok(())
    }

    /// Complete a write operation
    pub fn complete_write_operation(&mut self, neovim: &mut Neovim, operation_id: &str) -> PluginResult<()> {
        if let Some(metadata) = self.active_operations.remove(operation_id) {
            // Update visual block to completed state
            if let Some(sender) = &self.progress_sender {
                let update = FileProgressUpdate {
                    operation_id: operation_id.to_string(),
                    progress: 1.0,
                    bytes_processed: metadata.bytes_processed,
                    total_bytes: metadata.file_size,
                    error: None,
                };
                let _ = sender.send(update);
            }

            // Integrate with Neovim buffer if enabled
            if self.buffer_integration_enabled {
                self.sync_with_neovim_buffer(neovim, &metadata.file_path)?;
            }
        }

        Ok(())
    }

    /// Mark an operation as failed with error recovery
    pub fn fail_operation(&mut self, _neovim: &mut Neovim, operation_id: &str, error: String) -> PluginResult<()> {
        if let Some(metadata) = self.active_operations.remove(operation_id) {
            // Send error update
            if let Some(sender) = &self.progress_sender {
                let update = FileProgressUpdate {
                    operation_id: operation_id.to_string(),
                    progress: 0.0,
                    bytes_processed: metadata.bytes_processed,
                    total_bytes: metadata.file_size,
                    error: Some(error.clone()),
                };
                let _ = sender.send(update);
            }

            // Attempt error recovery
            self.attempt_error_recovery(&metadata, &error)?;
        }

        Ok(())
    }

    /// Create a backup of the file before writing
    fn create_backup(&self, file_path: &Path) -> PluginResult<()> {
        if !file_path.exists() {
            return Ok(());
        }

        let backup_name = format!("{}.backup.{}", 
            file_path.file_name().unwrap_or_default().to_string_lossy(),
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        );
        let backup_path = self.backup_directory.join(backup_name);

        fs::copy(file_path, &backup_path)
            .map_err(|e| PluginError::filesystem(&format!("Failed to create backup: {}", e)))?;

        Ok(())
    }

    /// Attempt to recover from file operation errors
    fn attempt_error_recovery(&self, metadata: &FileOperationMetadata, _error: &str) -> PluginResult<()> {
        match metadata.operation_type {
            FileOperationType::Write => {
                // If backup exists and write failed, suggest restoration
                if metadata.requires_backup {
                    let backup_files: Vec<_> = fs::read_dir(&self.backup_directory)
                        .map_err(|e| PluginError::filesystem(&format!("Failed to read backup directory: {}", e)))?
                        .filter_map(|entry| entry.ok())
                        .filter(|entry| {
                            let entry_name = entry.file_name().to_string_lossy().to_string();
                            let file_name = metadata.file_path.file_name().unwrap_or_default().to_string_lossy().to_string();
                            entry_name.contains(&file_name)
                        })
                        .collect();

                    if !backup_files.is_empty() {
                        // Log recovery suggestion (in a real implementation, this would notify the user)
                        eprintln!("Write operation failed. Backup files available for recovery: {:?}", backup_files);
                    }
                }
            }
            FileOperationType::Read => {
                // For read failures, check if file permissions changed
                if let Ok(permissions) = self.check_file_permissions(&metadata.file_path) {
                    if !permissions.readable {
                        eprintln!("Read failed due to permission changes. File is no longer readable.");
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Sync file operations with Neovim buffer management
    fn sync_with_neovim_buffer(&self, neovim: &mut Neovim, file_path: &Path) -> PluginResult<()> {
        let mut api = NeovimApiWrapper::new(neovim);
        
        // Check if file is already open in a buffer
        let lua_code = format!(
            r#"
            local buffers = vim.api.nvim_list_bufs()
            for _, buf in ipairs(buffers) do
                local buf_name = vim.api.nvim_buf_get_name(buf)
                if buf_name == '{}' then
                    return buf
                end
            end
            return nil
            "#,
            file_path.to_string_lossy().replace('\\', "\\\\").replace('\'', "\\'")
        );

        let result = api.execute_lua(&lua_code)?;
        
        if let Some(buffer_id) = result.as_i64() {
            // Buffer exists, reload it
            let reload_lua = format!(
                "vim.api.nvim_buf_call({}, function() vim.cmd('checktime') end)",
                buffer_id
            );
            api.execute_lua(&reload_lua)?;
        }

        Ok(())
    }

    /// Get all active operations
    pub fn get_active_operations(&self) -> &HashMap<String, FileOperationMetadata> {
        &self.active_operations
    }

    /// Clear completed operations
    pub fn clear_completed(&mut self) {
        // Operations are automatically removed when completed or failed
        // This method can be used for additional cleanup if needed
    }

    /// Get operation statistics
    pub fn get_operation_statistics(&self) -> (usize, u64, u64) {
        let active_count = self.active_operations.len();
        let total_bytes_processed: u64 = self.active_operations.values()
            .map(|op| op.bytes_processed)
            .sum();
        let total_bytes_remaining: u64 = self.active_operations.values()
            .filter_map(|op| op.file_size.map(|size| size.saturating_sub(op.bytes_processed)))
            .sum();

        (active_count, total_bytes_processed, total_bytes_remaining)
    }

    /// Set buffer integration enabled/disabled
    pub fn set_buffer_integration(&mut self, enabled: bool) {
        self.buffer_integration_enabled = enabled;
    }

    /// Set auto backup enabled/disabled
    pub fn set_auto_backup(&mut self, enabled: bool) {
        self.auto_backup_enabled = enabled;
    }

    /// Set backup directory
    pub fn set_backup_directory(&mut self, path: PathBuf) -> PluginResult<()> {
        if !path.exists() {
            fs::create_dir_all(&path)
                .map_err(|e| PluginError::filesystem(&format!("Failed to create backup directory: {}", e)))?;
        }
        self.backup_directory = path;
        Ok(())
    }

    /// Set maximum concurrent operations
    pub fn set_max_concurrent_operations(&mut self, max: usize) {
        self.max_concurrent_operations = max;
    }

    /// Check if we can start a new operation (respects concurrency limits)
    pub fn can_start_operation(&self) -> bool {
        self.active_operations.len() < self.max_concurrent_operations
    }

    /// Cancel an active operation
    pub fn cancel_operation(&mut self, operation_id: &str) -> PluginResult<()> {
        if let Some(metadata) = self.active_operations.remove(operation_id) {
            // Send cancellation update
            if let Some(sender) = &self.progress_sender {
                let update = FileProgressUpdate {
                    operation_id: operation_id.to_string(),
                    progress: 0.0,
                    bytes_processed: metadata.bytes_processed,
                    total_bytes: metadata.file_size,
                    error: Some("Operation cancelled by user".to_string()),
                };
                let _ = sender.send(update);
            }
        }
        Ok(())
    }
}