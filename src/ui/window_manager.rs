use neovim_lib::{Neovim, NeovimApi};
use crate::utils::error_handling::{PluginResult, PluginError};
use crate::ui::layout::{LayoutMode, WindowDimensions, WindowConfig, WindowState};
use crate::config::PersistenceManager;
use std::collections::HashMap;

/// Manages floating windows for the agent interface
pub struct WindowManager {
    pub state: WindowState,
    persistence: PersistenceManager,
    z_index_counter: i32,
    window_registry: HashMap<i32, WindowType>,
}

#[derive(Debug, Clone)]
enum WindowType {
    Chat,
    Input,
    CommandApproval,
}

impl WindowManager {
    pub fn new() -> PluginResult<Self> {
        let persistence = PersistenceManager::new(None)?;
        
        Ok(WindowManager {
            state: WindowState {
                chat_window: None,
                input_window: None,
                command_approval_window: None,
                layout_mode: LayoutMode::Normal,
                dimensions: WindowDimensions::default(),
            },
            persistence,
            z_index_counter: 100,
            window_registry: HashMap::new(),
        })
    }

    /// Initialize window manager with Neovim instance
    pub fn initialize(&mut self, neovim: &mut Neovim) -> PluginResult<()> {
        // Get terminal dimensions
        self.update_dimensions(neovim)?;
        
        // Load persisted window state if available
        self.load_window_state()?;
        
        // Set up autocmd for terminal resize
        self.setup_resize_handler(neovim)?;
        
        Ok(())
    }

    /// Create the main agent interface with two windows
    pub fn create_agent_interface(&mut self, neovim: &mut Neovim) -> PluginResult<()> {
        self.update_dimensions(neovim)?;
        
        // Create chat window (top window)
        self.create_chat_window(neovim)?;
        
        // Create input window (bottom window)
        self.create_input_window(neovim)?;
        
        // Focus input window for immediate typing
        if let Some(input_config) = &self.state.input_window {
            // Use Lua to set current window since the API method signature is different
            neovim.command(&format!("lua vim.api.nvim_set_current_win({})", input_config.window_id))?;
        }
        
        Ok(())
    }

    /// Update window dimensions based on terminal size
    fn update_dimensions(&mut self, neovim: &mut Neovim) -> PluginResult<()> {
        let columns = neovim.get_option("columns")?.as_i64().unwrap_or(80) as u32;
        let lines = neovim.get_option("lines")?.as_i64().unwrap_or(24) as u32;
        
        self.state.dimensions = WindowDimensions::new(columns, lines);
        Ok(())
    }

    /// Create the chat history window
    fn create_chat_window(&mut self, neovim: &mut Neovim) -> PluginResult<()> {
        let config = self.state.dimensions.calculate_chat_window_config();
        
        // Create buffer using Lua API
        let buffer_result = neovim.execute_lua("return vim.api.nvim_create_buf(false, true)", vec![])?;
        let buffer_id = buffer_result.as_i64().unwrap() as i32;
        
        // Create floating window using Lua API
        let lua_code = format!(
            r#"
            local opts = {{
                relative = 'editor',
                width = {},
                height = {},
                col = {},
                row = {},
                style = 'minimal',
                border = 'rounded'
            }}
            return vim.api.nvim_open_win({}, false, opts)
            "#,
            config.size.width,
            config.size.height,
            config.position.col,
            config.position.row,
            buffer_id
        );
        
        let window_result = neovim.execute_lua(&lua_code, vec![])?;
        let window_id = window_result.as_i64().unwrap() as i32;
        
        self.state.chat_window = Some(WindowConfig {
            buffer_id,
            window_id,
            position: config.position,
            size: config.size,
            z_index: config.z_index,
        });
        
        self.window_registry.insert(window_id, WindowType::Chat);
        
        Ok(())
    }

