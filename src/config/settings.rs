use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use crate::utils::error_handling::{PluginResult, PluginError};
use chrono::{DateTime, Utc};

/// Plugin configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub ui: UiSettings,
    pub agent: AgentSettings,
    pub spec: SpecSettings,
    pub persistence: PersistenceSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSettings {
    pub window_width_ratio: f32,
    pub window_height_ratio: f32,
    pub border_style: String,
    pub theme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSettings {
    pub auto_approve_safe_commands: bool,
    pub command_timeout_seconds: u64,
    pub max_conversation_history: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecSettings {
    pub default_spec_directory: String,
    pub auto_save_interval_seconds: u64,
    pub property_test_iterations: u32,
    pub enable_property_testing: bool,
    pub spec_template_directory: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceSettings {
    pub max_conversations: usize,
    pub conversation_retention_days: u32,
    pub auto_archive_enabled: bool,
    pub backup_enabled: bool,
    pub backup_interval_hours: u32,
}

impl Settings {
    /// Current configuration version for migration handling
    pub const CURRENT_VERSION: &'static str = "1.0.0";

    /// Load settings from file or create default with migration support
    pub fn load_or_default() -> PluginResult<Self> {
        let config_path = Self::get_config_path()?;
        
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .map_err(|e| PluginError::config(&format!("Failed to read config file: {}", e)))?;
            
            // Try to parse as current version first
            match serde_json::from_str::<Settings>(&content) {
                Ok(mut settings) => {
                    // Check if migration is needed
                    if settings.version != Self::CURRENT_VERSION {
                        settings = Self::migrate_settings(settings)?;
                        settings.save()?;
                    }
                    Ok(settings)
                }
                Err(_) => {
                    // Try to parse as legacy format and migrate
                    Self::migrate_from_legacy(&content)
                }
            }
        } else {
            let default_settings = Self::default();
            default_settings.save()?;
            Ok(default_settings)
        }
    }

    /// Save settings to file with backup
    pub fn save(&self) -> PluginResult<()> {
        let config_path = Self::get_config_path()?;
        
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| PluginError::config(&format!("Failed to create config directory: {}", e)))?;
        }
        
        // Create backup if file exists (skip in tests to avoid file locking issues)
        if config_path.exists() && !cfg!(test) {
            let backup_path = config_path.with_extension("json.backup");
            let _ = fs::copy(&config_path, &backup_path); // Ignore backup errors in production
        }
        
        let mut updated_settings = self.clone();
        updated_settings.updated_at = Utc::now();
        
        let content = serde_json::to_string_pretty(&updated_settings)
            .map_err(|e| PluginError::config(&format!("Failed to serialize config: {}", e)))?;
        
        fs::write(&config_path, content)
            .map_err(|e| PluginError::config(&format!("Failed to write config file: {}", e)))?;
        Ok(())
    }

    /// Get configuration file path
    fn get_config_path() -> PluginResult<PathBuf> {
        // Use test-specific path if in test mode
        if cfg!(test) {
            use std::sync::atomic::{AtomicUsize, Ordering};
            static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);
            
            let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
            let temp_dir = std::env::temp_dir();
            return Ok(temp_dir.join(format!("nvim-spec-agent-test-{}.json", test_id)));
        }
        
        let home = std::env::var("HOME")
            .map_err(|_| PluginError::config("HOME environment variable not found"))?;
        
        Ok(PathBuf::from(home)
            .join(".config")
            .join("nvim")
            .join("lua")
            .join("nvim-spec-agent")
            .join("config.json"))
    }

    /// Migrate settings from older version
    fn migrate_settings(mut settings: Settings) -> PluginResult<Settings> {
        // Version-specific migrations
        match settings.version.as_str() {
            "0.1.0" => {
                // Migrate from 0.1.0 to current
                settings.version = Self::CURRENT_VERSION.to_string();
                settings.updated_at = Utc::now();
                
                // Add any new fields with defaults if they don't exist
                // This is handled by serde's default values
            }
            _ => {
                // Unknown version, reset to defaults but preserve user customizations
                let mut new_settings = Self::default();
                new_settings.ui = settings.ui;
                new_settings.agent = settings.agent;
                new_settings.spec = settings.spec;
                settings = new_settings;
            }
        }
        
        Ok(settings)
    }

    /// Migrate from legacy configuration format
    fn migrate_from_legacy(content: &str) -> PluginResult<Settings> {
        // Try to parse as legacy format (without version field)
        #[derive(Deserialize)]
        struct LegacySettings {
            ui: UiSettings,
            agent: AgentSettings,
            spec: SpecSettings,
        }
        
        match serde_json::from_str::<LegacySettings>(content) {
            Ok(legacy) => {
                let mut settings = Self::default();
                settings.ui = legacy.ui;
                settings.agent = legacy.agent;
                settings.spec = legacy.spec;
                settings.save()?;
                Ok(settings)
            }
            Err(e) => {
                Err(PluginError::config(&format!("Failed to parse legacy config: {}", e)))
            }
        }
    }

    /// Update a specific setting and save
    pub fn update_ui_setting<F>(&mut self, updater: F) -> PluginResult<()>
    where
        F: FnOnce(&mut UiSettings),
    {
        updater(&mut self.ui);
        self.save()
    }

    /// Update agent settings and save
    pub fn update_agent_setting<F>(&mut self, updater: F) -> PluginResult<()>
    where
        F: FnOnce(&mut AgentSettings),
    {
        updater(&mut self.agent);
        self.save()
    }

    /// Update spec settings and save
    pub fn update_spec_setting<F>(&mut self, updater: F) -> PluginResult<()>
    where
        F: FnOnce(&mut SpecSettings),
    {
        updater(&mut self.spec);
        self.save()
    }

    /// Update persistence settings and save
    pub fn update_persistence_setting<F>(&mut self, updater: F) -> PluginResult<()>
    where
        F: FnOnce(&mut PersistenceSettings),
    {
        updater(&mut self.persistence);
        self.save()
    }

    /// Reset to default settings
    pub fn reset_to_defaults() -> PluginResult<Self> {
        let config_path = Self::get_config_path()?;
        
        // Backup existing config if it exists
        if config_path.exists() {
            let backup_path = config_path.with_extension("json.reset_backup");
            fs::copy(&config_path, &backup_path)
                .map_err(|e| PluginError::config(&format!("Failed to backup config before reset: {}", e)))?;
        }
        
        let default_settings = Self::default();
        default_settings.save()?;
        Ok(default_settings)
    }

    /// Validate configuration settings
    pub fn validate(&self) -> PluginResult<()> {
        // Validate UI settings
        if self.ui.window_width_ratio <= 0.0 || self.ui.window_width_ratio > 1.0 {
            return Err(PluginError::config("window_width_ratio must be between 0.0 and 1.0"));
        }
        if self.ui.window_height_ratio <= 0.0 || self.ui.window_height_ratio > 1.0 {
            return Err(PluginError::config("window_height_ratio must be between 0.0 and 1.0"));
        }

        // Validate agent settings
        if self.agent.command_timeout_seconds == 0 {
            return Err(PluginError::config("command_timeout_seconds must be greater than 0"));
        }
        if self.agent.max_conversation_history == 0 {
            return Err(PluginError::config("max_conversation_history must be greater than 0"));
        }

        // Validate spec settings
        if self.spec.auto_save_interval_seconds == 0 {
            return Err(PluginError::config("auto_save_interval_seconds must be greater than 0"));
        }
        if self.spec.property_test_iterations == 0 {
            return Err(PluginError::config("property_test_iterations must be greater than 0"));
        }

        // Validate persistence settings
        if self.persistence.max_conversations == 0 {
            return Err(PluginError::config("max_conversations must be greater than 0"));
        }
        if self.persistence.conversation_retention_days == 0 {
            return Err(PluginError::config("conversation_retention_days must be greater than 0"));
        }

        Ok(())
    }
}

impl Default for Settings {
    fn default() -> Self {
        let now = Utc::now();
        Settings {
            version: Self::CURRENT_VERSION.to_string(),
            created_at: now,
            updated_at: now,
            ui: UiSettings {
                window_width_ratio: 0.8,
                window_height_ratio: 0.6,
                border_style: "rounded".to_string(),
                theme: "default".to_string(),
            },
            agent: AgentSettings {
                auto_approve_safe_commands: false,
                command_timeout_seconds: 30,
                max_conversation_history: 1000,
            },
            spec: SpecSettings {
                default_spec_directory: ".kiro/specs".to_string(),
                auto_save_interval_seconds: 30,
                property_test_iterations: 100,
                enable_property_testing: true,
                spec_template_directory: None,
            },
            persistence: PersistenceSettings {
                max_conversations: 100,
                conversation_retention_days: 30,
                auto_archive_enabled: true,
                backup_enabled: true,
                backup_interval_hours: 24,
            },
        }
    }
}