use serde::{Deserialize, Serialize};
use std::process::Command;
use std::io::Result;
use std::path::PathBuf;
use std::env;
use crate::utils::error_handling::{PluginResult, PluginError};
use neovim_lib::{Neovim, NeovimApi};
use crate::ui::{WindowManager, VisualBlockManager};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::timeout;

/// Executes commands with approval workflow
pub struct CommandExecutor {
    window_manager: Arc<Mutex<WindowManager>>,
    visual_block_manager: Arc<Mutex<VisualBlockManager>>,
    working_directory: PathBuf,
    command_timeout: Duration,
}

impl CommandExecutor {
    pub fn new(
        window_manager: Arc<Mutex<WindowManager>>,
        visual_block_manager: Arc<Mutex<VisualBlockManager>>,
    ) -> Self {
        CommandExecutor {
            window_manager,
            visual_block_manager,
            working_directory: env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            command_timeout: Duration::from_secs(300), // 5 minute default timeout
        }
    }

    /// Set the working directory for command execution
    pub fn set_working_directory(&mut self, path: PathBuf) -> PluginResult<()> {
        if path.exists() && path.is_dir() {
            self.working_directory = path;
            Ok(())
        } else {
            Err(PluginError::command(&format!("Invalid working directory: {}", path.display())))
        }
    }

