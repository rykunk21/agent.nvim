use serde::{Deserialize, Serialize};
use neovim_lib::{Neovim, NeovimApi};
use crate::utils::error_handling::{PluginResult, PluginError};
use crate::agent::{CommandBlock, ApprovalStatus, RiskLevel};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Visual operation blocks for file operations and commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationBlock {
    ReadBlock {
        id: String,
        file_path: String,
        progress: f32,
        status: OperationStatus,
        #[serde(skip)]
        start_time: Option<Instant>,
        estimated_completion: Option<Duration>,
    },
    WriteBlock {
        id: String,
        file_path: String,
        content_preview: String,
        status: OperationStatus,
        #[serde(skip)]
        start_time: Option<Instant>,
        bytes_written: usize,
        total_bytes: Option<usize>,
    },
}

/// Status of ongoing operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationStatus {
    InProgress,
    Completed,
    Failed(String),
    Cancelled,
}

/// Block state for tracking active operations
#[derive(Debug, Clone)]
pub struct BlockState {
    pub id: String,
    pub block_type: BlockType,
    pub created_at: Instant,
    pub last_updated: Instant,
    pub buffer_lines: Vec<String>,
    pub needs_refresh: bool,
}

#[derive(Debug, Clone)]
pub enum BlockType {
    Operation(OperationBlock),
    Command(CommandBlock),
}

/// Manages visual blocks and their rendering
pub struct VisualBlockRenderer {
    active_blocks: HashMap<String, BlockState>,
    next_block_id: u32,
    refresh_interval: Duration,
}

impl VisualBlockRenderer {
    pub fn new() -> Self {
        VisualBlockRenderer {
            active_blocks: HashMap::new(),
            next_block_id: 1,
            refresh_interval: Duration::from_millis(100), // 10 FPS refresh rate
        }
    }

    /// Create a new operation block and return its ID
    pub fn create_operation_block(&mut self, block: OperationBlock) -> String {
        let id = self.generate_block_id();
        let now = Instant::now();
        
        let block_state = BlockState {
            id: id.clone(),
            block_type: BlockType::Operation(block),
            created_at: now,
            last_updated: now,
            buffer_lines: Vec::new(),
            needs_refresh: true,
        };
        
        self.active_blocks.insert(id.clone(), block_state);
        id
    }

    /// Create a new command block and return its ID
    pub fn create_command_block(&mut self, command_block: CommandBlock) -> String {
        let id = self.generate_block_id();
        let now = Instant::now();
        
        let block_state = BlockState {
            id: id.clone(),
            block_type: BlockType::Command(command_block),
            created_at: now,
            last_updated: now,
            buffer_lines: Vec::new(),
            needs_refresh: true,
        };
        
        self.active_blocks.insert(id.clone(), block_state);
        id
    }

    /// Update an existing operation block
    pub fn update_operation_block(&mut self, id: &str, block: OperationBlock) -> PluginResult<()> {
        if let Some(block_state) = self.active_blocks.get_mut(id) {
            block_state.block_type = BlockType::Operation(block);
            block_state.last_updated = Instant::now();
            block_state.needs_refresh = true;
            Ok(())
        } else {
            Err(PluginError::unknown(&format!("Block not found: {}", id)))
        }
    }

    /// Update an existing command block
    pub fn update_command_block(&mut self, id: &str, command_block: CommandBlock) -> PluginResult<()> {
        if let Some(block_state) = self.active_blocks.get_mut(id) {
            block_state.block_type = BlockType::Command(command_block);
            block_state.last_updated = Instant::now();
            block_state.needs_refresh = true;
            Ok(())
        } else {
            Err(PluginError::unknown(&format!("Block not found: {}", id)))
        }
    }

    /// Remove a block and clean up its resources
    pub fn remove_block(&mut self, id: &str) -> PluginResult<()> {
        if self.active_blocks.remove(id).is_some() {
            Ok(())
        } else {
            Err(PluginError::unknown(&format!("Block not found: {}", id)))
        }
    }