    /// Create the input window
    fn create_input_window(&mut self, neovim: &mut Neovim) -> PluginResult<()> {
        let config = self.state.dimensions.calculate_input_window_config();
        
        // Create buffer using Lua API
        let buffer_result = neovim.execute_lua("return vim.api.nvim_create_buf(false, true)", vec![])?;
        let buffer_id = buffer_result.as_i64().unwrap() as i32;
        
        // Create floating window using Lua API
        let lua_code = format!(
            r#"
            local opts = {{
                relative = 'editor',
                width = {},
                height = {},
                col = {},
                row = {},
                style = 'minimal',
                border = 'rounded'
            }}
            return vim.api.nvim_open_win({}, true, opts)
            "#,
            config.size.width,
            config.size.height,
            config.position.col,
            config.position.row,
            buffer_id
        );
        
        let window_result = neovim.execute_lua(&lua_code, vec![])?;
        let window_id = window_result.as_i64().unwrap() as i32;
        
        self.state.input_window = Some(WindowConfig {
            buffer_id,
            window_id,
            position: config.position,
            size: config.size,
            z_index: config.z_index,
        });
        
        self.window_registry.insert(window_id, WindowType::Input);
        
        Ok(())
    }

    /// Create command approval window
    pub fn create_command_approval_window(&mut self, neovim: &mut Neovim) -> PluginResult<()> {
        // Switch to command approval layout mode
        self.state.layout_mode = LayoutMode::CommandApproval;
        
        // Resize existing windows to make room
        self.resize_for_command_approval(neovim)?;
        
        let config = self.state.dimensions.calculate_command_approval_window_config();
        
        // Create buffer using Lua API
        let buffer_result = neovim.execute_lua("return vim.api.nvim_create_buf(false, true)", vec![])?;
        let buffer_id = buffer_result.as_i64().unwrap() as i32;
        
        // Create floating window using Lua API
        let lua_code = format!(
            r#"
            local opts = {{
                relative = 'editor',
                width = {},
                height = {},
                col = {},
                row = {},
                style = 'minimal',
                border = 'rounded',
                zindex = {}
            }}
            return vim.api.nvim_open_win({}, false, opts)
            "#,
            config.size.width,
            config.size.height,
            config.position.col,
            config.position.row,
            self.get_next_z_index(),
            buffer_id
        );
        
        let window_result = neovim.execute_lua(&lua_code, vec![])?;
        let window_id = window_result.as_i64().unwrap() as i32;
        
        self.state.command_approval_window = Some(WindowConfig {
            buffer_id,
            window_id,
            position: config.position,
            size: config.size,
            z_index: config.z_index,
        });
        
        self.window_registry.insert(window_id, WindowType::CommandApproval);
        
        Ok(())
    }

    /// Close command approval window and restore normal layout
    pub fn close_command_approval_window(&mut self, neovim: &mut Neovim) -> PluginResult<()> {
        if let Some(window_config) = &self.state.command_approval_window {
            // Close the window
            let lua_code = format!("vim.api.nvim_win_close({}, false)", window_config.window_id);
            neovim.execute_lua(&lua_code, vec![])?;
            
            // Remove from registry
            self.window_registry.remove(&window_config.window_id);
        }
        
        self.state.command_approval_window = None;
        self.state.layout_mode = LayoutMode::Normal;
        
        // Restore original window sizes
        self.resize_windows_for_layout(neovim)?;
        
        Ok(())
    }

    /// Handle terminal resize events
    pub fn handle_resize(&mut self, neovim: &mut Neovim) -> PluginResult<()> {
        self.update_dimensions(neovim)?;
        self.resize_windows_for_layout(neovim)?;
        Ok(())
    }

