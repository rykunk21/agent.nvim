use crate::agent::{CommandBlock, ApprovalStatus, RiskLevel, CommandOutput};
use crate::ui::{WindowManager, VisualBlockManager};
use crate::utils::error_handling::{PluginResult, PluginError};
use neovim_lib::{Neovim, NeovimApi};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use uuid::Uuid;

/// UI handler for command approval workflow
pub struct CommandApprovalUI {
    window_manager: Arc<Mutex<WindowManager>>,
    visual_block_manager: Arc<Mutex<VisualBlockManager>>,
    active_approvals: HashMap<String, CommandApprovalState>,
    keybinding_setup: bool,
}

/// State tracking for command approvals
#[derive(Debug, Clone)]
struct CommandApprovalState {
    command_block: CommandBlock,
    block_id: String,
    created_at: std::time::Instant,
}

impl CommandApprovalUI {
    pub fn new(
        window_manager: Arc<Mutex<WindowManager>>,
        visual_block_manager: Arc<Mutex<VisualBlockManager>>,
    ) -> Self {
        CommandApprovalUI {
            window_manager,
            visual_block_manager,
            active_approvals: HashMap::new(),
            keybinding_setup: false,
        }
    }

    /// Show command approval interface
    pub fn show_command_approval(
        &mut self,
        neovim: &mut Neovim,
        command_block: CommandBlock,
    ) -> PluginResult<String> {
        // Generate unique block ID
        let block_id = format!("cmd_approval_{}", Uuid::new_v4());

        // Create command approval window if not already open
        {
            let mut window_manager = self.window_manager.lock().unwrap();
            if !window_manager.is_interface_open() {
                window_manager.create_agent_interface(neovim)?;
            }
            window_manager.create_command_approval_window(neovim)?;
        }

        // Show command block in visual manager
        {
            let mut visual_manager = self.visual_block_manager.lock().unwrap();
            visual_manager.show_command_approval(neovim, command_block.clone())?;
        }

        // Setup keybindings if not already done
        if !self.keybinding_setup {
            self.setup_global_keybindings(neovim)?;
            self.keybinding_setup = true;
        }

        // Store approval state
        let approval_state = CommandApprovalState {
            command_block,
            block_id: block_id.clone(),
            created_at: std::time::Instant::now(),
        };
        self.active_approvals.insert(block_id.clone(), approval_state);

        // Focus the command approval window
        {
            let mut window_manager = self.window_manager.lock().unwrap();
            window_manager.focus_window(neovim, "command")?;
        }

        Ok(block_id)
    }

    /// Handle command approval
    pub fn approve_command(&mut self, neovim: &mut Neovim, block_id: &str) -> PluginResult<CommandBlock> {
        if let Some(mut approval_state) = self.active_approvals.remove(block_id) {
            approval_state.command_block.approval_status = ApprovalStatus::Approved;

            // Update visual display
            {
                let mut visual_manager = self.visual_block_manager.lock().unwrap();
                visual_manager.update_command_status(neovim, block_id, approval_state.command_block.clone())?;
            }

            // Show approval confirmation
            self.show_approval_feedback(neovim, "✅ Command APPROVED - Executing...")?;

            Ok(approval_state.command_block)
        } else {
            Err(PluginError::window(&format!("Command approval not found: {}", block_id)))
        }
    }

    /// Handle command rejection
    pub fn reject_command(&mut self, neovim: &mut Neovim, block_id: &str) -> PluginResult<CommandBlock> {
        if let Some(mut approval_state) = self.active_approvals.remove(block_id) {
            approval_state.command_block.approval_status = ApprovalStatus::Rejected;

            // Update visual display
            {
                let mut visual_manager = self.visual_block_manager.lock().unwrap();
                visual_manager.update_command_status(neovim, block_id, approval_state.command_block.clone())?;
            }

            // Show rejection confirmation
            self.show_approval_feedback(neovim, "❌ Command REJECTED - Cancelled")?;

            // Close command approval window after a brief delay
            self.close_approval_interface_delayed(neovim)?;

            Ok(approval_state.command_block)
        } else {
            Err(PluginError::window(&format!("Command approval not found: {}", block_id)))
        }
    }

    /// Update command execution status
    pub fn update_execution_status(
        &mut self,
        neovim: &mut Neovim,
        block_id: &str,
        output: CommandOutput,
    ) -> PluginResult<()> {
        if let Some(approval_state) = self.active_approvals.get_mut(block_id) {
            approval_state.command_block.approval_status = ApprovalStatus::Executed { output: output.clone() };

            // Update visual display
            {
                let mut visual_manager = self.visual_block_manager.lock().unwrap();
                visual_manager.update_command_status(neovim, block_id, approval_state.command_block.clone())?;
            }

            // Show execution result
            if output.success {
                self.show_approval_feedback(neovim, "🎉 Command completed successfully")?;
            } else {
                self.show_approval_feedback(neovim, &format!("⚠️ Command failed (exit code: {})", output.exit_code))?;
            }

            // Auto-close after showing results
            self.close_approval_interface_delayed(neovim)?;
        }

        Ok(())
    }