    /// Get all active blocks that need refreshing
    pub fn get_blocks_needing_refresh(&mut self) -> Vec<&mut BlockState> {
        self.active_blocks
            .values_mut()
            .filter(|block| block.needs_refresh)
            .collect()
    }

    /// Render all active blocks to buffer lines
    pub fn render_all_blocks(&mut self) -> Vec<String> {
        let mut all_lines = Vec::new();
        
        // Sort blocks by creation time for consistent ordering
        let mut sorted_blocks: Vec<_> = self.active_blocks.values().collect();
        sorted_blocks.sort_by_key(|block| block.created_at);
        
        for block_state in sorted_blocks {
            let rendered_lines = self.render_block_state(block_state);
            all_lines.extend(rendered_lines);
            all_lines.push(String::new()); // Add separator line
        }
        
        // Mark all blocks as refreshed
        for block_state in self.active_blocks.values_mut() {
            block_state.needs_refresh = false;
        }
        
        all_lines
    }

    /// Render a specific block state to lines
    fn render_block_state(&self, block_state: &BlockState) -> Vec<String> {
        match &block_state.block_type {
            BlockType::Operation(op_block) => self.render_operation_block(op_block),
            BlockType::Command(cmd_block) => self.render_command_block(cmd_block),
        }
    }

    /// Render an operation block as text lines
    pub fn render_operation_block(&self, block: &OperationBlock) -> Vec<String> {
        match block {
            OperationBlock::ReadBlock { 
                file_path, 
                progress, 
                status, 
                start_time,
                estimated_completion,
                ..
            } => {
                let mut lines = Vec::new();
                
                let status_icon = match status {
                    OperationStatus::InProgress => "📖",
                    OperationStatus::Completed => "✅",
                    OperationStatus::Failed(_) => "❌",
                    OperationStatus::Cancelled => "🚫",
                };
                
                // Main status line
                lines.push(format!("{} Reading: {}", status_icon, file_path));
                
                // Progress bar
                if matches!(status, OperationStatus::InProgress) {
                    let progress_bar = self.create_progress_bar(*progress, 40);
                    lines.push(format!("  {} {:.1}%", progress_bar, progress * 100.0));
                    
                    // Time estimation
                    if let (Some(start), Some(estimated)) = (start_time, estimated_completion) {
                        let elapsed = start.elapsed();
                        let remaining = if elapsed < *estimated {
                            *estimated - elapsed
                        } else {
                            Duration::from_secs(0)
                        };
                        lines.push(format!("  ⏱️  ETA: {:.1}s", remaining.as_secs_f32()));
                    }
                }
                
                // Error details
                if let OperationStatus::Failed(error) = status {
                    lines.push(format!("  ❗ Error: {}", error));
                }
                
                lines
            }
            OperationBlock::WriteBlock { 
                file_path, 
                content_preview, 
                status,
                bytes_written,
                total_bytes,
                ..
            } => {
                let mut lines = Vec::new();
                
                let status_icon = match status {
                    OperationStatus::InProgress => "✏️",
                    OperationStatus::Completed => "✅",
                    OperationStatus::Failed(_) => "❌",
                    OperationStatus::Cancelled => "🚫",
                };
                
                // Main status line
                lines.push(format!("{} Writing: {}", status_icon, file_path));
                
                // Content preview
                let preview = if content_preview.len() > 60 {
                    format!("{}...", &content_preview[..57])
                } else {
                    content_preview.clone()
                };
                lines.push(format!("  📝 {}", preview));
                
                // Progress information
                if matches!(status, OperationStatus::InProgress) {
                    if let Some(total) = total_bytes {
                        let progress = *bytes_written as f32 / *total as f32;
                        let progress_bar = self.create_progress_bar(progress, 40);
                        lines.push(format!("  {} {} / {} bytes", progress_bar, bytes_written, total));
                    } else {
                        lines.push(format!("  📊 {} bytes written", bytes_written));
                    }
                }
                
                // Error details
                if let OperationStatus::Failed(error) = status {
                    lines.push(format!("  ❗ Error: {}", error));
                }
                
                lines
            }
        }
    }

