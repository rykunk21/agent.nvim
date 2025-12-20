use neovim_lib::{Neovim, NeovimApi, Value};
use crate::utils::error_handling::{PluginResult, PluginError};
use serde_json;
use std::collections::HashMap;
use tokio::sync::mpsc;

/// Async communication handler for msgpack-rpc
pub struct AsyncNeovimHandler {
    sender: mpsc::UnboundedSender<NeovimCommand>,
    receiver: mpsc::UnboundedReceiver<NeovimResponse>,
}

/// Commands that can be sent to Neovim asynchronously
#[derive(Debug, Clone)]
pub enum NeovimCommand {
    CreateBuffer { name: Option<String> },
    CreateWindow { buffer_id: i32, config: WindowConfig },
    SetBufferLines { buffer_id: i32, lines: Vec<String> },
    ExecuteLua { code: String },
    ShowNotification { message: String, level: NotificationLevel },
    RegisterCommand { name: String, lua_callback: String },
    SetKeymap { mode: String, lhs: String, rhs: String, opts: KeymapOptions },
}

/// Responses from Neovim operations
#[derive(Debug, Clone)]
pub enum NeovimResponse {
    BufferId(i32),
    WindowId(i32),
    LuaResult(Value),
    Success,
    Error(String),
}

/// Window configuration for creating floating windows
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
    pub col: u32,
    pub row: u32,
    pub relative: String,
    pub style: String,
    pub border: String,
    pub focusable: bool,
    pub zindex: Option<u32>,
}

/// Keymap options for setting keybindings
#[derive(Debug, Clone)]
pub struct KeymapOptions {
    pub noremap: bool,
    pub silent: bool,
    pub expr: bool,
    pub desc: Option<String>,
}

/// Wrapper for Neovim API with convenience methods
pub struct NeovimApiWrapper<'a> {
    pub neovim: &'a mut Neovim,
    pub async_handler: Option<AsyncNeovimHandler>,
}

impl<'a> NeovimApiWrapper<'a> {
    pub fn new(neovim: &'a mut Neovim) -> Self {
        NeovimApiWrapper { 
            neovim,
            async_handler: None,
        }
    }

    /// Initialize async communication handler
    pub fn init_async_handler(&mut self) -> PluginResult<()> {
        let (cmd_sender, cmd_receiver) = mpsc::unbounded_channel();
        let (resp_sender, resp_receiver) = mpsc::unbounded_channel();
        
        self.async_handler = Some(AsyncNeovimHandler {
            sender: cmd_sender,
            receiver: resp_receiver,
        });

        // Start async message processing task
        tokio::spawn(async move {
            Self::process_async_commands(cmd_receiver, resp_sender).await;
        });

        Ok(())
    }

    /// Process async commands in background task
    async fn process_async_commands(
        mut receiver: mpsc::UnboundedReceiver<NeovimCommand>,
        sender: mpsc::UnboundedSender<NeovimResponse>,
    ) {
        while let Some(command) = receiver.recv().await {
            let response = match command {
                NeovimCommand::CreateBuffer { name: _ } => {
                    // This would need actual Neovim API call in real implementation
                    NeovimResponse::BufferId(1) // Placeholder
                },
                NeovimCommand::ShowNotification { message: _, level: _ } => {
                    // Process notification
                    NeovimResponse::Success
                },
                _ => NeovimResponse::Success,
            };

            if sender.send(response).is_err() {
                break;
            }
        }
    }

    /// Send async command to Neovim
    pub async fn send_async_command(&mut self, command: NeovimCommand) -> PluginResult<NeovimResponse> {
        if let Some(handler) = &mut self.async_handler {
            handler.sender.send(command)
                .map_err(|e| PluginError::agent(&format!("Failed to send async command: {}", e)))?;
            
            handler.receiver.recv().await
                .ok_or_else(|| PluginError::agent("Failed to receive async response"))
        } else {
            Err(PluginError::agent("Async handler not initialized"))
        }
    }

