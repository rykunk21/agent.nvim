/// Demonstration of the command execution and approval system
/// 
/// This module shows how to use the complete command approval workflow
/// including UI integration, risk assessment, and error handling.

use crate::agent::{CommandWorkflow, CommandBlock, ApprovalStatus, RiskLevel};
use crate::utils::error_handling::PluginResult;
use neovim_lib::Neovim;
use std::path::PathBuf;
use std::time::Duration;

/// Demo scenarios for command approval system
pub struct CommandApprovalDemo {
    workflow: CommandWorkflow,
}

impl CommandApprovalDemo {
    /// Create a new demo instance
    pub fn new() -> PluginResult<Self> {
        Ok(CommandApprovalDemo {
            workflow: CommandWorkflow::new()?,
        })
    }

    /// Initialize the demo with Neovim
    pub fn initialize(&mut self, neovim: &mut Neovim) -> PluginResult<()> {
        self.workflow.initialize(neovim)?;
        self.workflow.show_interface(neovim)?;
        Ok(())
    }

    /// Demo 1: Safe command execution (auto-approved)
    pub async fn demo_safe_command(&mut self, neovim: &mut Neovim) -> PluginResult<()> {
        println!("=== Demo 1: Safe Command Execution ===");
        
        let result = self.workflow.execute_safe_command(
            neovim,
            "ls -la",
            "List files in current directory"
        ).await?;

        match result.approval_status {
            ApprovalStatus::Executed { output } => {
                if output.success {
                    println!("✅ Safe command executed successfully");
                    println!("Output: {}", output.stdout);
                } else {
                    println!("❌ Safe command failed: {}", output.stderr);
                }
            }
            _ => println!("⚠️ Unexpected command status"),
        }

        Ok(())
    }

    /// Demo 2: Medium risk command requiring approval
    pub async fn demo_medium_risk_command(&mut self, neovim: &mut Neovim) -> PluginResult<CommandBlock> {
        println!("=== Demo 2: Medium Risk Command ===");
        
        let command_block = self.workflow.execute_command_with_approval(
            neovim,
            "rm temp_file.txt",
            "Remove temporary file"
        ).await?;

        println!("Command presented for approval:");
        println!("  Command: {}", command_block.command);
        println!("  Risk Level: {:?}", command_block.risk_level);
        println!("  Status: {:?}", command_block.approval_status);

        Ok(command_block)
    }

    /// Demo 3: High risk command with detailed warning
    pub async fn demo_high_risk_command(&mut self, neovim: &mut Neovim) -> PluginResult<CommandBlock> {
        println!("=== Demo 3: High Risk Command ===");
        
        let command_block = self.workflow.execute_command_with_approval(
            neovim,
            "sudo rm -rf /tmp/dangerous_folder",
            "Remove system folder with elevated privileges"
        ).await?;

        println!("⚠️ HIGH RISK COMMAND DETECTED:");
        println!("  Command: {}", command_block.command);
        println!("  Risk Level: {:?}", command_block.risk_level);
        println!("  Description: {}", command_block.description);
        println!("  Status: {:?}", command_block.approval_status);

        Ok(command_block)
    }

    /// Demo 4: Command validation and rejection
    pub fn demo_command_validation(&self) -> PluginResult<()> {
        println!("=== Demo 4: Command Validation ===");

        let test_commands = vec![
            ("echo hello", "Valid safe command"),
            ("", "Empty command (should fail)"),
            (":(){ :|:& };:", "Fork bomb (should fail)"),
            ("ls -la", "Valid list command"),
            ("rm -rf /", "Dangerous delete (should fail)"),
        ];

        for (command, description) in test_commands {
            match self.workflow.executor().validate_command(command) {
                Ok(_) => {
                    let risk = self.workflow.executor().is_safe_command(command);
                    println!("✅ {}: {} (Safe: {})", description, command, risk);
                }
                Err(e) => {
                    println!("❌ {}: {} - Error: {}", description, command, e);
                }
            }
        }

        Ok(())
    }

    /// Demo 5: Working directory management
    pub fn demo_working_directory(&mut self) -> PluginResult<()> {
        println!("=== Demo 5: Working Directory Management ===");

        // Get current directory
        let current_dir = std::env::current_dir()?;
        println!("Current directory: {}", current_dir.display());

        // Try to set to temp directory
        let temp_dir = std::env::temp_dir();
        match self.workflow.set_working_directory(temp_dir.clone()) {
            Ok(_) => println!("✅ Successfully set working directory to: {}", temp_dir.display()),
            Err(e) => println!("❌ Failed to set working directory: {}", e),
        }

        // Try to set to invalid directory
        let invalid_dir = PathBuf::from("/nonexistent/directory/path");
        match self.workflow.set_working_directory(invalid_dir.clone()) {
            Ok(_) => println!("⚠️ Unexpectedly succeeded setting invalid directory"),
            Err(e) => println!("✅ Correctly rejected invalid directory: {}", e),
        }

        // Restore original directory
        self.workflow.set_working_directory(current_dir)?;
        println!("✅ Restored original working directory");

        Ok(())
    }

    /// Demo 6: Timeout and error handling
    pub fn demo_timeout_handling(&mut self) -> PluginResult<()> {
        println!("=== Demo 6: Timeout and Error Handling ===");

        // Set a short timeout for demonstration
        self.workflow.set_timeout(Duration::from_secs(5));
        println!("✅ Set command timeout to 5 seconds");

        // Set a longer timeout for normal operations
        self.workflow.set_timeout(Duration::from_secs(300));
        println!("✅ Restored normal timeout (5 minutes)");

        Ok(())
    }