    /// Focus a specific window
    pub fn focus_window(&mut self, neovim: &mut Neovim, window_type: &str) -> PluginResult<()> {
        let window_id = match window_type {
            "chat" => self.state.chat_window.as_ref().map(|w| w.window_id),
            "input" => self.state.input_window.as_ref().map(|w| w.window_id),
            "command" => self.state.command_approval_window.as_ref().map(|w| w.window_id),
            _ => return Err(PluginError::window(&format!("Unknown window type: {}", window_type))),
        };

        if let Some(id) = window_id {
            let lua_code = format!("vim.api.nvim_set_current_win({})", id);
            neovim.execute_lua(&lua_code, vec![])?;
        }

        Ok(())
    }

    /// Close all agent windows
    pub fn close_all_windows(&mut self, neovim: &mut Neovim) -> PluginResult<()> {
        // Save current state before closing
        self.save_window_state()?;

        // Close all windows
        let windows = [&self.state.chat_window, &self.state.input_window, &self.state.command_approval_window];
        for window_option in windows.iter() {
            if let Some(window_config) = window_option {
                let lua_code = format!("vim.api.nvim_win_close({}, false)", window_config.window_id);
                let _ = neovim.execute_lua(&lua_code, vec![]); // Ignore errors for already closed windows
            }
        }

        // Clear state
        self.state.chat_window = None;
        self.state.input_window = None;
        self.state.command_approval_window = None;
        self.window_registry.clear();

        Ok(())
    }

    /// Check if agent interface is currently open
    pub fn is_interface_open(&self) -> bool {
        self.state.chat_window.is_some() && self.state.input_window.is_some()
    }

    /// Get next z-index for window layering
    fn get_next_z_index(&mut self) -> i32 {
        self.z_index_counter += 1;
        self.z_index_counter
    }

    /// Resize windows for command approval layout
    fn resize_for_command_approval(&mut self, neovim: &mut Neovim) -> PluginResult<()> {
        // Shrink chat window to make room for command approval
        if let Some(chat_window) = &mut self.state.chat_window {
            let new_config = self.state.dimensions.calculate_chat_window_config_with_command();
            
            let lua_code = format!(
                r#"
                vim.api.nvim_win_set_config({}, {{
                    relative = 'editor',
                    width = {},
                    height = {},
                    col = {},
                    row = {}
                }})
                "#,
                chat_window.window_id,
                new_config.size.width,
                new_config.size.height,
                new_config.position.col,
                new_config.position.row
            );
            
            neovim.execute_lua(&lua_code, vec![])?;
            
            chat_window.position = new_config.position;
            chat_window.size = new_config.size;
        }

        Ok(())
    }

    /// Resize windows based on current layout mode
    fn resize_windows_for_layout(&mut self, neovim: &mut Neovim) -> PluginResult<()> {
        match self.state.layout_mode {
            LayoutMode::Normal => {
                self.resize_chat_window(neovim)?;
                self.resize_input_window(neovim)?;
            }
            LayoutMode::CommandApproval => {
                self.resize_for_command_approval(neovim)?;
                self.resize_input_window(neovim)?;
            }
            LayoutMode::SpecNavigation => {
                // Future implementation for spec navigation layout
                self.resize_chat_window(neovim)?;
                self.resize_input_window(neovim)?;
            }
        }
        Ok(())
    }

    /// Resize chat window
    fn resize_chat_window(&mut self, neovim: &mut Neovim) -> PluginResult<()> {
        if let Some(chat_window) = &mut self.state.chat_window {
            let new_config = self.state.dimensions.calculate_chat_window_config();
            
            let lua_code = format!(
                r#"
                vim.api.nvim_win_set_config({}, {{
                    relative = 'editor',
                    width = {},
                    height = {},
                    col = {},
                    row = {}
                }})
                "#,
                chat_window.window_id,
                new_config.size.width,
                new_config.size.height,
                new_config.position.col,
                new_config.position.row
            );
            
            neovim.execute_lua(&lua_code, vec![])?;
            
            chat_window.position = new_config.position;
            chat_window.size = new_config.size;
        }
        Ok(())
    }