    /// Create a floating window with comprehensive configuration
    pub fn create_floating_window(
        &mut self,
        buffer_id: i32,
        config: &WindowConfig,
    ) -> PluginResult<i32> {
        let zindex_str = config.zindex
            .map(|z| format!(", zindex = {}", z))
            .unwrap_or_default();

        let lua_code = format!(
            r#"
            local opts = {{
                relative = '{}',
                width = {},
                height = {},
                col = {},
                row = {},
                style = '{}',
                border = '{}',
                focusable = {}{}
            }}
            return vim.api.nvim_open_win({}, {}, opts)
            "#,
            config.relative, config.width, config.height, 
            config.col, config.row, config.style, config.border,
            config.focusable, zindex_str, buffer_id, config.focusable
        );

        let window_result = self.neovim.execute_lua(&lua_code, vec![])?;
        let window_id = self.value_to_i32(window_result)?;

        Ok(window_id)
    }

    /// Create a new buffer
    pub fn create_buffer(&mut self, listed: bool, scratch: bool) -> PluginResult<i32> {
        let lua_code = format!(
            "return vim.api.nvim_create_buf({}, {})",
            listed, scratch
        );
        
        let buffer_result = self.neovim.execute_lua(&lua_code, vec![])?;
        let buffer_id = self.value_to_i32(buffer_result)?;

        Ok(buffer_id)
    }

    /// Set buffer option
    pub fn set_buffer_option(&mut self, buffer_id: i32, name: &str, value: &str) -> PluginResult<()> {
        let lua_code = format!(
            "vim.api.nvim_buf_set_option({}, '{}', '{}')",
            buffer_id, name, value
        );
        self.neovim.execute_lua(&lua_code, vec![])?;
        Ok(())
    }

    /// Set window option
    pub fn set_window_option(&mut self, window_id: i32, name: &str, value: &str) -> PluginResult<()> {
        let lua_code = format!(
            "vim.api.nvim_win_set_option({}, '{}', '{}')",
            window_id, name, value
        );
        self.neovim.execute_lua(&lua_code, vec![])?;
        Ok(())
    }

    /// Close window
    pub fn close_window(&mut self, window_id: i32, force: bool) -> PluginResult<()> {
        let lua_code = format!(
            "vim.api.nvim_win_close({}, {})",
            window_id, force
        );
        self.neovim.execute_lua(&lua_code, vec![])?;
        Ok(())
    }

    /// Delete buffer
    pub fn delete_buffer(&mut self, buffer_id: i32, force: bool) -> PluginResult<()> {
        let lua_code = format!(
            "vim.api.nvim_buf_delete({}, {{ force = {} }})",
            buffer_id, force
        );
        self.neovim.execute_lua(&lua_code, vec![])?;
        Ok(())
    }

    /// Set buffer content with lines
    pub fn set_buffer_lines(&mut self, buffer_id: i32, lines: Vec<String>) -> PluginResult<()> {
        let lines_json = serde_json::to_string(&lines)?;
        let lua_code = format!(
            "vim.api.nvim_buf_set_lines({}, 0, -1, false, {})",
            buffer_id, lines_json
        );
        self.neovim.execute_lua(&lua_code, vec![])?;
        Ok(())
    }

    /// Get terminal dimensions
    pub fn get_terminal_dimensions(&mut self) -> PluginResult<(u32, u32)> {
        let columns = self.neovim.get_option("columns")?.as_i64().unwrap_or(80) as u32;
        let lines = self.neovim.get_option("lines")?.as_i64().unwrap_or(24) as u32;
        Ok((columns, lines))
    }

    /// Show notification message
    pub fn show_notification(&mut self, message: &str, level: NotificationLevel) -> PluginResult<()> {
        let level_str = match level {
            NotificationLevel::Info => "info",
            NotificationLevel::Warning => "warn",
            NotificationLevel::Error => "error",
        };

        self.neovim.command(&format!(
            "lua vim.notify('{}', vim.log.levels.{})",
            message.replace("'", "\\'"),
            level_str.to_uppercase()
        ))?;

        Ok(())
    }

    /// Execute Lua code
    pub fn execute_lua(&mut self, code: &str) -> PluginResult<Value> {
        Ok(self.neovim.execute_lua(code, vec![])?)
    }

    /// Check if buffer exists
    pub fn buffer_exists(&mut self, buffer_id: i32) -> PluginResult<bool> {
        let lua_code = format!("return vim.api.nvim_buf_is_valid({})", buffer_id);
        let result = self.neovim.execute_lua(&lua_code, vec![])?;
        Ok(result.as_bool().unwrap_or(false))
    }

