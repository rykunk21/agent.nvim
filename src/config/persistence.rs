use crate::agent::chat_manager::Conversation;
use crate::utils::error_handling::{PluginResult, PluginError};
use crate::config::settings::{Settings, PersistenceSettings};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;
use chrono::{DateTime, Utc, Duration};
use std::collections::HashMap;

/// Manages conversation and state persistence
pub struct PersistenceManager {
    pub data_directory: PathBuf,
    pub settings: PersistenceSettings,
    pub last_cleanup: Option<DateTime<Utc>>,
}

impl PersistenceManager {
    pub fn new(workspace_path: Option<PathBuf>) -> PluginResult<Self> {
        let settings = Settings::load_or_default()?.persistence;
        
        let data_directory = if let Some(workspace) = workspace_path {
            // Use workspace-specific directory
            workspace.join(".nvim-spec-agent")
        } else {
            // Use global directory
            let home = std::env::var("HOME")
                .map_err(|_| PluginError::unknown("HOME environment variable not found"))?;
            
            PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("nvim-spec-agent")
        };

        fs::create_dir_all(&data_directory)
            .map_err(|e| PluginError::unknown(&format!("Failed to create data directory: {}", e)))?;

        // Create subdirectories
        for subdir in &["conversations", "archive", "backups", "state"] {
            fs::create_dir_all(data_directory.join(subdir))
                .map_err(|e| PluginError::unknown(&format!("Failed to create {} directory: {}", subdir, e)))?;
        }

        Ok(PersistenceManager { 
            data_directory,
            settings,
            last_cleanup: None,
        })
    }

    /// Update persistence settings
    pub fn update_settings(&mut self, settings: PersistenceSettings) {
        self.settings = settings;
    }