    /// Resize input window
    fn resize_input_window(&mut self, neovim: &mut Neovim) -> PluginResult<()> {
        if let Some(input_window) = &mut self.state.input_window {
            let new_config = self.state.dimensions.calculate_input_window_config();
            
            let lua_code = format!(
                r#"
                vim.api.nvim_win_set_config({}, {{
                    relative = 'editor',
                    width = {},
                    height = {},
                    col = {},
                    row = {}
                }})
                "#,
                input_window.window_id,
                new_config.size.width,
                new_config.size.height,
                new_config.position.col,
                new_config.position.row
            );
            
            neovim.execute_lua(&lua_code, vec![])?;
            
            input_window.position = new_config.position;
            input_window.size = new_config.size;
        }
        Ok(())
    }

    /// Setup resize handler for terminal resize events
    fn setup_resize_handler(&self, neovim: &mut Neovim) -> PluginResult<()> {
        let autocmd = r#"
        augroup NvimSpecAgentResize
            autocmd!
            autocmd VimResized * lua require('nvim-spec-agent').handle_resize()
        augroup END
        "#;
        
        neovim.command(autocmd)?;
        Ok(())
    }

    /// Save window state to persistence
    fn save_window_state(&self) -> PluginResult<()> {
        let mut plugin_state = self.persistence.load_state()
            .unwrap_or_default();

        // Convert window configurations to persistable format
        plugin_state.window_positions.clear();
        
        if let Some(chat_window) = &self.state.chat_window {
            plugin_state.window_positions.push(crate::config::persistence::WindowPosition {
                window_type: "chat".to_string(),
                x: chat_window.position.col,
                y: chat_window.position.row,
                width: chat_window.size.width,
                height: chat_window.size.height,
                z_index: chat_window.z_index,
            });
        }

        if let Some(input_window) = &self.state.input_window {
            plugin_state.window_positions.push(crate::config::persistence::WindowPosition {
                window_type: "input".to_string(),
                x: input_window.position.col,
                y: input_window.position.row,
                width: input_window.size.width,
                height: input_window.size.height,
                z_index: input_window.z_index,
            });
        }

        self.persistence.save_state(&plugin_state)
            .map_err(|e| PluginError::window(&format!("Failed to save window state: {}", e)))?;

        Ok(())
    }