    /// Check if window exists
    pub fn window_exists(&mut self, window_id: i32) -> PluginResult<bool> {
        let lua_code = format!("return vim.api.nvim_win_is_valid({})", window_id);
        let result = self.neovim.execute_lua(&lua_code, vec![])?;
        Ok(result.as_bool().unwrap_or(false))
    }

    /// Register a user command
    pub fn register_command(&mut self, name: &str, callback: &str, opts: &CommandOptions) -> PluginResult<()> {
        let nargs = match opts.nargs {
            Some(ref n) => format!(", nargs = '{}'", n),
            None => String::new(),
        };

        let complete = match opts.complete {
            Some(ref c) => format!(", complete = '{}'", c),
            None => String::new(),
        };

        let desc = match opts.desc {
            Some(ref d) => format!(", desc = '{}'", d.replace("'", "\\'")),
            None => String::new(),
        };

        let lua_code = format!(
            "vim.api.nvim_create_user_command('{}', {}, {{ force = {}{}{}{} }})",
            name, callback, opts.force, nargs, complete, desc
        );

        self.neovim.execute_lua(&lua_code, vec![])?;
        Ok(())
    }

    /// Set keymap
    pub fn set_keymap(&mut self, mode: &str, lhs: &str, rhs: &str, opts: &KeymapOptions) -> PluginResult<()> {
        let desc = match opts.desc {
            Some(ref d) => format!(", desc = '{}'", d.replace("'", "\\'")),
            None => String::new(),
        };

        let lua_code = format!(
            r#"vim.keymap.set('{}', '{}', '{}', {{ 
                noremap = {}, 
                silent = {}, 
                expr = {}{} 
            }})"#,
            mode, lhs, rhs, opts.noremap, opts.silent, opts.expr, desc
        );

        self.neovim.execute_lua(&lua_code, vec![])?;
        Ok(())
    }

    /// Get current working directory
    pub fn get_cwd(&mut self) -> PluginResult<String> {
        let result = self.neovim.execute_lua("return vim.fn.getcwd()", vec![])?;
        self.value_to_string(result)
    }

    /// Get buffer name
    pub fn get_buffer_name(&mut self, buffer_id: i32) -> PluginResult<String> {
        let lua_code = format!("return vim.api.nvim_buf_get_name({})", buffer_id);
        let result = self.neovim.execute_lua(&lua_code, vec![])?;
        self.value_to_string(result)
    }

    /// Get current buffer
    pub fn get_current_buffer(&mut self) -> PluginResult<i32> {
        let result = self.neovim.execute_lua("return vim.api.nvim_get_current_buf()", vec![])?;
        self.value_to_i32(result)
    }

    /// Get current window
    pub fn get_current_window(&mut self) -> PluginResult<i32> {
        let result = self.neovim.execute_lua("return vim.api.nvim_get_current_win()", vec![])?;
        self.value_to_i32(result)
    }

    /// Set current window
    pub fn set_current_window(&mut self, window_id: i32) -> PluginResult<()> {
        let lua_code = format!("vim.api.nvim_set_current_win({})", window_id);
        self.neovim.execute_lua(&lua_code, vec![])?;
        Ok(())
    }

    /// Get window configuration
    pub fn get_window_config(&mut self, window_id: i32) -> PluginResult<HashMap<String, Value>> {
        let lua_code = format!("return vim.api.nvim_win_get_config({})", window_id);
        let result = self.neovim.execute_lua(&lua_code, vec![])?;
        self.value_to_map(result)
    }

    /// Set window configuration
    pub fn set_window_config(&mut self, window_id: i32, config: &WindowConfig) -> PluginResult<()> {
        let zindex_str = config.zindex
            .map(|z| format!(", zindex = {}", z))
            .unwrap_or_default();

        let lua_code = format!(
            r#"
            vim.api.nvim_win_set_config({}, {{
                relative = '{}',
                width = {},
                height = {},
                col = {},
                row = {},
                focusable = {}{}
            }})
            "#,
            window_id, config.relative, config.width, config.height,
            config.col, config.row, config.focusable, zindex_str
        );

        self.neovim.execute_lua(&lua_code, vec![])?;
        Ok(())
    }

    /// Type conversion helpers
    fn value_to_i32(&self, value: Value) -> PluginResult<i32> {
        value.as_i64()
            .map(|i| i as i32)
            .ok_or_else(|| PluginError::agent("Failed to convert value to i32"))
    }

    fn value_to_string(&self, value: Value) -> PluginResult<String> {
        value.as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| PluginError::agent("Failed to convert value to string"))
    }

    fn value_to_map(&self, value: Value) -> PluginResult<HashMap<String, Value>> {
        if let Value::Map(map) = value {
            let mut result = HashMap::new();
            for (k, v) in map {
                if let Value::String(key) = k {
                    result.insert(key.into_str().unwrap_or_default(), v);
                }
            }
            Ok(result)
        } else {
            Err(PluginError::agent("Failed to convert value to map"))
        }
    }
}