    /// Save conversation to disk
    pub fn save_conversation(&self, conversation: &Conversation) -> PluginResult<()> {
        let file_path = self.data_directory
            .join("conversations")
            .join(format!("{}.json", conversation.id));

        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| PluginError::unknown(&format!("Failed to create conversations directory: {}", e)))?;
        }

        let content = serde_json::to_string_pretty(conversation)
            .map_err(|e| PluginError::unknown(&format!("Failed to serialize conversation: {}", e)))?;

        fs::write(&file_path, content)
            .map_err(|e| PluginError::unknown(&format!("Failed to write conversation file: {}", e)))?;
        Ok(())
    }

    /// Save all conversations to disk
    pub fn save_conversations(&self, conversations: &[Conversation]) -> PluginResult<()> {
        for conversation in conversations {
            self.save_conversation(conversation)?;
        }
        Ok(())
    }

    /// Load all conversations from disk
    pub fn load_conversations(&self) -> PluginResult<Vec<Conversation>> {
        let conversation_ids = self.list_conversations()?;
        let mut conversations = Vec::new();

        for id in conversation_ids {
            match self.load_conversation(id) {
                Ok(conversation) => conversations.push(conversation),
                Err(_) => {
                    // Skip corrupted conversations but continue loading others
                    continue;
                }
            }
        }

        // Sort by last updated time
        conversations.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(conversations)
    }

    /// Archive old conversations based on settings
    pub fn archive_conversations(&self, conversations: &[Conversation]) -> PluginResult<()> {
        let archive_dir = self.data_directory.join("archive");
        
        for conversation in conversations {
            let archive_path = archive_dir.join(format!("{}.json", conversation.id));
            let content = serde_json::to_string_pretty(conversation)
                .map_err(|e| PluginError::unknown(&format!("Failed to serialize conversation: {}", e)))?;

            fs::write(&archive_path, content)
                .map_err(|e| PluginError::unknown(&format!("Failed to archive conversation: {}", e)))?;

            // Remove from active conversations
            let active_path = self.data_directory
                .join("conversations")
                .join(format!("{}.json", conversation.id));
            
            if active_path.exists() {
                fs::remove_file(&active_path)
                    .map_err(|e| PluginError::unknown(&format!("Failed to remove archived conversation: {}", e)))?;
            }
        }

        Ok(())
    }

    /// Perform automatic cleanup based on settings
    pub fn perform_cleanup(&mut self) -> PluginResult<CleanupResult> {
        let now = Utc::now();
        let mut result = CleanupResult::default();

        // Check if cleanup is needed
        if let Some(last_cleanup) = self.last_cleanup {
            if now.signed_duration_since(last_cleanup) < Duration::hours(1) {
                return Ok(result); // Skip cleanup if done recently
            }
        }

        // Archive old conversations if auto-archive is enabled
        if self.settings.auto_archive_enabled {
            let conversations = self.load_conversations()?;
            let retention_cutoff = now - Duration::days(self.settings.conversation_retention_days as i64);
            
            let to_archive: Vec<_> = conversations
                .iter()
                .filter(|c| c.updated_at < retention_cutoff)
                .cloned()
                .collect();

            if !to_archive.is_empty() {
                self.archive_conversations(&to_archive)?;
                result.conversations_archived = to_archive.len();
            }

            // Limit active conversations
            let remaining_conversations = self.load_conversations()?;
            if remaining_conversations.len() > self.settings.max_conversations {
                let excess_count = remaining_conversations.len() - self.settings.max_conversations;
                let mut sorted_conversations = remaining_conversations;
                sorted_conversations.sort_by(|a, b| a.updated_at.cmp(&b.updated_at));
                
                let to_archive: Vec<_> = sorted_conversations
                    .into_iter()
                    .take(excess_count)
                    .collect();
                
                self.archive_conversations(&to_archive)?;
                result.conversations_archived += to_archive.len();
            }
        }

        // Create backup if enabled
        if self.settings.backup_enabled {
            self.create_backup()?;
            result.backup_created = true;
        }

        // Clean up old backups (keep last 7 days)
        self.cleanup_old_backups(7)?;

        self.last_cleanup = Some(now);
        Ok(result)
    }

    /// Create a backup of all data
    pub fn create_backup(&self) -> PluginResult<()> {
        let backup_dir = self.data_directory.join("backups");
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let backup_path = backup_dir.join(format!("backup_{}.tar.gz", timestamp));

        // For now, just copy the conversations directory
        // In a full implementation, this could create a proper tar.gz archive
        let backup_conversations_dir = backup_dir.join(format!("conversations_{}", timestamp));
        fs::create_dir_all(&backup_conversations_dir)
            .map_err(|e| PluginError::unknown(&format!("Failed to create backup directory: {}", e)))?;

        let conversations_dir = self.data_directory.join("conversations");
        if conversations_dir.exists() {
            for entry in fs::read_dir(&conversations_dir)
                .map_err(|e| PluginError::unknown(&format!("Failed to read conversations directory: {}", e)))? {
                let entry = entry
                    .map_err(|e| PluginError::unknown(&format!("Failed to read directory entry: {}", e)))?;
                let dest_path = backup_conversations_dir.join(entry.file_name());
                fs::copy(entry.path(), dest_path)
                    .map_err(|e| PluginError::unknown(&format!("Failed to copy conversation file: {}", e)))?;
            }
        }

        Ok(())
    }

    /// Clean up old backups
    fn cleanup_old_backups(&self, keep_days: u32) -> PluginResult<()> {
        let backup_dir = self.data_directory.join("backups");
        if !backup_dir.exists() {
            return Ok(());
        }

        let cutoff = Utc::now() - Duration::days(keep_days as i64);

        for entry in fs::read_dir(&backup_dir)
            .map_err(|e| PluginError::unknown(&format!("Failed to read backups directory: {}", e)))? {
            let entry = entry
                .map_err(|e| PluginError::unknown(&format!("Failed to read directory entry: {}", e)))?;
            
            if let Ok(metadata) = entry.metadata() {
                if let Ok(created) = metadata.created() {
                    let created_datetime: DateTime<Utc> = created.into();
                    if created_datetime < cutoff {
                        if entry.path().is_dir() {
                            fs::remove_dir_all(entry.path())
                                .map_err(|e| PluginError::unknown(&format!("Failed to remove old backup directory: {}", e)))?;
                        } else {
                            fs::remove_file(entry.path())
                                .map_err(|e| PluginError::unknown(&format!("Failed to remove old backup file: {}", e)))?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Load conversation from disk
    pub fn load_conversation(&self, id: Uuid) -> PluginResult<Conversation> {
        let file_path = self.data_directory
            .join("conversations")
            .join(format!("{}.json", id));

        let content = fs::read_to_string(&file_path)
            .map_err(|e| PluginError::unknown(&format!("Failed to read conversation file: {}", e)))?;
        
        let conversation: Conversation = serde_json::from_str(&content)
            .map_err(|e| PluginError::unknown(&format!("Failed to parse conversation: {}", e)))?;

        Ok(conversation)
    }

    /// List all saved conversations
    pub fn list_conversations(&self) -> PluginResult<Vec<Uuid>> {
        let conversations_dir = self.data_directory.join("conversations");
        
        if !conversations_dir.exists() {
            return Ok(Vec::new());
        }

        let mut conversation_ids = Vec::new();
        
        let entries = fs::read_dir(&conversations_dir)
            .map_err(|e| PluginError::unknown(&format!("Failed to read conversations directory: {}", e)))?;

        for entry in entries {
            let entry = entry
                .map_err(|e| PluginError::unknown(&format!("Failed to read directory entry: {}", e)))?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(id) = Uuid::parse_str(stem) {
                        conversation_ids.push(id);
                    }
                }
            }
        }

        Ok(conversation_ids)
    }

    /// Save plugin state
    pub fn save_state(&self, state: &PluginState) -> PluginResult<()> {
        let file_path = self.data_directory.join("state.json");

        let content = serde_json::to_string_pretty(state)
            .map_err(|e| PluginError::unknown(&format!("Failed to serialize state: {}", e)))?;

        fs::write(&file_path, content)
            .map_err(|e| PluginError::unknown(&format!("Failed to write state file: {}", e)))?;
        Ok(())
    }

    /// Load plugin state
    pub fn load_state(&self) -> PluginResult<PluginState> {
        let file_path = self.data_directory.join("state.json");

        if !file_path.exists() {
            return Ok(PluginState::default());
        }

        let content = fs::read_to_string(&file_path)
            .map_err(|e| PluginError::unknown(&format!("Failed to read state file: {}", e)))?;
        
        let state: PluginState = serde_json::from_str(&content)
            .map_err(|e| PluginError::unknown(&format!("Failed to parse state: {}", e)))?;

        Ok(state)
    }
}

/// Plugin state for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginState {
    pub version: String,
    pub last_conversation_id: Option<Uuid>,
    pub window_positions: Vec<WindowPosition>,
    pub active_spec: Option<String>,
    pub workspace_states: HashMap<String, WorkspaceState>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowPosition {
    pub window_type: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub z_index: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceState {
    pub last_opened: DateTime<Utc>,
    pub active_conversation: Option<Uuid>,
    pub open_specs: Vec<String>,
    pub window_layout: Option<String>,
}

/// Result of cleanup operations
#[derive(Debug, Default)]
pub struct CleanupResult {
    pub conversations_archived: usize,
    pub backup_created: bool,
    pub old_backups_removed: usize,
}

impl Default for PluginState {
    fn default() -> Self {
        PluginState {
            version: "1.0.0".to_string(),
            last_conversation_id: None,
            window_positions: Vec::new(),
            active_spec: None,
            workspace_states: HashMap::new(),
            last_updated: Utc::now(),
        }
    }
}

impl PluginState {
    /// Update workspace state
    pub fn update_workspace_state(&mut self, workspace_path: &str, state: WorkspaceState) {
        self.workspace_states.insert(workspace_path.to_string(), state);
        self.last_updated = Utc::now();
    }

    /// Get workspace state
    pub fn get_workspace_state(&self, workspace_path: &str) -> Option<&WorkspaceState> {
        self.workspace_states.get(workspace_path)
    }

    /// Update window positions
    pub fn update_window_positions(&mut self, positions: Vec<WindowPosition>) {
        self.window_positions = positions;
        self.last_updated = Utc::now();
    }

    /// Set active conversation
    pub fn set_active_conversation(&mut self, conversation_id: Option<Uuid>) {
        self.last_conversation_id = conversation_id;
        self.last_updated = Utc::now();
    }

    /// Set active spec
    pub fn set_active_spec(&mut self, spec_name: Option<String>) {
        self.active_spec = spec_name;
        self.last_updated = Utc::now();
    }
}