    /// Render a command block with approval interface
    pub fn render_command_block(&self, command_block: &CommandBlock) -> Vec<String> {
        let mut lines = Vec::new();
        
        // Header with risk level indicator
        let risk_icon = match command_block.risk_level {
            RiskLevel::Low => "🟢",
            RiskLevel::Medium => "🟡", 
            RiskLevel::High => "🔴",
        };
        
        lines.push(format!("{}━━━ COMMAND APPROVAL ━━━", risk_icon));
        lines.push(String::new());
        
        // Command details
        lines.push(format!("📁 Directory: {}", command_block.working_directory));
        lines.push(format!("💻 Command: {}", command_block.command));
        
        if !command_block.description.is_empty() {
            lines.push(format!("📋 Description: {}", command_block.description));
        }
        
        lines.push(String::new());
        
        // Risk level warning
        match command_block.risk_level {
            RiskLevel::High => {
                lines.push("⚠️  HIGH RISK COMMAND - Review carefully!".to_string());
            }
            RiskLevel::Medium => {
                lines.push("⚡ Medium risk command - Please review".to_string());
            }
            RiskLevel::Low => {
                lines.push("✨ Low risk command".to_string());
            }
        }
        
        lines.push(String::new());
        
        // Approval status and controls
        match &command_block.approval_status {
            ApprovalStatus::Pending => {
                lines.push("🤔 Awaiting your approval...".to_string());
                lines.push(String::new());
                lines.push("Press 'a' to APPROVE  |  Press 'r' to REJECT".to_string());
            }
            ApprovalStatus::Approved => {
                lines.push("✅ APPROVED - Executing...".to_string());
            }
            ApprovalStatus::Rejected => {
                lines.push("❌ REJECTED - Command cancelled".to_string());
            }
            ApprovalStatus::Executed { output } => {
                lines.push("✅ EXECUTED".to_string());
                lines.push(String::new());
                
                if output.exit_code == 0 {
                    lines.push("🎉 Command completed successfully".to_string());
                } else {
                    lines.push(format!("⚠️  Command failed with exit code: {}", output.exit_code));
                }
                
                // Show output if available
                if !output.stdout.is_empty() {
                    lines.push(String::new());
                    lines.push("📤 Output:".to_string());
                    for line in output.stdout.lines().take(10) { // Limit output lines
                        lines.push(format!("  {}", line));
                    }
                }
                
                if !output.stderr.is_empty() {
                    lines.push(String::new());
                    lines.push("❗ Errors:".to_string());
                    for line in output.stderr.lines().take(5) { // Limit error lines
                        lines.push(format!("  {}", line));
                    }
                }
            }
        }
        
        lines.push(String::new());
        lines.push("━".repeat(50));
        
        lines
    }

    /// Create a visual progress bar
    fn create_progress_bar(&self, progress: f32, width: usize) -> String {
        let filled = (progress * width as f32) as usize;
        let empty = width - filled;
        
        format!("[{}{}]", 
            "█".repeat(filled),
            "░".repeat(empty)
        )
    }

    /// Generate a unique block ID
    fn generate_block_id(&mut self) -> String {
        let id = format!("block_{}", self.next_block_id);
        self.next_block_id += 1;
        id
    }

    /// Clean up completed or failed blocks older than specified duration
    pub fn cleanup_old_blocks(&mut self, max_age: Duration) {
        let now = Instant::now();
        let mut to_remove = Vec::new();
        
        for (id, block_state) in &self.active_blocks {
            let is_completed = match &block_state.block_type {
                BlockType::Operation(op_block) => {
                    matches!(op_block.status(), OperationStatus::Completed | OperationStatus::Failed(_) | OperationStatus::Cancelled)
                }
                BlockType::Command(cmd_block) => {
                    matches!(cmd_block.approval_status, ApprovalStatus::Executed { .. } | ApprovalStatus::Rejected)
                }
            };
            
            if is_completed && now.duration_since(block_state.last_updated) > max_age {
                to_remove.push(id.clone());
            }
        }
        
        for id in to_remove {
            self.active_blocks.remove(&id);
        }
    }

