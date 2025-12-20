use crate::agent::{CommandBlock, CommandIntegration, ApprovalStatus};
use crate::ui::{WindowManager, VisualBlockManager, CommandApprovalUI};
use crate::utils::error_handling::{PluginResult, PluginError};
use neovim_lib::{Neovim, NeovimApi};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

/// Complete command execution workflow with approval system
pub struct CommandWorkflow {
    integration: CommandIntegration,
    approval_ui: CommandApprovalUI,
    window_manager: Arc<Mutex<WindowManager>>,
    visual_block_manager: Arc<Mutex<VisualBlockManager>>,
}

impl CommandWorkflow {
    /// Create a new command workflow system
    pub fn new() -> PluginResult<Self> {
        let window_manager = Arc::new(Mutex::new(WindowManager::new()?));
        let visual_block_manager = Arc::new(Mutex::new(VisualBlockManager::new()));
        
        let integration = CommandIntegration::new(
            window_manager.clone(),
            visual_block_manager.clone(),
        );
        
        let approval_ui = CommandApprovalUI::new(
            window_manager.clone(),
            visual_block_manager.clone(),
        );

        Ok(CommandWorkflow {
            integration,
            approval_ui,
            window_manager,
            visual_block_manager,
        })
    }

    /// Initialize the workflow system with Neovim
    pub fn initialize(&mut self, neovim: &mut Neovim) -> PluginResult<()> {
        // Initialize window manager
        {
            let mut window_manager = self.window_manager.lock().unwrap();
            window_manager.initialize(neovim)?;
        }

        // Set up buffer IDs for visual block manager
        self.setup_visual_buffers(neovim)?;

        Ok(())
    }

    /// Execute a command with full approval workflow
    pub async fn execute_command_with_approval(
        &mut self,
        neovim: &mut Neovim,
        command: &str,
        description: &str,
    ) -> PluginResult<CommandBlock> {
        // Step 1: Validate command
        self.integration.executor().validate_command(command)?;

        // Step 2: Show approval UI
        let command_block = self.integration.executor().present_command(
            command,
            &self.integration.executor().get_working_directory().to_string_lossy(),
            description,
        );

        let block_id = self.approval_ui.show_command_approval(neovim, command_block.clone())?;

        // Step 3: Wait for user decision (this would be handled by keybindings in practice)
        // For now, we'll return the command block in pending state
        Ok(command_block)
    }

    /// Handle approval from user input
    pub async fn handle_approval(&mut self, neovim: &mut Neovim, block_id: &str) -> PluginResult<()> {
        // Get approved command
        let mut command_block = self.approval_ui.approve_command(neovim, block_id)?;

        // Execute the command
        self.integration.executor_mut().execute_command_async(neovim, &mut command_block, block_id).await?;

        // Update UI with execution results
        if let ApprovalStatus::Executed { output } = &command_block.approval_status {
            self.approval_ui.update_execution_status(neovim, block_id, output.clone())?;
        }

        Ok(())
    }

    /// Handle rejection from user input
    pub async fn handle_rejection(&mut self, neovim: &mut Neovim, block_id: &str) -> PluginResult<()> {
        let _command_block = self.approval_ui.reject_command(neovim, block_id)?;
        Ok(())
    }

    /// Set working directory for command execution
    pub fn set_working_directory(&mut self, path: PathBuf) -> PluginResult<()> {
        self.integration.executor_mut().set_working_directory(path)
    }