    /// Close command approval interface
    pub fn close_approval_interface(&mut self, neovim: &mut Neovim) -> PluginResult<()> {
        // Close command approval window
        {
            let mut window_manager = self.window_manager.lock().unwrap();
            window_manager.close_command_approval_window(neovim)?;
        }

        // Clear any remaining visual blocks
        {
            let mut visual_manager = self.visual_block_manager.lock().unwrap();
            for block_id in self.active_approvals.keys() {
                let _ = visual_manager.remove_block(neovim, block_id);
            }
        }

        // Clear active approvals
        self.active_approvals.clear();

        Ok(())
    }

    /// Setup global keybindings for command approval
    fn setup_global_keybindings(&self, neovim: &mut Neovim) -> PluginResult<()> {
        // Create Lua functions for approval/rejection
        let lua_setup = r#"
        local nvim_spec_agent = require('nvim-spec-agent')
        
        -- Global approval function
        function _G.nvim_spec_agent_approve_current()
            nvim_spec_agent.approve_current_command()
        end
        
        -- Global rejection function
        function _G.nvim_spec_agent_reject_current()
            nvim_spec_agent.reject_current_command()
        end
        
        -- Setup buffer-local keybindings for command approval window
        function _G.nvim_spec_agent_setup_approval_keys(bufnr)
            local opts = { noremap = true, silent = true, buffer = bufnr }
            vim.keymap.set('n', 'a', _G.nvim_spec_agent_approve_current, opts)
            vim.keymap.set('n', 'r', _G.nvim_spec_agent_reject_current, opts)
            vim.keymap.set('n', '<CR>', _G.nvim_spec_agent_approve_current, opts)
            vim.keymap.set('n', '<Esc>', _G.nvim_spec_agent_reject_current, opts)
        end
        "#;

        neovim.execute_lua(lua_setup, vec![])?;

        Ok(())
    }