/// Notification levels for user messages
#[derive(Debug, Clone)]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
}

/// Command options for registering user commands
#[derive(Debug, Clone)]
pub struct CommandOptions {
    pub force: bool,
    pub nargs: Option<String>,
    pub complete: Option<String>,
    pub desc: Option<String>,
}

impl Default for CommandOptions {
    fn default() -> Self {
        CommandOptions {
            force: true,
            nargs: None,
            complete: None,
            desc: None,
        }
    }
}

impl Default for KeymapOptions {
    fn default() -> Self {
        KeymapOptions {
            noremap: true,
            silent: true,
            expr: false,
            desc: None,
        }
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        WindowConfig {
            width: 80,
            height: 20,
            col: 10,
            row: 5,
            relative: "editor".to_string(),
            style: "minimal".to_string(),
            border: "rounded".to_string(),
            focusable: true,
            zindex: None,
        }
    }
}

/// Plugin registration helper
pub struct PluginRegistration {
    pub name: String,
    pub commands: Vec<(String, String, CommandOptions)>,
    pub keymaps: Vec<(String, String, String, KeymapOptions)>,
    pub autocmds: Vec<AutoCommand>,
}

/// Auto command configuration
#[derive(Debug, Clone)]
pub struct AutoCommand {
    pub event: String,
    pub pattern: Option<String>,
    pub callback: String,
    pub group: Option<String>,
}

impl PluginRegistration {
    pub fn new(name: &str) -> Self {
        PluginRegistration {
            name: name.to_string(),
            commands: Vec::new(),
            keymaps: Vec::new(),
            autocmds: Vec::new(),
        }
    }

    pub fn add_command(&mut self, name: &str, callback: &str, opts: CommandOptions) {
        self.commands.push((name.to_string(), callback.to_string(), opts));
    }

    pub fn add_keymap(&mut self, mode: &str, lhs: &str, rhs: &str, opts: KeymapOptions) {
        self.keymaps.push((mode.to_string(), lhs.to_string(), rhs.to_string(), opts));
    }

    pub fn add_autocmd(&mut self, autocmd: AutoCommand) {
        self.autocmds.push(autocmd);
    }

    /// Register all plugin components with Neovim
    pub fn register(&self, api: &mut NeovimApiWrapper) -> PluginResult<()> {
        // Register commands
        for (name, callback, opts) in &self.commands {
            api.register_command(name, callback, opts)?;
        }

        // Register keymaps
        for (mode, lhs, rhs, opts) in &self.keymaps {
            api.set_keymap(mode, lhs, rhs, opts)?;
        }

        // Register autocmds
        for autocmd in &self.autocmds {
            api.register_autocmd(autocmd)?;
        }

        Ok(())
    }
}

impl<'a> NeovimApiWrapper<'a> {
    /// Register an autocommand
    pub fn register_autocmd(&mut self, autocmd: &AutoCommand) -> PluginResult<()> {
        let pattern = autocmd.pattern
            .as_ref()
            .map(|p| format!(", pattern = '{}'", p))
            .unwrap_or_default();

        let group = autocmd.group
            .as_ref()
            .map(|g| format!(", group = '{}'", g))
            .unwrap_or_default();

        let lua_code = format!(
            "vim.api.nvim_create_autocmd('{}', {{ callback = {}{}{} }})",
            autocmd.event, autocmd.callback, pattern, group
        );

        self.neovim.execute_lua(&lua_code, vec![])?;
        Ok(())
    }