    /// Get count of active blocks by type
    pub fn get_block_counts(&self) -> (usize, usize) {
        let mut operation_count = 0;
        let mut command_count = 0;
        
        for block_state in self.active_blocks.values() {
            match &block_state.block_type {
                BlockType::Operation(_) => operation_count += 1,
                BlockType::Command(_) => command_count += 1,
            }
        }
        
        (operation_count, command_count)
    }

    /// Check if there are any pending command approvals
    pub fn has_pending_commands(&self) -> bool {
        self.active_blocks.values().any(|block_state| {
            if let BlockType::Command(cmd_block) = &block_state.block_type {
                matches!(cmd_block.approval_status, ApprovalStatus::Pending)
            } else {
                false
            }
        })
    }

    /// Get all pending command block IDs
    pub fn get_pending_command_ids(&self) -> Vec<String> {
        self.active_blocks
            .values()
            .filter_map(|block_state| {
                if let BlockType::Command(cmd_block) = &block_state.block_type {
                    if matches!(cmd_block.approval_status, ApprovalStatus::Pending) {
                        Some(block_state.id.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }
}

impl OperationBlock {
    /// Get the status of the operation block
    pub fn status(&self) -> &OperationStatus {
        match self {
            OperationBlock::ReadBlock { status, .. } => status,
            OperationBlock::WriteBlock { status, .. } => status,
        }
    }

    /// Get the ID of the operation block
    pub fn id(&self) -> &str {
        match self {
            OperationBlock::ReadBlock { id, .. } => id,
            OperationBlock::WriteBlock { id, .. } => id,
        }
    }

    /// Check if the operation is still in progress
    pub fn is_in_progress(&self) -> bool {
        matches!(self.status(), OperationStatus::InProgress)
    }

    /// Check if the operation is completed (successfully or with failure)
    pub fn is_finished(&self) -> bool {
        matches!(self.status(), OperationStatus::Completed | OperationStatus::Failed(_) | OperationStatus::Cancelled)
    }
}

/// Manages visual blocks integration with window system
pub struct VisualBlockManager {
    renderer: VisualBlockRenderer,
    chat_buffer_id: Option<i32>,
    command_buffer_id: Option<i32>,
    auto_cleanup_enabled: bool,
    cleanup_interval: Duration,
}

impl VisualBlockManager {
    pub fn new() -> Self {
        VisualBlockManager {
            renderer: VisualBlockRenderer::new(),
            chat_buffer_id: None,
            command_buffer_id: None,
            auto_cleanup_enabled: true,
            cleanup_interval: Duration::from_secs(30), // Clean up completed blocks after 30 seconds
        }
    }

    /// Set the chat buffer ID for rendering operation blocks
    pub fn set_chat_buffer(&mut self, buffer_id: i32) {
        self.chat_buffer_id = Some(buffer_id);
    }

    /// Set the command buffer ID for rendering command blocks
    pub fn set_command_buffer(&mut self, buffer_id: i32) {
        self.command_buffer_id = Some(buffer_id);
    }

    /// Create and display a read operation block
    pub fn show_read_operation(&mut self, neovim: &mut Neovim, file_path: String) -> PluginResult<String> {
        let block = OperationBlock::ReadBlock {
            id: String::new(), // Will be set by renderer
            file_path,
            progress: 0.0,
            status: OperationStatus::InProgress,
            start_time: Some(Instant::now()),
            estimated_completion: None,
        };
        
        let block_id = self.renderer.create_operation_block(block);
        self.refresh_chat_display(neovim)?;
        Ok(block_id)
    }

    /// Create and display a write operation block
    pub fn show_write_operation(&mut self, neovim: &mut Neovim, file_path: String, content_preview: String) -> PluginResult<String> {
        let block = OperationBlock::WriteBlock {
            id: String::new(), // Will be set by renderer
            file_path,
            content_preview,
            status: OperationStatus::InProgress,
            start_time: Some(Instant::now()),
            bytes_written: 0,
            total_bytes: None,
        };
        
        let block_id = self.renderer.create_operation_block(block);
        self.refresh_chat_display(neovim)?;
        Ok(block_id)
    }

    /// Update progress of a read operation
    pub fn update_read_progress(&mut self, neovim: &mut Neovim, block_id: &str, progress: f32, estimated_completion: Option<Duration>) -> PluginResult<()> {
        if let Some(block_state) = self.renderer.active_blocks.get(block_id) {
            if let BlockType::Operation(OperationBlock::ReadBlock { id, file_path, status, start_time, .. }) = &block_state.block_type {
                let updated_block = OperationBlock::ReadBlock {
                    id: id.clone(),
                    file_path: file_path.clone(),
                    progress: progress.clamp(0.0, 1.0),
                    status: status.clone(),
                    start_time: *start_time,
                    estimated_completion,
                };
                
                self.renderer.update_operation_block(block_id, updated_block)?;
                self.refresh_chat_display(neovim)?;
            }
        }
        Ok(())
    }

    /// Update progress of a write operation
    pub fn update_write_progress(&mut self, neovim: &mut Neovim, block_id: &str, bytes_written: usize, total_bytes: Option<usize>) -> PluginResult<()> {
        if let Some(block_state) = self.renderer.active_blocks.get(block_id) {
            if let BlockType::Operation(OperationBlock::WriteBlock { id, file_path, content_preview, status, start_time, .. }) = &block_state.block_type {
                let updated_block = OperationBlock::WriteBlock {
                    id: id.clone(),
                    file_path: file_path.clone(),
                    content_preview: content_preview.clone(),
                    status: status.clone(),
                    start_time: *start_time,
                    bytes_written,
                    total_bytes,
                };
                
                self.renderer.update_operation_block(block_id, updated_block)?;
                self.refresh_chat_display(neovim)?;
            }
        }
        Ok(())
    }

    /// Complete an operation block
    pub fn complete_operation(&mut self, neovim: &mut Neovim, block_id: &str) -> PluginResult<()> {
        if let Some(block_state) = self.renderer.active_blocks.get(block_id) {
            let updated_block = match &block_state.block_type {
                BlockType::Operation(OperationBlock::ReadBlock { id, file_path, start_time, estimated_completion, .. }) => {
                    OperationBlock::ReadBlock {
                        id: id.clone(),
                        file_path: file_path.clone(),
                        progress: 1.0,
                        status: OperationStatus::Completed,
                        start_time: *start_time,
                        estimated_completion: *estimated_completion,
                    }
                }
                BlockType::Operation(OperationBlock::WriteBlock { id, file_path, content_preview, start_time, bytes_written, total_bytes, .. }) => {
                    OperationBlock::WriteBlock {
                        id: id.clone(),
                        file_path: file_path.clone(),
                        content_preview: content_preview.clone(),
                        status: OperationStatus::Completed,
                        start_time: *start_time,
                        bytes_written: *bytes_written,
                        total_bytes: *total_bytes,
                    }
                }
                _ => return Ok(()), // Not an operation block
            };
            
            self.renderer.update_operation_block(block_id, updated_block)?;
            self.refresh_chat_display(neovim)?;
        }
        Ok(())
    }

    /// Fail an operation block with error message
    pub fn fail_operation(&mut self, neovim: &mut Neovim, block_id: &str, error_message: String) -> PluginResult<()> {
        if let Some(block_state) = self.renderer.active_blocks.get(block_id) {
            let updated_block = match &block_state.block_type {
                BlockType::Operation(OperationBlock::ReadBlock { id, file_path, progress, start_time, estimated_completion, .. }) => {
                    OperationBlock::ReadBlock {
                        id: id.clone(),
                        file_path: file_path.clone(),
                        progress: *progress,
                        status: OperationStatus::Failed(error_message),
                        start_time: *start_time,
                        estimated_completion: *estimated_completion,
                    }
                }
                BlockType::Operation(OperationBlock::WriteBlock { id, file_path, content_preview, start_time, bytes_written, total_bytes, .. }) => {
                    OperationBlock::WriteBlock {
                        id: id.clone(),
                        file_path: file_path.clone(),
                        content_preview: content_preview.clone(),
                        status: OperationStatus::Failed(error_message),
                        start_time: *start_time,
                        bytes_written: *bytes_written,
                        total_bytes: *total_bytes,
                    }
                }
                _ => return Ok(()), // Not an operation block
            };
            
            self.renderer.update_operation_block(block_id, updated_block)?;
            self.refresh_chat_display(neovim)?;
        }
        Ok(())
    }

    /// Show a command approval block and trigger layout adjustment
    pub fn show_command_approval(&mut self, neovim: &mut Neovim, command_block: CommandBlock) -> PluginResult<String> {
        let block_id = self.renderer.create_command_block(command_block);
        
        // Refresh command buffer display
        self.refresh_command_display(neovim)?;
        
        Ok(block_id)
    }

    /// Update command block status
    pub fn update_command_status(&mut self, neovim: &mut Neovim, block_id: &str, command_block: CommandBlock) -> PluginResult<()> {
        self.renderer.update_command_block(block_id, command_block)?;
        self.refresh_command_display(neovim)?;
        Ok(())
    }

    /// Remove a block and refresh display
    pub fn remove_block(&mut self, neovim: &mut Neovim, block_id: &str) -> PluginResult<()> {
        self.renderer.remove_block(block_id)?;
        self.refresh_chat_display(neovim)?;
        self.refresh_command_display(neovim)?;
        Ok(())
    }

    /// Refresh the chat buffer with current operation blocks
    fn refresh_chat_display(&mut self, neovim: &mut Neovim) -> PluginResult<()> {
        if let Some(buffer_id) = self.chat_buffer_id {
            let operation_lines = self.get_operation_block_lines();
            self.update_buffer_content(neovim, buffer_id, &operation_lines)?;
        }
        Ok(())
    }

    /// Refresh the command buffer with current command blocks
    fn refresh_command_display(&mut self, neovim: &mut Neovim) -> PluginResult<()> {
        if let Some(buffer_id) = self.command_buffer_id {
            let command_lines = self.get_command_block_lines();
            self.update_buffer_content(neovim, buffer_id, &command_lines)?;
        }
        Ok(())
    }

    /// Get lines for operation blocks only
    fn get_operation_block_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        
        for block_state in self.renderer.active_blocks.values() {
            if let BlockType::Operation(op_block) = &block_state.block_type {
                let block_lines = self.renderer.render_operation_block(op_block);
                lines.extend(block_lines);
                lines.push(String::new()); // Separator
            }
        }
        
        lines
    }

    /// Get lines for command blocks only
    fn get_command_block_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        
        for block_state in self.renderer.active_blocks.values() {
            if let BlockType::Command(cmd_block) = &block_state.block_type {
                let block_lines = self.renderer.render_command_block(cmd_block);
                lines.extend(block_lines);
                lines.push(String::new()); // Separator
            }
        }
        
        lines
    }

    /// Update buffer content with new lines
    fn update_buffer_content(&self, neovim: &mut Neovim, buffer_id: i32, lines: &[String]) -> PluginResult<()> {
        let lua_code = format!(
            r#"
            local lines = {{}}
            for i, line in ipairs({:?}) do
                table.insert(lines, line)
            end
            vim.api.nvim_buf_set_lines({}, 0, -1, false, lines)
            "#,
            lines, buffer_id
        );
        
        neovim.execute_lua(&lua_code, vec![])?;
        Ok(())
    }

    /// Perform automatic cleanup of old completed blocks
    pub fn auto_cleanup(&mut self) {
        if self.auto_cleanup_enabled {
            self.renderer.cleanup_old_blocks(self.cleanup_interval);
        }
    }

    /// Check if there are pending command approvals
    pub fn has_pending_commands(&self) -> bool {
        self.renderer.has_pending_commands()
    }

    /// Get all pending command IDs
    pub fn get_pending_command_ids(&self) -> Vec<String> {
        self.renderer.get_pending_command_ids()
    }

    /// Get block counts for monitoring
    pub fn get_block_counts(&self) -> (usize, usize) {
        self.renderer.get_block_counts()
    }

    /// Enable or disable auto cleanup
    pub fn set_auto_cleanup(&mut self, enabled: bool) {
        self.auto_cleanup_enabled = enabled;
    }

    /// Set cleanup interval
    pub fn set_cleanup_interval(&mut self, interval: Duration) {
        self.cleanup_interval = interval;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{CommandBlock, ApprovalStatus, RiskLevel};
    use std::time::Duration;

    #[test]
    fn test_visual_block_renderer_creation() {
        let renderer = VisualBlockRenderer::new();
        assert_eq!(renderer.active_blocks.len(), 0);
        assert_eq!(renderer.next_block_id, 1);
    }

    #[test]
    fn test_operation_block_creation() {
        let mut renderer = VisualBlockRenderer::new();
        
        let read_block = OperationBlock::ReadBlock {
            id: "test_read".to_string(),
            file_path: "test.txt".to_string(),
            progress: 0.5,
            status: OperationStatus::InProgress,
            start_time: Some(Instant::now()),
            estimated_completion: Some(Duration::from_secs(10)),
        };
        
        let block_id = renderer.create_operation_block(read_block);
        assert_eq!(renderer.active_blocks.len(), 1);
        assert!(renderer.active_blocks.contains_key(&block_id));
    }

    #[test]
    fn test_command_block_creation() {
        let mut renderer = VisualBlockRenderer::new();
        
        let command_block = CommandBlock {
            command: "ls -la".to_string(),
            working_directory: "/tmp".to_string(),
            description: "List files".to_string(),
            risk_level: RiskLevel::Low,
            approval_status: ApprovalStatus::Pending,
        };
        
        let block_id = renderer.create_command_block(command_block);
        assert_eq!(renderer.active_blocks.len(), 1);
        assert!(renderer.active_blocks.contains_key(&block_id));
    }

    #[test]
    fn test_operation_block_rendering() {
        let renderer = VisualBlockRenderer::new();
        
        let read_block = OperationBlock::ReadBlock {
            id: "test_read".to_string(),
            file_path: "test.txt".to_string(),
            progress: 0.75,
            status: OperationStatus::InProgress,
            start_time: Some(Instant::now()),
            estimated_completion: Some(Duration::from_secs(5)),
        };
        
        let lines = renderer.render_operation_block(&read_block);
        assert!(!lines.is_empty());
        assert!(lines[0].contains("📖"));
        assert!(lines[0].contains("test.txt"));
        assert!(lines[1].contains("75.0%"));
    }

    #[test]
    fn test_command_block_rendering() {
        let renderer = VisualBlockRenderer::new();
        
        let command_block = CommandBlock {
            command: "echo hello".to_string(),
            working_directory: "/tmp".to_string(),
            description: "Test command".to_string(),
            risk_level: RiskLevel::Low,
            approval_status: ApprovalStatus::Pending,
        };
        
        let lines = renderer.render_command_block(&command_block);
        assert!(!lines.is_empty());
        assert!(lines.iter().any(|line| line.contains("echo hello")));
        assert!(lines.iter().any(|line| line.contains("/tmp")));
        assert!(lines.iter().any(|line| line.contains("Test command")));
    }

    #[test]
    fn test_progress_bar_creation() {
        let renderer = VisualBlockRenderer::new();
        
        let progress_bar_0 = renderer.create_progress_bar(0.0, 10);
        assert_eq!(progress_bar_0, "[░░░░░░░░░░]");
        
        let progress_bar_50 = renderer.create_progress_bar(0.5, 10);
        assert_eq!(progress_bar_50, "[█████░░░░░]");
        
        let progress_bar_100 = renderer.create_progress_bar(1.0, 10);
        assert_eq!(progress_bar_100, "[██████████]");
    }

    #[test]
    fn test_block_cleanup() {
        let mut renderer = VisualBlockRenderer::new();
        
        // Create a completed operation block
        let completed_block = OperationBlock::ReadBlock {
            id: "completed".to_string(),
            file_path: "test.txt".to_string(),
            progress: 1.0,
            status: OperationStatus::Completed,
            start_time: Some(Instant::now() - Duration::from_secs(60)), // 1 minute ago
            estimated_completion: None,
        };
        
        let block_id = renderer.create_operation_block(completed_block);
        
        // Manually set the last_updated time to simulate old block
        if let Some(block_state) = renderer.active_blocks.get_mut(&block_id) {
            block_state.last_updated = Instant::now() - Duration::from_secs(60);
        }
        
        assert_eq!(renderer.active_blocks.len(), 1);
        
        // Cleanup with 30 second threshold should remove the block
        renderer.cleanup_old_blocks(Duration::from_secs(30));
        assert_eq!(renderer.active_blocks.len(), 0);
    }

    #[test]
    fn test_pending_command_detection() {
        let mut renderer = VisualBlockRenderer::new();
        
        assert!(!renderer.has_pending_commands());
        
        let pending_command = CommandBlock {
            command: "test".to_string(),
            working_directory: "/tmp".to_string(),
            description: "Test".to_string(),
            risk_level: RiskLevel::Low,
            approval_status: ApprovalStatus::Pending,
        };
        
        renderer.create_command_block(pending_command);
        assert!(renderer.has_pending_commands());
        
        let pending_ids = renderer.get_pending_command_ids();
        assert_eq!(pending_ids.len(), 1);
    }

    #[test]
    fn test_block_counts() {
        let mut renderer = VisualBlockRenderer::new();
        
        let (op_count, cmd_count) = renderer.get_block_counts();
        assert_eq!(op_count, 0);
        assert_eq!(cmd_count, 0);
        
        // Add operation block
        let read_block = OperationBlock::ReadBlock {
            id: "test".to_string(),
            file_path: "test.txt".to_string(),
            progress: 0.0,
            status: OperationStatus::InProgress,
            start_time: Some(Instant::now()),
            estimated_completion: None,
        };
        renderer.create_operation_block(read_block);
        
        // Add command block
        let command_block = CommandBlock {
            command: "test".to_string(),
            working_directory: "/tmp".to_string(),
            description: "Test".to_string(),
            risk_level: RiskLevel::Low,
            approval_status: ApprovalStatus::Pending,
        };
        renderer.create_command_block(command_block);
        
        let (op_count, cmd_count) = renderer.get_block_counts();
        assert_eq!(op_count, 1);
        assert_eq!(cmd_count, 1);
    }

    #[test]
    fn test_visual_block_manager_creation() {
        let manager = VisualBlockManager::new();
        assert!(manager.chat_buffer_id.is_none());
        assert!(manager.command_buffer_id.is_none());
        assert!(manager.auto_cleanup_enabled);
    }

    #[test]
    fn test_operation_block_status_methods() {
        let in_progress_block = OperationBlock::ReadBlock {
            id: "test".to_string(),
            file_path: "test.txt".to_string(),
            progress: 0.5,
            status: OperationStatus::InProgress,
            start_time: Some(Instant::now()),
            estimated_completion: None,
        };
        
        assert!(in_progress_block.is_in_progress());
        assert!(!in_progress_block.is_finished());
        
        let completed_block = OperationBlock::ReadBlock {
            id: "test".to_string(),
            file_path: "test.txt".to_string(),
            progress: 1.0,
            status: OperationStatus::Completed,
            start_time: Some(Instant::now()),
            estimated_completion: None,
        };
        
        assert!(!completed_block.is_in_progress());
        assert!(completed_block.is_finished());
    }
}