    /// Show feedback message to user
    fn show_approval_feedback(&self, neovim: &mut Neovim, message: &str) -> PluginResult<()> {
        let lua_code = format!(
            r#"
            vim.notify("{}", vim.log.levels.INFO, {{
                title = "Command Approval",
                timeout = 3000,
            }})
            "#,
            message.replace('"', r#"\""#)
        );

        neovim.execute_lua(&lua_code, vec![])?;
        Ok(())
    }

    /// Close approval interface with delay
    fn close_approval_interface_delayed(&self, neovim: &mut Neovim) -> PluginResult<()> {
        let lua_code = r#"
        vim.defer_fn(function()
            require('nvim-spec-agent').close_command_approval()
        end, 2000) -- 2 second delay
        "#;

        neovim.execute_lua(lua_code, vec![])?;
        Ok(())
    }

    /// Get current active approval (if any)
    pub fn get_current_approval(&self) -> Option<&CommandApprovalState> {
        // Return the most recent approval
        self.active_approvals.values()
            .max_by_key(|state| state.created_at)
    }

    /// Check if there are any pending approvals
    pub fn has_pending_approvals(&self) -> bool {
        !self.active_approvals.is_empty()
    }

    /// Get count of pending approvals
    pub fn pending_approval_count(&self) -> usize {
        self.active_approvals.len()
    }

    /// Clean up old approvals (older than specified duration)
    pub fn cleanup_old_approvals(&mut self, max_age: std::time::Duration) {
        let now = std::time::Instant::now();
        self.active_approvals.retain(|_, state| {
            now.duration_since(state.created_at) <= max_age
        });
    }

    /// Get approval by block ID
    pub fn get_approval(&self, block_id: &str) -> Option<&CommandApprovalState> {
        self.active_approvals.get(block_id)
    }

    /// Format command for display
    pub fn format_command_display(&self, command_block: &CommandBlock) -> Vec<String> {
        let mut lines = Vec::new();

        // Risk level header
        let risk_indicator = match command_block.risk_level {
            RiskLevel::Low => "🟢 LOW RISK",
            RiskLevel::Medium => "🟡 MEDIUM RISK",
            RiskLevel::High => "🔴 HIGH RISK - CAUTION!",
        };

        lines.push(format!("┌─ {} ─┐", risk_indicator));
        lines.push(format!("│ Command: {}", command_block.command));
        lines.push(format!("│ Directory: {}", command_block.working_directory));
        
        if !command_block.description.is_empty() {
            lines.push(format!("│ Description: {}", command_block.description));
        }

        lines.push("└─────────────────────────────────────────┘".to_string());
        lines.push(String::new());

        // Status-specific information
        match &command_block.approval_status {
            ApprovalStatus::Pending => {
                lines.push("⏳ Awaiting your decision...".to_string());
                lines.push(String::new());
                lines.push("Press 'a' to APPROVE  |  Press 'r' to REJECT".to_string());
                lines.push("Press ENTER to approve  |  Press ESC to reject".to_string());
            }
            ApprovalStatus::Approved => {
                lines.push("✅ APPROVED - Executing command...".to_string());
            }
            ApprovalStatus::Rejected => {
                lines.push("❌ REJECTED - Command cancelled".to_string());
            }
            ApprovalStatus::Executed { output } => {
                if output.success {
                    lines.push("🎉 Command completed successfully!".to_string());
                } else {
                    lines.push(format!("⚠️ Command failed with exit code: {}", output.exit_code));
                }

                if !output.stdout.is_empty() {
                    lines.push(String::new());
                    lines.push("📤 Output:".to_string());
                    for line in output.stdout.lines().take(10) {
                        lines.push(format!("  {}", line));
                    }
                }

                if !output.stderr.is_empty() {
                    lines.push(String::new());
                    lines.push("❗ Errors:".to_string());
                    for line in output.stderr.lines().take(5) {
                        lines.push(format!("  {}", line));
                    }
                }
            }
        }

        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{CommandBlock, ApprovalStatus, RiskLevel};
    use crate::ui::{WindowManager, VisualBlockManager};
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_command_approval_ui_creation() {
        let window_manager = Arc::new(Mutex::new(WindowManager::new().unwrap()));
        let visual_manager = Arc::new(Mutex::new(VisualBlockManager::new()));
        
        let approval_ui = CommandApprovalUI::new(window_manager, visual_manager);
        assert!(!approval_ui.has_pending_approvals());
        assert_eq!(approval_ui.pending_approval_count(), 0);
    }

    #[test]
    fn test_command_display_formatting() {
        let window_manager = Arc::new(Mutex::new(WindowManager::new().unwrap()));
        let visual_manager = Arc::new(Mutex::new(VisualBlockManager::new()));
        let approval_ui = CommandApprovalUI::new(window_manager, visual_manager);

        let command_block = CommandBlock {
            command: "ls -la".to_string(),
            working_directory: "/tmp".to_string(),
            description: "List files in detail".to_string(),
            risk_level: RiskLevel::Low,
            approval_status: ApprovalStatus::Pending,
        };

        let display_lines = approval_ui.format_command_display(&command_block);
        
        assert!(!display_lines.is_empty());
        assert!(display_lines.iter().any(|line| line.contains("ls -la")));
        assert!(display_lines.iter().any(|line| line.contains("/tmp")));
        assert!(display_lines.iter().any(|line| line.contains("List files in detail")));
        assert!(display_lines.iter().any(|line| line.contains("LOW RISK")));
    }

    #[test]
    fn test_approval_state_tracking() {
        let window_manager = Arc::new(Mutex::new(WindowManager::new().unwrap()));
        let visual_manager = Arc::new(Mutex::new(VisualBlockManager::new()));
        let mut approval_ui = CommandApprovalUI::new(window_manager, visual_manager);

        let command_block = CommandBlock {
            command: "echo test".to_string(),
            working_directory: "/tmp".to_string(),
            description: "Test command".to_string(),
            risk_level: RiskLevel::Low,
            approval_status: ApprovalStatus::Pending,
        };

        // Simulate adding an approval
        let approval_state = CommandApprovalState {
            command_block,
            block_id: "test_block".to_string(),
            created_at: std::time::Instant::now(),
        };

        approval_ui.active_approvals.insert("test_block".to_string(), approval_state);

        assert!(approval_ui.has_pending_approvals());
        assert_eq!(approval_ui.pending_approval_count(), 1);
        assert!(approval_ui.get_approval("test_block").is_some());
        assert!(approval_ui.get_current_approval().is_some());
    }

    #[test]
    fn test_cleanup_old_approvals() {
        let window_manager = Arc::new(Mutex::new(WindowManager::new().unwrap()));
        let visual_manager = Arc::new(Mutex::new(VisualBlockManager::new()));
        let mut approval_ui = CommandApprovalUI::new(window_manager, visual_manager);

        // Add an old approval
        let old_approval = CommandApprovalState {
            command_block: CommandBlock {
                command: "old command".to_string(),
                working_directory: "/tmp".to_string(),
                description: "Old test".to_string(),
                risk_level: RiskLevel::Low,
                approval_status: ApprovalStatus::Pending,
            },
            block_id: "old_block".to_string(),
            created_at: std::time::Instant::now() - std::time::Duration::from_secs(3600), // 1 hour ago
        };

        approval_ui.active_approvals.insert("old_block".to_string(), old_approval);
        assert_eq!(approval_ui.pending_approval_count(), 1);

        // Cleanup approvals older than 30 minutes
        approval_ui.cleanup_old_approvals(std::time::Duration::from_secs(1800));
        assert_eq!(approval_ui.pending_approval_count(), 0);
    }
}