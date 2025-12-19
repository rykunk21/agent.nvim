use crate::agent::{CommandExecutor, CommandBlock, ApprovalStatus};
use crate::ui::{WindowManager, VisualBlockManager};
use crate::utils::error_handling::{PluginResult, PluginError};
use neovim_lib::Neovim;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Integration layer for command execution with UI approval workflow
pub struct CommandIntegration {
    executor: CommandExecutor,
    pending_commands: Arc<Mutex<HashMap<String, CommandBlock>>>,
    approval_sender: mpsc::UnboundedSender<CommandApprovalEvent>,
    approval_receiver: Arc<Mutex<mpsc::UnboundedReceiver<CommandApprovalEvent>>>,
}

/// Events for command approval workflow
#[derive(Debug, Clone)]
pub enum CommandApprovalEvent {
    Approve(String), // block_id
    Reject(String),  // block_id
    Timeout(String), // block_id
}

impl CommandIntegration {
    pub fn new(
        window_manager: Arc<Mutex<WindowManager>>,
        visual_block_manager: Arc<Mutex<VisualBlockManager>>,
    ) -> Self {
        let (approval_sender, approval_receiver) = mpsc::unbounded_channel();
        
        CommandIntegration {
            executor: CommandExecutor::new(window_manager, visual_block_manager),
            pending_commands: Arc::new(Mutex::new(HashMap::new())),
            approval_sender,
            approval_receiver: Arc::new(Mutex::new(approval_receiver)),
        }
    }

    /// Present a command for approval and wait for user response
    pub async fn request_command_approval(
        &mut self,
        neovim: &mut Neovim,
        command: &str,
        description: &str,
    ) -> PluginResult<CommandBlock> {
        // Validate command first
        self.executor.validate_command(command)?;

        // Present command for approval
        let block_id = self.executor.present_command_for_approval(neovim, command, description).await?;

        // Create command block and store it
        let command_block = self.executor.present_command(
            command,
            &self.executor.get_working_directory().to_string_lossy(),
            description,
        );

        {
            let mut pending = self.pending_commands.lock().unwrap();
            pending.insert(block_id.clone(), command_block.clone());
        }

        // Wait for approval or rejection
        self.wait_for_approval(&block_id).await
    }

    /// Handle approval event
    pub async fn handle_approval(&mut self, neovim: &mut Neovim, block_id: &str) -> PluginResult<()> {
        self.approval_sender.send(CommandApprovalEvent::Approve(block_id.to_string()))
            .map_err(|_| PluginError::agent("Failed to send approval event"))?;

        self.executor.approve_command(neovim, block_id).await
    }

    /// Handle rejection event
    pub async fn handle_rejection(&mut self, neovim: &mut Neovim, block_id: &str) -> PluginResult<()> {
        self.approval_sender.send(CommandApprovalEvent::Reject(block_id.to_string()))
            .map_err(|_| PluginError::agent("Failed to send rejection event"))?;

        self.executor.reject_command(neovim, block_id).await
    }

    /// Wait for user approval or rejection
    async fn wait_for_approval(&self, block_id: &str) -> PluginResult<CommandBlock> {
        let mut receiver = self.approval_receiver.lock().unwrap();
        
        while let Some(event) = receiver.recv().await {
            match event {
                CommandApprovalEvent::Approve(id) if id == block_id => {
                    let mut pending = self.pending_commands.lock().unwrap();
                    if let Some(mut command_block) = pending.remove(&id) {
                        command_block.approval_status = ApprovalStatus::Approved;
                        return Ok(command_block);
                    }
                }
                CommandApprovalEvent::Reject(id) if id == block_id => {
                    let mut pending = self.pending_commands.lock().unwrap();
                    if let Some(mut command_block) = pending.remove(&id) {
                        command_block.approval_status = ApprovalStatus::Rejected;
                        return Ok(command_block);
                    }
                }
                CommandApprovalEvent::Timeout(id) if id == block_id => {
                    let mut pending = self.pending_commands.lock().unwrap();
                    pending.remove(&id);
                    return Err(PluginError::command("Command approval timed out"));
                }
                _ => continue, // Event for different command
            }
        }

        Err(PluginError::agent("Approval channel closed unexpectedly"))
    }