    /// Load window state from persistence
    fn load_window_state(&mut self) -> PluginResult<()> {
        let plugin_state = self.persistence.load_state()
            .unwrap_or_default();

        // Apply saved window positions if they exist and are reasonable
        for window_pos in &plugin_state.window_positions {
            match window_pos.window_type.as_str() {
                "chat" => {
                    // Validate dimensions are reasonable for current terminal
                    if self.is_position_valid(&window_pos) {
                        // Store for later use when creating windows
                        // This will be used in create_chat_window if available
                    }
                }
                "input" => {
                    if self.is_position_valid(&window_pos) {
                        // Store for later use when creating windows
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Check if a window position is valid for current terminal size
    fn is_position_valid(&self, pos: &crate::config::persistence::WindowPosition) -> bool {
        pos.x + pos.width <= self.state.dimensions.terminal_width &&
        pos.y + pos.height <= self.state.dimensions.terminal_height &&
        pos.width > 0 && pos.height > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::layout::{WindowDimensions, LayoutMode, WindowLayoutConfig, Position, Size};

    #[test]
    fn test_window_manager_creation() {
        let window_manager = WindowManager::new();
        assert!(window_manager.is_ok());
        
        let wm = window_manager.unwrap();
        assert_eq!(wm.state.layout_mode, LayoutMode::Normal);
        assert!(wm.state.chat_window.is_none());
        assert!(wm.state.input_window.is_none());
        assert!(wm.state.command_approval_window.is_none());
    }

    #[test]
    fn test_window_dimensions_calculations() {
        let dimensions = WindowDimensions::new(100, 50);
        
        // Test chat window config
        let chat_config = dimensions.calculate_chat_window_config();
        assert_eq!(chat_config.size.width, 80); // 80% of 100
        assert_eq!(chat_config.size.height, 30); // 60% of 50
        assert_eq!(chat_config.position.col, 10); // Centered
        
        // Test input window config
        let input_config = dimensions.calculate_input_window_config();
        assert_eq!(input_config.size.width, 80); // 80% of 100
        assert_eq!(input_config.size.height, 3); // Fixed height
        assert_eq!(input_config.position.col, 10); // Centered
        
        // Test command approval window config
        let command_config = dimensions.calculate_command_approval_window_config();
        assert_eq!(command_config.size.width, 80); // 80% of 100
        assert_eq!(command_config.size.height, 8); // Fixed height
        assert_eq!(command_config.position.col, 10); // Centered
    }

    #[test]
    fn test_responsive_layout_calculations() {
        let dimensions = WindowDimensions::new(120, 40);
        
        // Test normal layout
        let normal_chat = dimensions.calculate_responsive_config("chat", &LayoutMode::Normal);
        let command_chat = dimensions.calculate_responsive_config("chat", &LayoutMode::CommandApproval);
        
        // Command approval layout should have smaller chat window
        assert!(command_chat.size.height < normal_chat.size.height);
    }

    #[test]
    fn test_window_bounds_validation() {
        let dimensions = WindowDimensions::new(80, 24);
        
        // Valid configuration
        let valid_config = WindowLayoutConfig {
            position: Position { col: 10, row: 5 },
            size: Size { width: 60, height: 15 },
            z_index: 100,
        };
        assert!(dimensions.validate_window_bounds(&valid_config));
        
        // Invalid configuration (too wide)
        let invalid_config = WindowLayoutConfig {
            position: Position { col: 10, row: 5 },
            size: Size { width: 80, height: 15 }, // 10 + 80 > 80
            z_index: 100,
        };
        assert!(!dimensions.validate_window_bounds(&invalid_config));
    }

    #[test]
    fn test_window_bounds_adjustment() {
        let dimensions = WindowDimensions::new(80, 24);
        
        // Configuration that needs adjustment
        let oversized_config = WindowLayoutConfig {
            position: Position { col: 70, row: 20 },
            size: Size { width: 50, height: 10 }, // Would exceed bounds
            z_index: 100,
        };
        
        let adjusted = dimensions.adjust_to_bounds(oversized_config);
        
        // Should fit within bounds
        assert!(dimensions.validate_window_bounds(&adjusted));
        
        // Should maintain minimum sizes
        assert!(adjusted.size.width >= 20);
        assert!(adjusted.size.height >= 3);
    }

    #[test]
    fn test_interface_state_tracking() {
        let mut wm = WindowManager::new().unwrap();
        
        // Initially not open
        assert!(!wm.is_interface_open());
        
        // Simulate windows being created
        wm.state.chat_window = Some(WindowConfig {
            buffer_id: 1,
            window_id: 1,
            position: Position { col: 10, row: 5 },
            size: Size { width: 60, height: 20 },
            z_index: 100,
        });
        
        // Still not open (need both windows)
        assert!(!wm.is_interface_open());
        
        wm.state.input_window = Some(WindowConfig {
            buffer_id: 2,
            window_id: 2,
            position: Position { col: 10, row: 25 },
            size: Size { width: 60, height: 3 },
            z_index: 101,
        });
        
        // Now interface is open
        assert!(wm.is_interface_open());
    }

    #[test]
    fn test_z_index_management() {
        let mut wm = WindowManager::new().unwrap();
        
        let initial_z = wm.z_index_counter;
        let z1 = wm.get_next_z_index();
        let z2 = wm.get_next_z_index();
        
        assert_eq!(z1, initial_z + 1);
        assert_eq!(z2, initial_z + 2);
        assert!(z2 > z1); // Ensure proper ordering
    }
}