    /// Set command execution timeout
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.integration.executor_mut().set_timeout(timeout);
    }

    /// Check if there are pending command approvals
    pub fn has_pending_commands(&self) -> bool {
        self.approval_ui.has_pending_approvals()
    }

    /// Get count of pending commands
    pub fn pending_command_count(&self) -> usize {
        self.approval_ui.pending_approval_count()
    }

    /// Close all command approval interfaces
    pub fn close_all_approvals(&mut self, neovim: &mut Neovim) -> PluginResult<()> {
        self.approval_ui.close_approval_interface(neovim)
    }

    /// Clean up old completed commands and approvals
    pub fn cleanup(&mut self) {
        self.integration.cleanup_completed_commands();
        self.approval_ui.cleanup_old_approvals(Duration::from_secs(300)); // 5 minutes
        
        // Auto-cleanup visual blocks
        {
            let mut visual_manager = self.visual_block_manager.lock().unwrap();
            visual_manager.auto_cleanup();
        }
    }

    /// Handle terminal resize events
    pub fn handle_resize(&mut self, neovim: &mut Neovim) -> PluginResult<()> {
        let mut window_manager = self.window_manager.lock().unwrap();
        window_manager.handle_resize(neovim)
    }

    /// Show agent interface
    pub fn show_interface(&mut self, neovim: &mut Neovim) -> PluginResult<()> {
        let mut window_manager = self.window_manager.lock().unwrap();
        window_manager.create_agent_interface(neovim)
    }

    /// Hide agent interface
    pub fn hide_interface(&mut self, neovim: &mut Neovim) -> PluginResult<()> {
        let mut window_manager = self.window_manager.lock().unwrap();
        window_manager.close_all_windows(neovim)
    }

    /// Check if agent interface is open
    pub fn is_interface_open(&self) -> bool {
        let window_manager = self.window_manager.lock().unwrap();
        window_manager.is_interface_open()
    }

    /// Execute a safe command without approval (for low-risk operations)
    pub async fn execute_safe_command(
        &mut self,
        neovim: &mut Neovim,
        command: &str,
        description: &str,
    ) -> PluginResult<CommandBlock> {
        // Validate command is actually safe
        if !self.integration.executor().is_safe_command(command) {
            return Err(PluginError::command("Command is not safe for automatic execution"));
        }

        // Create command block
        let mut command_block = self.integration.executor().present_command(
            command,
            &self.integration.executor().get_working_directory().to_string_lossy(),
            description,
        );

        // Auto-approve safe command
        command_block.approval_status = ApprovalStatus::Approved;

        // Execute directly
        let block_id = format!("safe_exec_{}", Uuid::new_v4());
        self.integration.executor_mut().execute_command_async(neovim, &mut command_block, &block_id).await?;

        Ok(command_block)
    }

    /// Get detailed error information from last command
    pub fn get_last_error_details(&self) -> Option<String> {
        // This would track the last executed command's output
        // For now, return None as placeholder
        None
    }

    /// Setup visual buffer IDs
    fn setup_visual_buffers(&mut self, neovim: &mut Neovim) -> PluginResult<()> {
        // Create buffers for visual blocks if needed
        let chat_buffer_result = neovim.execute_lua("return vim.api.nvim_create_buf(false, true)", vec![])?;
        let chat_buffer_id = chat_buffer_result.as_i64().unwrap() as i32;

        let command_buffer_result = neovim.execute_lua("return vim.api.nvim_create_buf(false, true)", vec![])?;
        let command_buffer_id = command_buffer_result.as_i64().unwrap() as i32;

        {
            let mut visual_manager = self.visual_block_manager.lock().unwrap();
            visual_manager.set_chat_buffer(chat_buffer_id);
            visual_manager.set_command_buffer(command_buffer_id);
        }

        Ok(())
    }

    /// Get executor reference for validation and utility functions
    pub fn executor(&self) -> &crate::agent::CommandExecutor {
        self.integration.executor()
    }

    /// Get workflow statistics
    pub fn get_statistics(&self) -> WorkflowStatistics {
        let (operation_count, command_count) = {
            let visual_manager = self.visual_block_manager.lock().unwrap();
            visual_manager.get_block_counts()
        };

        WorkflowStatistics {
            pending_approvals: self.approval_ui.pending_approval_count(),
            pending_commands: self.integration.pending_command_count(),
            active_operations: operation_count,
            active_command_blocks: command_count,
            interface_open: self.is_interface_open(),
        }
    }
}

/// Statistics about the current workflow state
#[derive(Debug, Clone)]
pub struct WorkflowStatistics {
    pub pending_approvals: usize,
    pub pending_commands: usize,
    pub active_operations: usize,
    pub active_command_blocks: usize,
    pub interface_open: bool,
}

impl WorkflowStatistics {
    /// Check if the workflow is idle
    pub fn is_idle(&self) -> bool {
        self.pending_approvals == 0 && 
        self.pending_commands == 0 && 
        self.active_operations == 0 && 
        self.active_command_blocks == 0
    }

    /// Get total active items
    pub fn total_active(&self) -> usize {
        self.pending_approvals + self.pending_commands + self.active_operations + self.active_command_blocks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workflow_creation() {
        let workflow = CommandWorkflow::new();
        assert!(workflow.is_ok());
        
        let wf = workflow.unwrap();
        assert!(!wf.has_pending_commands());
        assert_eq!(wf.pending_command_count(), 0);
        assert!(!wf.is_interface_open());
    }

    #[test]
    fn test_workflow_statistics() {
        let workflow = CommandWorkflow::new().unwrap();
        let stats = workflow.get_statistics();
        
        assert!(stats.is_idle());
        assert_eq!(stats.total_active(), 0);
        assert!(!stats.interface_open);
    }

    #[test]
    fn test_safe_command_validation() {
        let workflow = CommandWorkflow::new().unwrap();
        
        // Test safe commands
        assert!(workflow.integration.executor().is_safe_command("ls -la"));
        assert!(workflow.integration.executor().is_safe_command("pwd"));
        assert!(workflow.integration.executor().is_safe_command("echo hello"));
        
        // Test unsafe commands
        assert!(!workflow.integration.executor().is_safe_command("rm -rf /"));
        assert!(!workflow.integration.executor().is_safe_command("sudo rm file"));
    }

    #[test]
    fn test_working_directory_management() {
        let mut workflow = CommandWorkflow::new().unwrap();
        
        // Test setting valid directory
        let temp_dir = std::env::temp_dir();
        assert!(workflow.set_working_directory(temp_dir.clone()).is_ok());
        
        // Test setting invalid directory
        let invalid_dir = PathBuf::from("/nonexistent/directory");
        assert!(workflow.set_working_directory(invalid_dir).is_err());
    }

    #[test]
    fn test_timeout_management() {
        let mut workflow = CommandWorkflow::new().unwrap();
        
        // Test setting timeout
        workflow.set_timeout(Duration::from_secs(60));
        
        // Verify timeout is set (we can't directly test this without accessing private fields)
        // But we can ensure the method doesn't panic
    }
}