    /// Demo 7: Workflow statistics and monitoring
    pub fn demo_workflow_statistics(&self) -> PluginResult<()> {
        println!("=== Demo 7: Workflow Statistics ===");

        let stats = self.workflow.get_statistics();
        
        println!("Workflow Statistics:");
        println!("  Pending Approvals: {}", stats.pending_approvals);
        println!("  Pending Commands: {}", stats.pending_commands);
        println!("  Active Operations: {}", stats.active_operations);
        println!("  Active Command Blocks: {}", stats.active_command_blocks);
        println!("  Interface Open: {}", stats.interface_open);
        println!("  Is Idle: {}", stats.is_idle());
        println!("  Total Active: {}", stats.total_active());

        Ok(())
    }

    /// Demo 8: Error scenarios and recovery
    pub async fn demo_error_scenarios(&mut self, neovim: &mut Neovim) -> PluginResult<()> {
        println!("=== Demo 8: Error Scenarios and Recovery ===");

        // Test command that will fail
        let result = self.workflow.execute_safe_command(
            neovim,
            "nonexistent_command_xyz",
            "Command that doesn't exist"
        ).await;

        match result {
            Ok(command_block) => {
                if let ApprovalStatus::Executed { output } = command_block.approval_status {
                    if !output.success {
                        println!("✅ Correctly handled command failure:");
                        println!("  Exit code: {}", output.exit_code);
                        println!("  Error: {}", output.stderr);
                        
                        let error_details = self.workflow.executor().get_error_details(&output);
                        println!("  Detailed error: {}", error_details);
                    }
                }
            }
            Err(e) => {
                println!("✅ Correctly caught execution error: {}", e);
            }
        }

        Ok(())
    }

    /// Run all demos in sequence
    pub async fn run_all_demos(&mut self, neovim: &mut Neovim) -> PluginResult<()> {
        println!("🚀 Starting Command Approval System Demo");
        println!("==========================================");

        // Initialize
        self.initialize(neovim)?;

        // Run demos
        self.demo_command_validation()?;
        self.demo_working_directory()?;
        self.demo_timeout_handling()?;
        self.demo_workflow_statistics()?;

        // Async demos
        self.demo_safe_command(neovim).await?;
        self.demo_error_scenarios(neovim).await?;

        // Interactive demos (would require user input in real scenario)
        let _medium_risk = self.demo_medium_risk_command(neovim).await?;
        let _high_risk = self.demo_high_risk_command(neovim).await?;

        // Final statistics
        println!("\n=== Final Statistics ===");
        self.demo_workflow_statistics()?;

        // Cleanup
        self.workflow.cleanup();
        println!("✅ Demo completed successfully!");

        Ok(())
    }

    /// Simulate user approval for demo purposes
    pub async fn simulate_approval(&mut self, neovim: &mut Neovim, block_id: &str) -> PluginResult<()> {
        println!("🤖 Simulating user approval for block: {}", block_id);
        self.workflow.handle_approval(neovim, block_id).await
    }

    /// Simulate user rejection for demo purposes
    pub async fn simulate_rejection(&mut self, neovim: &mut Neovim, block_id: &str) -> PluginResult<()> {
        println!("🤖 Simulating user rejection for block: {}", block_id);
        self.workflow.handle_rejection(neovim, block_id).await
    }

    /// Get workflow reference for advanced operations
    pub fn workflow(&self) -> &CommandWorkflow {
        &self.workflow
    }

    /// Get mutable workflow reference
    pub fn workflow_mut(&mut self) -> &mut CommandWorkflow {
        &mut self.workflow
    }
}

/// Helper function to create a demo command block for testing
pub fn create_demo_command_block(command: &str, risk_level: RiskLevel) -> CommandBlock {
    CommandBlock {
        command: command.to_string(),
        working_directory: std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("/"))
            .to_string_lossy()
            .to_string(),
        description: format!("Demo command: {}", command),
        risk_level,
        approval_status: ApprovalStatus::Pending,
    }
}

/// Helper function to format demo output
pub fn format_demo_output(title: &str, content: &str) -> String {
    let border = "=".repeat(title.len() + 4);
    format!("{}\n= {} =\n{}\n{}\n", border, title, border, content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_demo_creation() {
        let demo = CommandApprovalDemo::new();
        assert!(demo.is_ok());
    }

    #[test]
    fn test_demo_command_block_creation() {
        let command_block = create_demo_command_block("ls -la", RiskLevel::Low);
        assert_eq!(command_block.command, "ls -la");
        assert_eq!(command_block.risk_level, RiskLevel::Low);
        assert!(matches!(command_block.approval_status, ApprovalStatus::Pending));
    }

    #[test]
    fn test_demo_output_formatting() {
        let output = format_demo_output("Test", "This is test content");
        assert!(output.contains("Test"));
        assert!(output.contains("This is test content"));
        assert!(output.contains("="));
    }

    #[tokio::test]
    async fn test_demo_validation() {
        let demo = CommandApprovalDemo::new().unwrap();
        
        // Test validation works
        let result = demo.demo_command_validation();
        assert!(result.is_ok());
    }

    #[test]
    fn test_demo_working_directory() {
        let mut demo = CommandApprovalDemo::new().unwrap();
        
        // Test working directory management
        let result = demo.demo_working_directory();
        assert!(result.is_ok());
    }

    #[test]
    fn test_demo_timeout_handling() {
        let mut demo = CommandApprovalDemo::new().unwrap();
        
        // Test timeout configuration
        let result = demo.demo_timeout_handling();
        assert!(result.is_ok());
    }

    #[test]
    fn test_demo_statistics() {
        let demo = CommandApprovalDemo::new().unwrap();
        
        // Test statistics gathering
        let result = demo.demo_workflow_statistics();
        assert!(result.is_ok());
    }
}