    /// Execute a command with full workflow
    pub async fn execute_command_with_approval(
        &mut self,
        neovim: &mut Neovim,
        command: &str,
        description: &str,
    ) -> PluginResult<CommandBlock> {
        // Request approval
        let mut command_block = self.request_command_approval(neovim, command, description).await?;

        // If approved, execute the command
        if matches!(command_block.approval_status, ApprovalStatus::Approved) {
            let block_id = format!("exec_{}", Uuid::new_v4());
            self.executor.execute_command_async(neovim, &mut command_block, &block_id).await?;
        }

        Ok(command_block)
    }

    /// Get executor reference for direct access
    pub fn executor(&self) -> &CommandExecutor {
        &self.executor
    }

    /// Get mutable executor reference
    pub fn executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    /// Clean up completed or rejected commands
    pub fn cleanup_completed_commands(&self) {
        let mut pending = self.pending_commands.lock().unwrap();
        pending.retain(|_, command_block| {
            matches!(command_block.approval_status, ApprovalStatus::Pending)
        });
    }

    /// Get count of pending commands
    pub fn pending_command_count(&self) -> usize {
        let pending = self.pending_commands.lock().unwrap();
        pending.len()
    }

    /// Check if there are any pending commands
    pub fn has_pending_commands(&self) -> bool {
        self.pending_command_count() > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{WindowManager, VisualBlockManager};
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn test_command_integration_creation() {
        let window_manager = Arc::new(Mutex::new(WindowManager::new().unwrap()));
        let visual_manager = Arc::new(Mutex::new(VisualBlockManager::new()));
        
        let integration = CommandIntegration::new(window_manager, visual_manager);
        assert_eq!(integration.pending_command_count(), 0);
        assert!(!integration.has_pending_commands());
    }

    #[test]
    fn test_command_validation() {
        let window_manager = Arc::new(Mutex::new(WindowManager::new().unwrap()));
        let visual_manager = Arc::new(Mutex::new(VisualBlockManager::new()));
        let integration = CommandIntegration::new(window_manager, visual_manager);

        // Valid command
        assert!(integration.executor().validate_command("ls -la").is_ok());

        // Empty command
        assert!(integration.executor().validate_command("").is_err());
        assert!(integration.executor().validate_command("   ").is_err());

        // Dangerous command
        assert!(integration.executor().validate_command("rm -rf /").is_err());
    }

    #[test]
    fn test_risk_assessment() {
        let window_manager = Arc::new(Mutex::new(WindowManager::new().unwrap()));
        let visual_manager = Arc::new(Mutex::new(VisualBlockManager::new()));
        let integration = CommandIntegration::new(window_manager, visual_manager);

        // Safe command
        assert!(integration.executor().is_safe_command("ls -la"));
        assert!(integration.executor().is_safe_command("echo hello"));

        // Risky command
        assert!(!integration.executor().is_safe_command("rm -rf /"));
        assert!(!integration.executor().is_safe_command("sudo rm file"));
    }

    #[test]
    fn test_cleanup_functionality() {
        let window_manager = Arc::new(Mutex::new(WindowManager::new().unwrap()));
        let visual_manager = Arc::new(Mutex::new(VisualBlockManager::new()));
        let integration = CommandIntegration::new(window_manager, visual_manager);

        // Add some mock pending commands
        {
            let mut pending = integration.pending_commands.lock().unwrap();
            
            let mut approved_cmd = CommandBlock {
                command: "ls".to_string(),
                working_directory: "/tmp".to_string(),
                description: "List files".to_string(),
                risk_level: crate::agent::RiskLevel::Low,
                approval_status: ApprovalStatus::Approved,
            };
            
            let pending_cmd = CommandBlock {
                command: "pwd".to_string(),
                working_directory: "/tmp".to_string(),
                description: "Print directory".to_string(),
                risk_level: crate::agent::RiskLevel::Low,
                approval_status: ApprovalStatus::Pending,
            };

            pending.insert("approved".to_string(), approved_cmd);
            pending.insert("pending".to_string(), pending_cmd);
        }

        assert_eq!(integration.pending_command_count(), 2);

        integration.cleanup_completed_commands();

        // Only pending command should remain
        assert_eq!(integration.pending_command_count(), 1);
        
        let pending = integration.pending_commands.lock().unwrap();
        assert!(pending.contains_key("pending"));
        assert!(!pending.contains_key("approved"));
    }
}