use neovim_lib::{Neovim, Session};
use crate::utils::error_handling::PluginResult;

pub mod ui;
pub mod agent;
pub mod spec;
pub mod config;
pub mod utils;
pub mod examples;
pub mod communication;
pub mod container;

use crate::config::Settings;
use crate::ui::WindowManager;
use crate::agent::ChatManager;
use crate::spec::workflow::SpecWorkflow;
use std::path::PathBuf;

/// Main plugin struct that coordinates all components
pub struct NvimSpecAgent {
    pub neovim: Neovim,
    pub settings: Settings,
    pub window_manager: WindowManager,
    pub chat_manager: ChatManager,
    pub spec_workflow: SpecWorkflow,
}

impl NvimSpecAgent {
    /// Initialize the plugin with Neovim session
    pub fn new(session: Session) -> PluginResult<Self> {
        let neovim = Neovim::new(session);
        let settings = Settings::load_or_default()?;
        
        // Default spec root to .kiro/specs in current directory
        let spec_root = PathBuf::from(".kiro/specs");
        
        Ok(NvimSpecAgent {
            neovim,
            settings: settings.clone(),
            window_manager: WindowManager::new()?,
            chat_manager: ChatManager::new(None)?,
            spec_workflow: SpecWorkflow::new(spec_root),
        })
    }

    /// Main plugin entry point called by Neovim
    pub fn start(&mut self) -> PluginResult<()> {
        log::info!("Starting nvim-spec-agent plugin");
        
        // Register plugin commands
        self.register_commands()?;
        
        // Set up keybindings
        self.setup_keybindings()?;
        
        // Initialize UI components
        self.window_manager.initialize(&mut self.neovim)?;
        
        log::info!("nvim-spec-agent plugin started successfully");
        Ok(())
    }

    /// Register plugin commands with Neovim
    fn register_commands(&mut self) -> PluginResult<()> {
        use crate::utils::neovim_api::{NeovimApiWrapper, CommandOptions};
        
        let mut api = NeovimApiWrapper::new(&mut self.neovim);
        let mut registration = api.create_plugin_registration("nvim-spec-agent");

        // Register main agent interface command
        registration.add_command(
            "SpecAgent",
            "require('nvim-spec-agent').open_agent()",
            CommandOptions {
                desc: Some("Open the spec agent interface".to_string()),
                ..Default::default()
            }
        );

        // Register spec-related commands
        registration.add_command(
            "SpecNew",
            "require('nvim-spec-agent').new_spec()",
            CommandOptions {
                desc: Some("Create a new spec".to_string()),
                nargs: Some("?".to_string()),
                ..Default::default()
            }
        );

        registration.add_command(
            "SpecOpen",
            "require('nvim-spec-agent').open_spec()",
            CommandOptions {
                desc: Some("Open an existing spec".to_string()),
                nargs: Some("?".to_string()),
                complete: Some("file".to_string()),
                ..Default::default()
            }
        );

        // Register the plugin
        registration.register(&mut api)?;
        
        Ok(())
    }

    /// Set up default keybindings
    fn setup_keybindings(&mut self) -> PluginResult<()> {
        use crate::utils::neovim_api::{NeovimApiWrapper, KeymapOptions};
        
        let mut api = NeovimApiWrapper::new(&mut self.neovim);

        // Default keybinding to open agent interface
        api.set_keymap(
            "n",
            "<leader>sa",
            ":SpecAgent<CR>",
            &KeymapOptions {
                desc: Some("Open Spec Agent".to_string()),
                ..Default::default()
            }
        )?;

        // Spec navigation keybindings
        api.set_keymap(
            "n",
            "<leader>sn",
            ":SpecNew<CR>",
            &KeymapOptions {
                desc: Some("New Spec".to_string()),
                ..Default::default()
            }
        )?;

        api.set_keymap(
            "n",
            "<leader>so",
            ":SpecOpen<CR>",
            &KeymapOptions {
                desc: Some("Open Spec".to_string()),
                ..Default::default()
            }
        )?;
        
        Ok(())
    }

    /// Open the agent interface
    pub fn open_agent(&mut self) -> PluginResult<()> {
        log::info!("Opening agent interface");
        self.window_manager.create_agent_interface(&mut self.neovim)?;
        Ok(())
    }

    /// Create a new spec
    pub fn new_spec(&mut self, feature_name: Option<String>) -> PluginResult<()> {
        let name = feature_name.unwrap_or_else(|| "new-feature".to_string());
        log::info!("Creating new spec: {}", name);
        self.spec_workflow.create_new_spec(name)?;
        Ok(())
    }

    /// Open existing spec
    pub fn open_spec(&mut self, spec_name: Option<String>) -> PluginResult<()> {
        let name = spec_name.unwrap_or_else(|| "existing-spec".to_string());
        log::info!("Opening spec: {}", name);
        self.spec_workflow.open_spec(name)?;
        Ok(())
    }
}

/// Plugin entry point for Neovim remote plugin
#[no_mangle]
pub extern "C" fn nvim_spec_agent_main() {
    env_logger::init();
    
    let session = Session::new_child().expect("Failed to create Neovim session");
    let mut plugin = NvimSpecAgent::new(session).expect("Failed to initialize plugin");
    
    if let Err(e) = plugin.start() {
        log::error!("Failed to start plugin: {}", e);
    }
}