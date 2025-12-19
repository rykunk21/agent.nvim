use serde::{Deserialize, Serialize};

/// Layout modes for different UI states
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LayoutMode {
    Normal,
    CommandApproval,
    SpecNavigation,
}

/// Window dimensions and positioning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowDimensions {
    pub terminal_width: u32,
    pub terminal_height: u32,
}

impl WindowDimensions {
    pub fn new(width: u32, height: u32) -> Self {
        WindowDimensions {
            terminal_width: width,
            terminal_height: height,
        }
    }

    /// Calculate chat window configuration
    pub fn calculate_chat_window_config(&self) -> WindowLayoutConfig {
        let width = (self.terminal_width as f32 * 0.8) as u32;
        let height = (self.terminal_height as f32 * 0.6) as u32;
        let col = (self.terminal_width - width) / 2;
        let row = (self.terminal_height - height) / 4;

        WindowLayoutConfig {
            position: Position { col, row },
            size: Size { width, height },
            z_index: 100,
        }
    }

    /// Calculate input window configuration
    pub fn calculate_input_window_config(&self) -> WindowLayoutConfig {
        let width = (self.terminal_width as f32 * 0.8) as u32;
        let height = 3; // Fixed height for input
        let col = (self.terminal_width - width) / 2;
        let row = self.terminal_height - height - 2;

        WindowLayoutConfig {
            position: Position { col, row },
            size: Size { width, height },
            z_index: 101,
        }
    }

    /// Calculate chat window configuration when command approval is active
    pub fn calculate_chat_window_config_with_command(&self) -> WindowLayoutConfig {
        let width = (self.terminal_width as f32 * 0.8) as u32;
        let height = (self.terminal_height as f32 * 0.4) as u32; // Smaller to make room for command
        let col = (self.terminal_width - width) / 2;
        let row = (self.terminal_height - height) / 6;

        WindowLayoutConfig {
            position: Position { col, row },
            size: Size { width, height },
            z_index: 100,
        }
    }

    /// Calculate command approval window configuration
    pub fn calculate_command_approval_window_config(&self) -> WindowLayoutConfig {
        let width = (self.terminal_width as f32 * 0.8) as u32;
        let height = 8; // Fixed height for command approval
        let col = (self.terminal_width - width) / 2;
        let row = self.terminal_height / 2;

        WindowLayoutConfig {
            position: Position { col, row },
            size: Size { width, height },
            z_index: 102,
        }
    }

    /// Calculate responsive dimensions based on terminal size
    pub fn calculate_responsive_config(&self, window_type: &str, layout_mode: &LayoutMode) -> WindowLayoutConfig {
        match (window_type, layout_mode) {
            ("chat", LayoutMode::Normal) => self.calculate_chat_window_config(),
            ("chat", LayoutMode::CommandApproval) => self.calculate_chat_window_config_with_command(),
            ("input", _) => self.calculate_input_window_config(),
            ("command", _) => self.calculate_command_approval_window_config(),
            _ => self.calculate_chat_window_config(), // Default fallback
        }
    }

    /// Validate window fits within terminal bounds
    pub fn validate_window_bounds(&self, config: &WindowLayoutConfig) -> bool {
        config.position.col + config.size.width <= self.terminal_width &&
        config.position.row + config.size.height <= self.terminal_height
    }

    /// Adjust window configuration to fit within terminal bounds
    pub fn adjust_to_bounds(&self, mut config: WindowLayoutConfig) -> WindowLayoutConfig {
        // Ensure minimum sizes first
        config.size.width = config.size.width.max(20);
        config.size.height = config.size.height.max(3);

        // Adjust width if too wide
        if config.position.col + config.size.width > self.terminal_width {
            if config.size.width <= self.terminal_width {
                // Window fits, just need to move it
                config.position.col = self.terminal_width - config.size.width;
            } else {
                // Window is too wide, resize and position at 0
                config.position.col = 0;
                config.size.width = self.terminal_width;
            }
        }

        // Adjust height if too tall
        if config.position.row + config.size.height > self.terminal_height {
            if config.size.height <= self.terminal_height {
                // Window fits, just need to move it
                config.position.row = self.terminal_height - config.size.height;
            } else {
                // Window is too tall, resize and position at 0
                config.position.row = 0;
                config.size.height = self.terminal_height;
            }
        }

        config
    }
}

impl Default for WindowDimensions {
    fn default() -> Self {
        WindowDimensions::new(80, 24)
    }
}

/// Window configuration for layout calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowLayoutConfig {
    pub position: Position,
    pub size: Size,
    pub z_index: i32,
}

/// Window configuration with Neovim IDs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    pub buffer_id: i32,
    pub window_id: i32,
    pub position: Position,
    pub size: Size,
    pub z_index: i32,
}

/// Window position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub col: u32,
    pub row: u32,
}

/// Window size
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

/// Complete window state management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    pub chat_window: Option<WindowConfig>,
    pub input_window: Option<WindowConfig>,
    pub command_approval_window: Option<WindowConfig>,
    pub layout_mode: LayoutMode,
    pub dimensions: WindowDimensions,
}