    /// Create a plugin registration helper
    pub fn create_plugin_registration(&self, name: &str) -> PluginRegistration {
        PluginRegistration::new(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_config_default() {
        let config = WindowConfig::default();
        assert_eq!(config.width, 80);
        assert_eq!(config.height, 20);
        assert_eq!(config.relative, "editor");
        assert_eq!(config.style, "minimal");
        assert_eq!(config.border, "rounded");
        assert!(config.focusable);
        assert!(config.zindex.is_none());
    }

    #[test]
    fn test_keymap_options_default() {
        let opts = KeymapOptions::default();
        assert!(opts.noremap);
        assert!(opts.silent);
        assert!(!opts.expr);
        assert!(opts.desc.is_none());
    }

    #[test]
    fn test_command_options_default() {
        let opts = CommandOptions::default();
        assert!(opts.force);
        assert!(opts.nargs.is_none());
        assert!(opts.complete.is_none());
        assert!(opts.desc.is_none());
    }

    #[test]
    fn test_plugin_registration_creation() {
        let registration = PluginRegistration::new("test-plugin");
        assert_eq!(registration.name, "test-plugin");
        assert!(registration.commands.is_empty());
        assert!(registration.keymaps.is_empty());
        assert!(registration.autocmds.is_empty());
    }

    #[test]
    fn test_plugin_registration_add_command() {
        let mut registration = PluginRegistration::new("test-plugin");
        let opts = CommandOptions {
            desc: Some("Test command".to_string()),
            ..Default::default()
        };
        
        registration.add_command("TestCmd", "echo 'test'", opts);
        assert_eq!(registration.commands.len(), 1);
        assert_eq!(registration.commands[0].0, "TestCmd");
        assert_eq!(registration.commands[0].1, "echo 'test'");
    }

    #[test]
    fn test_plugin_registration_add_keymap() {
        let mut registration = PluginRegistration::new("test-plugin");
        let opts = KeymapOptions {
            desc: Some("Test keymap".to_string()),
            ..Default::default()
        };
        
        registration.add_keymap("n", "<leader>t", ":TestCmd<CR>", opts);
        assert_eq!(registration.keymaps.len(), 1);
        assert_eq!(registration.keymaps[0].0, "n");
        assert_eq!(registration.keymaps[0].1, "<leader>t");
        assert_eq!(registration.keymaps[0].2, ":TestCmd<CR>");
    }

    #[test]
    fn test_autocmd_creation() {
        let autocmd = AutoCommand {
            event: "BufEnter".to_string(),
            pattern: Some("*.rs".to_string()),
            callback: "print('Rust file opened')".to_string(),
            group: Some("RustGroup".to_string()),
        };
        
        assert_eq!(autocmd.event, "BufEnter");
        assert_eq!(autocmd.pattern, Some("*.rs".to_string()));
        assert_eq!(autocmd.callback, "print('Rust file opened')");
        assert_eq!(autocmd.group, Some("RustGroup".to_string()));
    }

    #[test]
    fn test_neovim_command_variants() {
        let cmd1 = NeovimCommand::CreateBuffer { name: Some("test.txt".to_string()) };
        let cmd2 = NeovimCommand::ShowNotification { 
            message: "Hello".to_string(), 
            level: NotificationLevel::Info 
        };
        
        match cmd1 {
            NeovimCommand::CreateBuffer { name } => {
                assert_eq!(name, Some("test.txt".to_string()));
            },
            _ => panic!("Wrong command variant"),
        }
        
        match cmd2 {
            NeovimCommand::ShowNotification { message, level } => {
                assert_eq!(message, "Hello");
                matches!(level, NotificationLevel::Info);
            },
            _ => panic!("Wrong command variant"),
        }
    }

    #[test]
    fn test_neovim_response_variants() {
        let resp1 = NeovimResponse::BufferId(42);
        let resp2 = NeovimResponse::WindowId(24);
        let resp3 = NeovimResponse::Success;
        let resp4 = NeovimResponse::Error("Test error".to_string());
        
        match resp1 {
            NeovimResponse::BufferId(id) => assert_eq!(id, 42),
            _ => panic!("Wrong response variant"),
        }
        
        match resp2 {
            NeovimResponse::WindowId(id) => assert_eq!(id, 24),
            _ => panic!("Wrong response variant"),
        }
        
        matches!(resp3, NeovimResponse::Success);
        
        match resp4 {
            NeovimResponse::Error(msg) => assert_eq!(msg, "Test error"),
            _ => panic!("Wrong response variant"),
        }
    }
}