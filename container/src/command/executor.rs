use anyhow::Result;
use log::info;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;
use std::collections::HashMap;

/// Command block for approval workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandBlock {
    pub id: String,
    pub command: String,
    pub working_directory: String,
    pub description: String,
    pub risk_level: RiskLevel,
    pub approval_status: ApprovalStatus,
}

/// Command approval status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

/// Command executor for handling command execution through container
pub struct CommandExecutor {
    pending_commands: HashMap<String, CommandBlock>,
    command_timeout: Duration,
}

impl CommandExecutor {
    pub fn new() -> Self {
        CommandExecutor {
            pending_commands: HashMap::new(),
            command_timeout: Duration::from_secs(300), // 5 minute default timeout
        }
    }

    /// Create a command block for approval
    pub fn create_command_block(
        &mut self,
        command: String,
        working_directory: String,
        description: String,
    ) -> Result<String> {
        let block_id = Uuid::new_v4().to_string();
        
        let risk_level = self.assess_risk_level(&command);
        
        let command_block = CommandBlock {
            id: block_id.clone(),
            command,
            working_directory,
            description,
            risk_level,
            approval_status: ApprovalStatus::Pending,
        };

        info!("Created command block: {}", block_id);
        self.pending_commands.insert(block_id.clone(), command_block);
        Ok(block_id)
    }

    /// Get a command block by ID
    pub fn get_command_block(&self, block_id: &str) -> Result<CommandBlock> {
        self.pending_commands
            .get(block_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Command block not found: {}", block_id))
    }

    /// Approve a command
    pub fn approve_command(&mut self, block_id: &str) -> Result<()> {
        let command_block = self
            .pending_commands
            .get_mut(block_id)
            .ok_or_else(|| anyhow::anyhow!("Command block not found: {}", block_id))?;

        command_block.approval_status = ApprovalStatus::Approved;
        info!("Approved command: {}", block_id);
        Ok(())
    }

    /// Reject a command
    pub fn reject_command(&mut self, block_id: &str) -> Result<()> {
        let command_block = self
            .pending_commands
            .get_mut(block_id)
            .ok_or_else(|| anyhow::anyhow!("Command block not found: {}", block_id))?;

        command_block.approval_status = ApprovalStatus::Rejected;
        info!("Rejected command: {}", block_id);
        Ok(())
    }

    /// Execute an approved command
    pub fn execute_command(&mut self, block_id: &str) -> Result<CommandOutput> {
        let command_block = self
            .pending_commands
            .get_mut(block_id)
            .ok_or_else(|| anyhow::anyhow!("Command block not found: {}", block_id))?;

        if !matches!(command_block.approval_status, ApprovalStatus::Approved) {
            return Err(anyhow::anyhow!("Command not approved for execution"));
        }

        // Validate command before execution
        self.validate_command(&command_block.command)?;

        // Execute the command
        let output = self.execute_command_internal(&command_block)?;

        // Update command block with execution result
        command_block.approval_status = ApprovalStatus::Executed {
            output: output.clone(),
        };

        info!("Executed command: {} (exit code: {})", block_id, output.exit_code);
        Ok(output)
    }

    /// Execute command internally
    fn execute_command_internal(&self, command_block: &CommandBlock) -> Result<CommandOutput> {
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

    /// Validate command before execution
    pub fn validate_command(&self, command: &str) -> Result<()> {
        if command.trim().is_empty() {
            return Err(anyhow::anyhow!("Command cannot be empty"));
        }

        // Check for potentially dangerous patterns
        let forbidden_patterns = [
            ":(){ :|:& };:", // Fork bomb
            "rm -rf /",      // Delete root
            "mkfs",          // Format filesystem
        ];

        for pattern in &forbidden_patterns {
            if command.contains(pattern) {
                return Err(anyhow::anyhow!(
                    "Command contains forbidden pattern: {}",
                    pattern
                ));
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

    /// Set command execution timeout
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.command_timeout = timeout;
    }

    /// Get command execution timeout
    pub fn get_timeout(&self) -> Duration {
        self.command_timeout
    }

    /// Clean up executed command blocks
    pub fn cleanup_executed_commands(&mut self) {
        self.pending_commands.retain(|_, block| {
            !matches!(block.approval_status, ApprovalStatus::Executed { .. })
        });
    }

    /// List all pending commands
    pub fn list_pending_commands(&self) -> Vec<CommandBlock> {
        self.pending_commands
            .values()
            .filter(|block| matches!(block.approval_status, ApprovalStatus::Pending))
            .cloned()
            .collect()
    }
}

impl Default for CommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}
