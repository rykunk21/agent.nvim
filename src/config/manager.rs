use crate::config::{Settings, PersistenceManager, PluginState};
use crate::utils::error_handling::PluginResult;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc, Duration};

/// Central configuration manager that coordinates settings and persistence
pub struct ConfigurationManager {
    settings: Arc<Mutex<Settings>>,
    persistence_manager: Arc<Mutex<PersistenceManager>>,
    plugin_state: Arc<Mutex<PluginState>>,
    workspace_path: Option<PathBuf>,
    last_settings_check: Option<DateTime<Utc>>,
}

impl ConfigurationManager {
    /// Create a new configuration manager
    pub fn new(workspace_path: Option<PathBuf>) -> PluginResult<Self> {
        let settings = Arc::new(Mutex::new(Settings::load_or_default()?));
        let persistence_manager = Arc::new(Mutex::new(PersistenceManager::new(workspace_path.clone())?));
        
        // Load plugin state
        let plugin_state = {
            let pm = persistence_manager.lock().unwrap();
            Arc::new(Mutex::new(pm.load_state()?))
        };

        Ok(ConfigurationManager {
            settings,
            persistence_manager,
            plugin_state,
            workspace_path,
            last_settings_check: None,
        })
    }

    /// Get a copy of current settings
    pub fn get_settings(&self) -> PluginResult<Settings> {
        let settings = self.settings.lock().unwrap();
        Ok(settings.clone())
    }

    /// Update settings with a closure
    pub fn update_settings<F>(&mut self, updater: F) -> PluginResult<()>
    where
        F: FnOnce(&mut Settings) -> PluginResult<()>,
    {
        let mut settings = self.settings.lock().unwrap();
        
        // Create a copy to validate before saving
        let mut updated_settings = settings.clone();
        updater(&mut updated_settings)?;
        updated_settings.validate()?;
        
        // Only update and save if validation passes
        *settings = updated_settings;
        settings.save()?;
        
        // Update persistence manager with new settings
        let mut pm = self.persistence_manager.lock().unwrap();
        pm.update_settings(settings.persistence.clone());
        
        Ok(())
    }

    /// Get persistence manager
    pub fn get_persistence_manager(&self) -> Arc<Mutex<PersistenceManager>> {
        Arc::clone(&self.persistence_manager)
    }

    /// Get plugin state
    pub fn get_plugin_state(&self) -> Arc<Mutex<PluginState>> {
        Arc::clone(&self.plugin_state)
    }

    /// Save current plugin state
    pub fn save_plugin_state(&self) -> PluginResult<()> {
        let state = self.plugin_state.lock().unwrap();
        let pm = self.persistence_manager.lock().unwrap();
        pm.save_state(&*state)
    }

    /// Perform periodic maintenance
    pub fn perform_maintenance(&mut self) -> PluginResult<MaintenanceResult> {
        let now = Utc::now();
        let mut result = MaintenanceResult::default();

        // Check for settings file changes (every 30 seconds)
        if self.should_check_settings_file(now) {
            if self.check_and_reload_settings()? {
                result.settings_reloaded = true;
            }
            self.last_settings_check = Some(now);
        }

        // Perform persistence cleanup
        let cleanup_result = {
            let mut pm = self.persistence_manager.lock().unwrap();
            pm.perform_cleanup()?
        };
        result.cleanup_result = cleanup_result;

        // Save plugin state
        self.save_plugin_state()?;

        Ok(result)
    }

    /// Check if we should check the settings file for changes
    fn should_check_settings_file(&self, now: DateTime<Utc>) -> bool {
        match self.last_settings_check {
            Some(last_check) => now.signed_duration_since(last_check) >= Duration::seconds(30),
            None => true,
        }
    }

    /// Check if settings file has changed and reload if necessary
    fn check_and_reload_settings(&self) -> PluginResult<bool> {
        // In a full implementation, this would check file modification time
        // For now, we'll just reload the settings
        match Settings::load_or_default() {
            Ok(new_settings) => {
                let mut current_settings = self.settings.lock().unwrap();
                if new_settings.updated_at > current_settings.updated_at {
                    *current_settings = new_settings;
                    
                    // Update persistence manager
                    let mut pm = self.persistence_manager.lock().unwrap();
                    pm.update_settings(current_settings.persistence.clone());
                    
                    return Ok(true);
                }
                Ok(false)
            }
            Err(_) => Ok(false), // Don't fail maintenance for settings reload issues
        }
    }