    /// Set command execution timeout
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.command_timeout = timeout;
    }

    /// Get current working directory
    pub fn get_working_directory(&self) -> &PathBuf {
        &self.working_directory
    }

    /// Present a command for user approval with UI integration
    pub async fn present_command_for_approval(
        &self,
        neovim: &mut Neovim,
        command: &str,
        description: &str,
    ) -> PluginResult<String> {
        let working_dir = self.working_directory.to_string_lossy().to_string();
        
        let command_block = CommandBlock {
            command: command.to_string(),
            working_directory: working_dir,
            description: description.to_string(),
            risk_level: self.assess_risk_level(command),
            approval_status: ApprovalStatus::Pending,
        };

        // Create command approval window
        {
            let mut window_manager = self.window_manager.lock().unwrap();
            window_manager.create_command_approval_window(neovim)?;
        }

        // Show command block in UI
        let block_id = {
            let mut visual_manager = self.visual_block_manager.lock().unwrap();
            visual_manager.show_command_approval(neovim, command_block)?
        };

        // Set up keybindings for approval/rejection
        self.setup_approval_keybindings(neovim, &block_id)?;

        Ok(block_id)
    }

    /// Present a command for user approval (synchronous version for compatibility)
    pub fn present_command(&self, command: &str, working_directory: &str, description: &str) -> CommandBlock {
        CommandBlock {
            command: command.to_string(),
            working_directory: working_directory.to_string(),
            description: description.to_string(),
            risk_level: self.assess_risk_level(command),
            approval_status: ApprovalStatus::Pending,
        }
    }

    /// Execute an approved command with full error handling and output capture
    pub async fn execute_command_async(
        &self,
        neovim: &mut Neovim,
        command_block: &mut CommandBlock,
        block_id: &str,
    ) -> PluginResult<()> {
        if !matches!(command_block.approval_status, ApprovalStatus::Approved) {
            return Err(PluginError::command("Command not approved for execution"));
        }

        // Update UI to show execution in progress
        command_block.approval_status = ApprovalStatus::Approved;
        {
            let mut visual_manager = self.visual_block_manager.lock().unwrap();
            visual_manager.update_command_status(neovim, block_id, command_block.clone())?;
        }

        // Execute command with timeout
        let command_future = self.execute_command_with_timeout(command_block);
        
        match timeout(self.command_timeout, command_future).await {
            Ok(result) => {
                match result {
                    Ok(output) => {
                        command_block.approval_status = ApprovalStatus::Executed { output };
                        
                        // Update UI with results
                        let mut visual_manager = self.visual_block_manager.lock().unwrap();
                        visual_manager.update_command_status(neovim, block_id, command_block.clone())?;
                        
                        Ok(())
                    }
                    Err(e) => {
                        let error_output = CommandOutput {
                            stdout: String::new(),
                            stderr: format!("Command execution failed: {}", e),
                            exit_code: -1,
                            success: false,
                        };
                        command_block.approval_status = ApprovalStatus::Executed { output: error_output };
                        
                        // Update UI with error
                        let mut visual_manager = self.visual_block_manager.lock().unwrap();
                        visual_manager.update_command_status(neovim, block_id, command_block.clone())?;
                        
                        Err(PluginError::command(&format!("Command execution failed: {}", e)))
                    }
                }
            }
            Err(_) => {
                let timeout_output = CommandOutput {
                    stdout: String::new(),
                    stderr: format!("Command timed out after {} seconds", self.command_timeout.as_secs()),
                    exit_code: -1,
                    success: false,
                };
                command_block.approval_status = ApprovalStatus::Executed { output: timeout_output };
                
                // Update UI with timeout error
                let mut visual_manager = self.visual_block_manager.lock().unwrap();
                visual_manager.update_command_status(neovim, block_id, command_block.clone())?;
                
                Err(PluginError::command("Command execution timed out"))
            }
        }
    }

    /// Execute an approved command (synchronous version for compatibility)
    pub fn execute_command(&self, command_block: &mut CommandBlock) -> Result<()> {
        if !matches!(command_block.approval_status, ApprovalStatus::Approved) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Command not approved for execution"
            ));
        }

        let output = Command::new("sh")
            .arg("-c")
            .arg(&command_block.command)
            .current_dir(&command_block.working_directory)
            .output()?;

        command_block.approval_status = ApprovalStatus::Executed {
            output: CommandOutput {
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                exit_code: output.status.code().unwrap_or(-1),
                success: output.status.success(),
            }
        };

        Ok(())
    }

    /// Handle command approval
    pub async fn approve_command(
        &self,
        neovim: &mut Neovim,
        block_id: &str,
    ) -> PluginResult<()> {
        // Get the command block and update its status
        let mut command_block = {
            let visual_manager = self.visual_block_manager.lock().unwrap();
            // Extract command block from visual manager
            // This is a simplified version - in practice, you'd need to get the actual block
            self.get_command_block_by_id(block_id)?
        };

        command_block.approval_status = ApprovalStatus::Approved;

        // Execute the command
        self.execute_command_async(neovim, &mut command_block, block_id).await?;

        Ok(())
    }

    /// Handle command rejection
    pub async fn reject_command(
        &self,
        neovim: &mut Neovim,
        block_id: &str,
    ) -> PluginResult<()> {
        // Get the command block and update its status
        let mut command_block = self.get_command_block_by_id(block_id)?;
        command_block.approval_status = ApprovalStatus::Rejected;

        // Update UI
        {
            let mut visual_manager = self.visual_block_manager.lock().unwrap();
            visual_manager.update_command_status(neovim, block_id, command_block)?;
        }

        // Close command approval window and return to normal mode
        {
            let mut window_manager = self.window_manager.lock().unwrap();
            window_manager.close_command_approval_window(neovim)?;
        }

        Ok(())
    }

    /// Execute command with proper error handling and output capture
    async fn execute_command_with_timeout(&self, command_block: &CommandBlock) -> Result<CommandOutput> {
        let output = Command::new("sh")
            .arg("-c")
            .arg(&command_block.command)
            .current_dir(&command_block.working_directory)
            .output()?;

        Ok(CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            success: output.status.success(),
        })
    }

    /// Setup keybindings for command approval/rejection
    fn setup_approval_keybindings(&self, neovim: &mut Neovim, block_id: &str) -> PluginResult<()> {
        // Set up 'a' key for approval
        let approve_cmd = format!(
            "nnoremap <buffer> a :lua require('nvim-spec-agent').approve_command('{}')<CR>",
            block_id
        );
        neovim.command(&approve_cmd)?;

        // Set up 'r' key for rejection
        let reject_cmd = format!(
            "nnoremap <buffer> r :lua require('nvim-spec-agent').reject_command('{}')<CR>",
            block_id
        );
        neovim.command(&reject_cmd)?;

        Ok(())
    }

    /// Get command block by ID (helper method)
    fn get_command_block_by_id(&self, _block_id: &str) -> PluginResult<CommandBlock> {
        // This is a placeholder - in a real implementation, you'd retrieve the actual block
        // from the visual block manager
        Ok(CommandBlock {
            command: String::new(),
            working_directory: String::new(),
            description: String::new(),
            risk_level: RiskLevel::Low,
            approval_status: ApprovalStatus::Pending,
        })
    }

    /// Validate command before execution
    pub fn validate_command(&self, command: &str) -> PluginResult<()> {
        if command.trim().is_empty() {
            return Err(PluginError::command("Command cannot be empty"));
        }

        // Check for potentially dangerous patterns
        let forbidden_patterns = [
            ":(){ :|:& };:", // Fork bomb
            "rm -rf /",      // Delete root
            "mkfs",          // Format filesystem
        ];

        for pattern in &forbidden_patterns {
            if command.contains(pattern) {
                return Err(PluginError::command(&format!(
                    "Command contains forbidden pattern: {}",
                    pattern
                )));
            }
        }

        Ok(())
    }

    /// Assess the risk level of a command
    fn assess_risk_level(&self, command: &str) -> RiskLevel {
        let high_risk_patterns = ["rm -rf", "sudo", "chmod 777", "dd if=", "mkfs", "> /dev/"];
        let medium_risk_patterns = ["rm ", "mv ", "cp ", "chmod", "chown", "kill", "pkill"];

        if high_risk_patterns.iter().any(|pattern| command.contains(pattern)) {
            RiskLevel::High
        } else if medium_risk_patterns.iter().any(|pattern| command.contains(pattern)) {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        }
    }

    /// Get detailed error information from command output
    pub fn get_error_details(&self, output: &CommandOutput) -> String {
        let mut details = Vec::new();

        if !output.success {
            details.push(format!("Exit code: {}", output.exit_code));
        }

        if !output.stderr.is_empty() {
            details.push(format!("Error output:\n{}", output.stderr));
        }

        if !output.stdout.is_empty() && !output.success {
            details.push(format!("Standard output:\n{}", output.stdout));
        }

        if details.is_empty() {
            "Command failed with no error details".to_string()
        } else {
            details.join("\n\n")
        }
    }

    /// Check if a command is safe to execute
    pub fn is_safe_command(&self, command: &str) -> bool {
        self.validate_command(command).is_ok() && 
        !matches!(self.assess_risk_level(command), RiskLevel::High)
    }
}

/// Command block for approval workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandBlock {
    pub command: String,
    pub working_directory: String,
    pub description: String,
    pub risk_level: RiskLevel,
    pub approval_status: ApprovalStatus,
}

/// Command approval status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Executed { output: CommandOutput },
}

/// Command execution output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub success: bool,
}

/// Risk level assessment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}