    /// Reset all configuration to defaults
    pub fn reset_to_defaults(&mut self) -> PluginResult<()> {
        // Reset settings
        let new_settings = Settings::reset_to_defaults()?;
        {
            let mut settings = self.settings.lock().unwrap();
            *settings = new_settings;
        }

        // Reset plugin state
        {
            let mut state = self.plugin_state.lock().unwrap();
            *state = PluginState::default();
        }

        // Recreate persistence manager with new settings
        let new_pm = PersistenceManager::new(self.workspace_path.clone())?;
        {
            let mut pm = self.persistence_manager.lock().unwrap();
            *pm = new_pm;
        }

        // Save everything
        self.save_plugin_state()?;

        Ok(())
    }

    /// Export configuration for backup or sharing
    pub fn export_configuration(&self) -> PluginResult<ConfigurationExport> {
        let settings = self.get_settings()?;
        let state = self.plugin_state.lock().unwrap().clone();
        
        Ok(ConfigurationExport {
            settings,
            plugin_state: state,
            exported_at: Utc::now(),
            workspace_path: self.workspace_path.clone(),
        })
    }

    /// Import configuration from backup
    pub fn import_configuration(&mut self, export: ConfigurationExport) -> PluginResult<()> {
        // Validate the import
        export.settings.validate()?;

        // Update settings
        {
            let mut settings = self.settings.lock().unwrap();
            *settings = export.settings;
            settings.save()?;
        }

        // Update plugin state
        {
            let mut state = self.plugin_state.lock().unwrap();
            *state = export.plugin_state;
        }

        // Update persistence manager
        let settings = self.get_settings()?;
        {
            let mut pm = self.persistence_manager.lock().unwrap();
            pm.update_settings(settings.persistence);
        }

        // Save everything
        self.save_plugin_state()?;

        Ok(())
    }
}

/// Result of maintenance operations
#[derive(Debug, Default)]
pub struct MaintenanceResult {
    pub settings_reloaded: bool,
    pub cleanup_result: crate::config::persistence::CleanupResult,
}

/// Configuration export for backup/sharing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConfigurationExport {
    pub settings: Settings,
    pub plugin_state: PluginState,
    pub exported_at: DateTime<Utc>,
    pub workspace_path: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config_manager() -> (ConfigurationManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let workspace_path = Some(temp_dir.path().to_path_buf());
        let config_manager = ConfigurationManager::new(workspace_path).unwrap();
        (config_manager, temp_dir)
    }

    #[test]
    fn test_configuration_manager_creation() {
        let (config_manager, _temp_dir) = create_test_config_manager();
        let settings = config_manager.get_settings().unwrap();
        assert_eq!(settings.version, Settings::CURRENT_VERSION);
    }

    #[test]
    fn test_settings_update() {
        let (mut config_manager, _temp_dir) = create_test_config_manager();
        
        config_manager.update_settings(|settings| {
            settings.ui.window_width_ratio = 0.9;
            Ok(())
        }).unwrap();
        
        let updated_settings = config_manager.get_settings().unwrap();
        assert_eq!(updated_settings.ui.window_width_ratio, 0.9);
    }

    #[test]
    fn test_plugin_state_management() {
        let (config_manager, _temp_dir) = create_test_config_manager();
        
        {
            let plugin_state_arc = config_manager.get_plugin_state();
            let mut state = plugin_state_arc.lock().unwrap();
            state.set_active_spec(Some("test-spec".to_string()));
        }
        
        config_manager.save_plugin_state().unwrap();
        
        let plugin_state_arc = config_manager.get_plugin_state();
        let state = plugin_state_arc.lock().unwrap();
        assert_eq!(state.active_spec, Some("test-spec".to_string()));
    }

    #[test]
    fn test_configuration_export_import() {
        let (mut config_manager, _temp_dir) = create_test_config_manager();
        
        // Modify some settings
        config_manager.update_settings(|settings| {
            settings.ui.theme = "dark".to_string();
            Ok(())
        }).unwrap();
        
        // Export configuration
        let export = config_manager.export_configuration().unwrap();
        assert_eq!(export.settings.ui.theme, "dark");
        
        // Reset to defaults
        config_manager.reset_to_defaults().unwrap();
        let reset_settings = config_manager.get_settings().unwrap();
        assert_eq!(reset_settings.ui.theme, "default");
        
        // Import the exported configuration
        config_manager.import_configuration(export).unwrap();
        let imported_settings = config_manager.get_settings().unwrap();
        assert_eq!(imported_settings.ui.theme, "dark");
    }

    #[test]
    fn test_settings_validation() {
        let (mut config_manager, _temp_dir) = create_test_config_manager();
        
        // Try to set invalid settings
        let result = config_manager.update_settings(|settings| {
            settings.ui.window_width_ratio = 1.5; // Invalid: > 1.0
            Ok(())
        });
        
        assert!(result.is_err());
        
        // Verify settings weren't changed
        let settings = config_manager.get_settings().unwrap();
        assert_eq!(settings.ui.window_width_ratio, 0.8); // Default value
    }
}