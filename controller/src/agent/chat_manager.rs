use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::io::Result;
use std::path::PathBuf;
use crate::utils::error_handling::{PluginResult, PluginError};
use crate::config::persistence::PersistenceManager;

/// Manages conversation history and context
pub struct ChatManager {
    pub conversations: Vec<Conversation>,
    pub current_conversation: Option<Uuid>,
    pub workspace_context: WorkspaceContext,
    pub persistence_manager: PersistenceManager,
    pub message_formatter: MessageFormatter,
    pub response_parser: ResponseParser,
}

impl ChatManager {
    pub fn new(workspace_path: Option<PathBuf>) -> PluginResult<Self> {
        let persistence_manager = PersistenceManager::new(workspace_path.clone())?;
        let workspace_context = WorkspaceContext::new(workspace_path);
        
        let mut chat_manager = ChatManager {
            conversations: Vec::new(),
            current_conversation: None,
            workspace_context,
            persistence_manager,
            message_formatter: MessageFormatter::new(),
            response_parser: ResponseParser::new(),
        };
        
        // Load existing conversations from persistence
        chat_manager.load_conversations()?;
        
        Ok(chat_manager)
    }

    /// Create a new conversation
    pub fn create_conversation(&mut self) -> Uuid {
        let conversation = Conversation::new();
        let id = conversation.id;
        self.conversations.push(conversation);
        self.current_conversation = Some(id);
        id
    }

    /// Add a message to the current conversation
    pub fn add_message(&mut self, role: MessageRole, content: MessageContent) -> Result<()> {
        if let Some(conv_id) = self.current_conversation {
            if let Some(conversation) = self.conversations.iter_mut().find(|c| c.id == conv_id) {
                conversation.add_message(role, content);
            }
        }
        Ok(())
    }

    /// Get current conversation
    pub fn get_current_conversation(&self) -> Option<&Conversation> {
        self.current_conversation
            .and_then(|id| self.conversations.iter().find(|c| c.id == id))
    }

    /// Get mutable reference to current conversation
    pub fn get_current_conversation_mut(&mut self) -> Option<&mut Conversation> {
        self.current_conversation
            .and_then(|id| self.conversations.iter_mut().find(|c| c.id == id))
    }

    /// Load conversations from persistence
    pub fn load_conversations(&mut self) -> PluginResult<()> {
        match self.persistence_manager.load_conversations() {
            Ok(conversations) => {
                self.conversations = conversations;
                // Set the most recent conversation as current if none is set
                if self.current_conversation.is_none() && !self.conversations.is_empty() {
                    let most_recent = self.conversations
                        .iter()
                        .max_by_key(|c| c.updated_at)
                        .map(|c| c.id);
                    self.current_conversation = most_recent;
                }
                Ok(())
            }
            Err(_) => {
                // If loading fails, start with empty conversations
                self.conversations = Vec::new();
                Ok(())
            }
        }
    }

    /// Save conversations to persistence
    pub fn save_conversations(&self) -> PluginResult<()> {
        self.persistence_manager.save_conversations(&self.conversations)
    }

    /// Parse and add agent response to current conversation
    pub fn add_agent_response(&mut self, response_text: &str) -> PluginResult<Vec<MessageContent>> {
        let parsed_content = self.response_parser.parse_response(response_text)?;
        
        for content in &parsed_content {
            self.add_message(MessageRole::Agent, content.clone())?;
        }
        
        // Save after adding messages
        self.save_conversations()?;
        
        Ok(parsed_content)
    }

    /// Add user message with automatic formatting
    pub fn add_user_message(&mut self, text: &str) -> PluginResult<()> {
        let formatted_content = self.message_formatter.format_user_message(text);
        self.add_message(MessageRole::User, formatted_content)?;
        self.save_conversations()?;
        Ok(())
    }

    /// Get formatted conversation history for display
    pub fn get_formatted_history(&self, limit: Option<usize>) -> Vec<String> {
        if let Some(conversation) = self.get_current_conversation() {
            let messages: Vec<&Message> = if let Some(limit) = limit {
                conversation.messages.iter().rev().take(limit).rev().collect()
            } else {
                conversation.messages.iter().collect()
            };
            
            self.message_formatter.format_conversation_history(&messages)
        } else {
            vec!["No active conversation".to_string()]
        }
    }

    /// Update workspace context with current project state
    pub fn update_workspace_context(&mut self, file_tree: Vec<String>, open_files: Vec<String>) -> PluginResult<()> {
        self.workspace_context.update_file_tree(file_tree);
        self.workspace_context.update_open_files(open_files);
        
        // Create snapshot before borrowing conversation mutably
        let snapshot = self.workspace_context.create_snapshot();
        
        // Update current conversation's context if it exists
        if let Some(conversation) = self.get_current_conversation_mut() {
            conversation.workspace_snapshot = Some(snapshot);
        }
        
        Ok(())
    }

    /// Get conversation context for agent
    pub fn get_conversation_context(&self) -> ConversationContext {
        ConversationContext {
            current_conversation_id: self.current_conversation,
            message_count: self.get_current_conversation()
                .map(|c| c.messages.len())
                .unwrap_or(0),
            spec_context: self.get_current_conversation()
                .and_then(|c| c.spec_context.clone()),
            workspace_context: self.workspace_context.clone(),
        }
    }

    /// Switch to a different conversation
    pub fn switch_conversation(&mut self, conversation_id: Uuid) -> PluginResult<()> {
        if self.conversations.iter().any(|c| c.id == conversation_id) {
            self.current_conversation = Some(conversation_id);
            Ok(())
        } else {
            Err(PluginError::unknown(&format!("Conversation not found: {}", conversation_id)))
        }
    }

    /// Archive old conversations to maintain performance
    pub fn archive_old_conversations(&mut self, max_conversations: usize) -> PluginResult<()> {
        if self.conversations.len() > max_conversations {
            // Sort by last updated time and keep the most recent ones
            self.conversations.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            
            let to_archive = self.conversations.split_off(max_conversations);
            
            // Archive the old conversations
            self.persistence_manager.archive_conversations(&to_archive)?;
        }
        
        Ok(())
    }

    /// Search conversations by content
    pub fn search_conversations(&self, query: &str) -> Vec<ConversationSearchResult> {
        let mut results = Vec::new();
        
        for conversation in &self.conversations {
            for (msg_index, message) in conversation.messages.iter().enumerate() {
                if let MessageContent::Text(text) = &message.content {
                    if text.to_lowercase().contains(&query.to_lowercase()) {
                        results.push(ConversationSearchResult {
                            conversation_id: conversation.id,
                            message_index: msg_index,
                            message_id: message.id,
                            snippet: self.create_search_snippet(text, query),
                            timestamp: message.timestamp,
                        });
                    }
                }
            }
        }
        
        // Sort by relevance (most recent first for now)
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        results
    }

    /// Create a search snippet with highlighted query
    fn create_search_snippet(&self, text: &str, query: &str) -> String {
        let query_lower = query.to_lowercase();
        let text_lower = text.to_lowercase();
        
        if let Some(pos) = text_lower.find(&query_lower) {
            let start = pos.saturating_sub(50);
            let end = (pos + query.len() + 50).min(text.len());
            
            let mut snippet = text[start..end].to_string();
            if start > 0 {
                snippet = format!("...{}", snippet);
            }
            if end < text.len() {
                snippet = format!("{}...", snippet);
            }
            
            snippet
        } else {
            text.chars().take(100).collect::<String>()
        }
    }
}

/// Conversation structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: Uuid,
    pub messages: Vec<Message>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub spec_context: Option<SpecContext>,
    pub workspace_snapshot: Option<WorkspaceSnapshot>,
    pub conversation_metadata: ConversationMetadata,
}

impl Conversation {
    pub fn new() -> Self {
        let now = Utc::now();
        Conversation {
            id: Uuid::new_v4(),
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
            spec_context: None,
            workspace_snapshot: None,
            conversation_metadata: ConversationMetadata::default(),
        }
    }

    pub fn add_message(&mut self, role: MessageRole, content: MessageContent) {
        let message = Message::new(role, content);
        self.messages.push(message);
        self.updated_at = Utc::now();
        
        // Update conversation metadata
        self.conversation_metadata.message_count = self.messages.len();
        self.conversation_metadata.last_activity = self.updated_at;
    }

    /// Get conversation summary for display
    pub fn get_summary(&self) -> String {
        if self.messages.is_empty() {
            return "Empty conversation".to_string();
        }

        // Try to get a meaningful summary from the first few messages
        let first_user_message = self.messages
            .iter()
            .find(|m| matches!(m.role, MessageRole::User))
            .and_then(|m| {
                if let MessageContent::Text(text) = &m.content {
                    Some(text.chars().take(100).collect::<String>())
                } else {
                    None
                }
            });

        first_user_message.unwrap_or_else(|| {
            format!("Conversation from {}", self.created_at.format("%Y-%m-%d %H:%M"))
        })
    }

    /// Get message count by role
    pub fn get_message_counts(&self) -> (usize, usize, usize) {
        let mut user_count = 0;
        let mut agent_count = 0;
        let mut system_count = 0;

        for message in &self.messages {
            match message.role {
                MessageRole::User => user_count += 1,
                MessageRole::Agent => agent_count += 1,
                MessageRole::System => system_count += 1,
            }
        }

        (user_count, agent_count, system_count)
    }

    /// Check if conversation has spec context
    pub fn has_spec_context(&self) -> bool {
        self.spec_context.is_some()
    }

    /// Update spec context
    pub fn update_spec_context(&mut self, spec_context: SpecContext) {
        self.spec_context = Some(spec_context);
        self.updated_at = Utc::now();
    }
}

/// Message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub role: MessageRole,
    pub content: MessageContent,
    pub timestamp: DateTime<Utc>,
    pub metadata: MessageMetadata,
}

impl Message {
    pub fn new(role: MessageRole, content: MessageContent) -> Self {
        Message {
            id: Uuid::new_v4(),
            role,
            content,
            timestamp: Utc::now(),
            metadata: MessageMetadata::default(),
        }
    }
}

/// Message role
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Agent,
    System,
}

/// Message content types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    Text(String),
    OperationBlock(crate::ui::OperationBlock),
    CommandBlock(crate::agent::CommandBlock),
    SpecUpdate(SpecUpdate),
}

/// Message metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageMetadata {
    pub tokens: Option<u32>,
    pub model: Option<String>,
}

/// Spec context for conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecContext {
    pub feature_name: String,
    pub current_phase: SpecPhase,
    pub requirements: Option<RequirementsDocument>,
    pub design: Option<DesignDocument>,
    pub tasks: Option<TasksDocument>,
}

/// Requirements document structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementsDocument {
    pub content: String,
    pub approved: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Design document structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignDocument {
    pub content: String,
    pub approved: bool,
    pub correctness_properties: Vec<CorrectnessProperty>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Tasks document structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TasksDocument {
    pub content: String,
    pub approved: bool,
    pub tasks: Vec<Task>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Individual task structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub completed: bool,
    pub dependencies: Vec<String>,
    pub requirements_refs: Vec<String>,
}

/// Correctness property for property-based testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectnessProperty {
    pub id: String,
    pub name: String,
    pub description: String,
    pub property_type: PropertyType,
    pub requirements_refs: Vec<String>,
}

/// Types of correctness properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropertyType {
    Invariant,
    RoundTrip,
    Idempotence,
    Metamorphic,
    ModelBased,
    Confluence,
    ErrorCondition,
}

/// Spec development phases
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SpecPhase {
    Requirements,
    Design,
    Tasks,
    Implementation,
}

/// Spec update information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecUpdate {
    pub phase: SpecPhase,
    pub action: String,
    pub details: String,
}

/// Workspace context for maintaining project understanding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceContext {
    pub workspace_path: Option<PathBuf>,
    pub file_tree: Vec<String>,
    pub open_files: Vec<String>,
    pub project_type: Option<ProjectType>,
    pub last_updated: DateTime<Utc>,
}

impl WorkspaceContext {
    pub fn new(workspace_path: Option<PathBuf>) -> Self {
        WorkspaceContext {
            workspace_path,
            file_tree: Vec::new(),
            open_files: Vec::new(),
            project_type: None,
            last_updated: Utc::now(),
        }
    }

    pub fn update_file_tree(&mut self, file_tree: Vec<String>) {
        self.file_tree = file_tree;
        self.last_updated = Utc::now();
        
        // Try to detect project type from file tree
        self.detect_project_type();
    }

    pub fn update_open_files(&mut self, open_files: Vec<String>) {
        self.open_files = open_files;
        self.last_updated = Utc::now();
    }

    fn detect_project_type(&mut self) {
        // Simple project type detection based on common files
        if self.file_tree.iter().any(|f| f.ends_with("Cargo.toml")) {
            self.project_type = Some(ProjectType::Rust);
        } else if self.file_tree.iter().any(|f| f.ends_with("package.json")) {
            self.project_type = Some(ProjectType::JavaScript);
        } else if self.file_tree.iter().any(|f| f.ends_with("pyproject.toml") || f.ends_with("requirements.txt")) {
            self.project_type = Some(ProjectType::Python);
        } else if self.file_tree.iter().any(|f| f.ends_with("go.mod")) {
            self.project_type = Some(ProjectType::Go);
        } else {
            self.project_type = Some(ProjectType::Unknown);
        }
    }

    pub fn create_snapshot(&self) -> WorkspaceSnapshot {
        WorkspaceSnapshot {
            file_tree: self.file_tree.clone(),
            open_files: self.open_files.clone(),
            project_type: self.project_type.clone(),
            timestamp: Utc::now(),
        }
    }
}

/// Project type detection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProjectType {
    Rust,
    JavaScript,
    Python,
    Go,
    Unknown,
}

/// Snapshot of workspace state at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSnapshot {
    pub file_tree: Vec<String>,
    pub open_files: Vec<String>,
    pub project_type: Option<ProjectType>,
    pub timestamp: DateTime<Utc>,
}

/// Conversation metadata for tracking and organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMetadata {
    pub message_count: usize,
    pub last_activity: DateTime<Utc>,
    pub tags: Vec<String>,
    pub archived: bool,
}

impl Default for ConversationMetadata {
    fn default() -> Self {
        ConversationMetadata {
            message_count: 0,
            last_activity: Utc::now(),
            tags: Vec::new(),
            archived: false,
        }
    }
}

/// Context information for agent interactions
#[derive(Debug, Clone)]
pub struct ConversationContext {
    pub current_conversation_id: Option<Uuid>,
    pub message_count: usize,
    pub spec_context: Option<SpecContext>,
    pub workspace_context: WorkspaceContext,
}

/// Search result for conversation content
#[derive(Debug, Clone)]
pub struct ConversationSearchResult {
    pub conversation_id: Uuid,
    pub message_index: usize,
    pub message_id: Uuid,
    pub snippet: String,
    pub timestamp: DateTime<Utc>,
}

/// Message formatter for display and rendering
pub struct MessageFormatter {
    pub max_line_length: usize,
    pub timestamp_format: String,
}

impl MessageFormatter {
    pub fn new() -> Self {
        MessageFormatter {
            max_line_length: 80,
            timestamp_format: "%H:%M:%S".to_string(),
        }
    }

    /// Format user message content
    pub fn format_user_message(&self, text: &str) -> MessageContent {
        MessageContent::Text(text.to_string())
    }

    /// Format conversation history for display
    pub fn format_conversation_history(&self, messages: &[&Message]) -> Vec<String> {
        let mut formatted_lines = Vec::new();

        for message in messages {
            let role_prefix = match message.role {
                MessageRole::User => "👤 You:",
                MessageRole::Agent => "🤖 Agent:",
                MessageRole::System => "⚙️  System:",
            };

            let timestamp = message.timestamp.format(&self.timestamp_format);
            
            match &message.content {
                MessageContent::Text(text) => {
                    formatted_lines.push(format!("{} [{}]", role_prefix, timestamp));
                    
                    // Wrap long lines
                    for line in text.lines() {
                        if line.len() > self.max_line_length {
                            for chunk in line.chars().collect::<Vec<_>>().chunks(self.max_line_length) {
                                let chunk_str: String = chunk.iter().collect();
                                formatted_lines.push(format!("  {}", chunk_str));
                            }
                        } else {
                            formatted_lines.push(format!("  {}", line));
                        }
                    }
                }
                MessageContent::OperationBlock(_) => {
                    formatted_lines.push(format!("{} [{}] 📋 Operation Block", role_prefix, timestamp));
                }
                MessageContent::CommandBlock(_) => {
                    formatted_lines.push(format!("{} [{}] 💻 Command Block", role_prefix, timestamp));
                }
                MessageContent::SpecUpdate(spec_update) => {
                    formatted_lines.push(format!("{} [{}] 📝 Spec Update: {} - {}", 
                        role_prefix, timestamp, spec_update.action, spec_update.details));
                }
            }
            
            formatted_lines.push(String::new()); // Add separator
        }

        formatted_lines
    }

    /// Format a single message for display
    pub fn format_message(&self, message: &Message) -> Vec<String> {
        self.format_conversation_history(&[message])
    }
}

/// Response parser for handling agent responses
pub struct ResponseParser {
    pub command_patterns: Vec<String>,
    pub operation_patterns: Vec<String>,
}

impl ResponseParser {
    pub fn new() -> Self {
        ResponseParser {
            command_patterns: vec![
                "```bash".to_string(),
                "```sh".to_string(),
                "```shell".to_string(),
            ],
            operation_patterns: vec![
                "Reading file:".to_string(),
                "Writing file:".to_string(),
                "Creating file:".to_string(),
            ],
        }
    }

    /// Parse agent response into structured content
    pub fn parse_response(&self, response_text: &str) -> PluginResult<Vec<MessageContent>> {
        let mut content_blocks = Vec::new();
        let mut current_text = String::new();
        let mut in_code_block = false;
        let mut code_block_content = String::new();

        for line in response_text.lines() {
            // Check for code block markers
            if self.command_patterns.iter().any(|pattern| line.trim().starts_with(pattern)) {
                // Save any accumulated text
                if !current_text.trim().is_empty() {
                    content_blocks.push(MessageContent::Text(current_text.trim().to_string()));
                    current_text.clear();
                }
                in_code_block = true;
                code_block_content.clear();
                continue;
            }

            if in_code_block && line.trim() == "```" {
                // End of code block - this could be a command
                if !code_block_content.trim().is_empty() {
                    // For now, treat as text. In a full implementation, this could create CommandBlocks
                    content_blocks.push(MessageContent::Text(format!("Command:\n{}", code_block_content.trim())));
                }
                in_code_block = false;
                code_block_content.clear();
                continue;
            }

            if in_code_block {
                code_block_content.push_str(line);
                code_block_content.push('\n');
            } else {
                // Check for operation indicators
                if self.operation_patterns.iter().any(|pattern| line.contains(pattern)) {
                    // Save any accumulated text
                    if !current_text.trim().is_empty() {
                        content_blocks.push(MessageContent::Text(current_text.trim().to_string()));
                        current_text.clear();
                    }
                    
                    // For now, treat as text. In a full implementation, this could create OperationBlocks
                    content_blocks.push(MessageContent::Text(line.to_string()));
                } else {
                    current_text.push_str(line);
                    current_text.push('\n');
                }
            }
        }

        // Add any remaining text
        if !current_text.trim().is_empty() {
            content_blocks.push(MessageContent::Text(current_text.trim().to_string()));
        }

        // If no structured content was found, treat the entire response as text
        if content_blocks.is_empty() {
            content_blocks.push(MessageContent::Text(response_text.to_string()));
        }

        Ok(content_blocks)
    }

    /// Check if response contains commands that need approval
    pub fn contains_commands(&self, response_text: &str) -> bool {
        self.command_patterns.iter().any(|pattern| response_text.contains(pattern))
    }

    /// Extract commands from response text
    pub fn extract_commands(&self, response_text: &str) -> Vec<String> {
        let mut commands = Vec::new();
        let mut in_code_block = false;
        let mut current_command = String::new();

        for line in response_text.lines() {
            if self.command_patterns.iter().any(|pattern| line.trim().starts_with(pattern)) {
                in_code_block = true;
                current_command.clear();
                continue;
            }

            if in_code_block && line.trim() == "```" {
                if !current_command.trim().is_empty() {
                    commands.push(current_command.trim().to_string());
                }
                in_code_block = false;
                current_command.clear();
                continue;
            }

            if in_code_block {
                current_command.push_str(line);
                current_command.push('\n');
            }
        }

        commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_chat_manager() -> (ChatManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let workspace_path = Some(temp_dir.path().to_path_buf());
        let chat_manager = ChatManager::new(workspace_path).unwrap();
        (chat_manager, temp_dir)
    }

    #[test]
    fn test_chat_manager_creation() {
        let (chat_manager, _temp_dir) = create_test_chat_manager();
        assert_eq!(chat_manager.conversations.len(), 0);
        assert!(chat_manager.current_conversation.is_none());
    }

    #[test]
    fn test_create_conversation() {
        let (mut chat_manager, _temp_dir) = create_test_chat_manager();
        
        let conversation_id = chat_manager.create_conversation();
        
        assert_eq!(chat_manager.conversations.len(), 1);
        assert_eq!(chat_manager.current_conversation, Some(conversation_id));
        
        let conversation = chat_manager.get_current_conversation().unwrap();
        assert_eq!(conversation.id, conversation_id);
        assert_eq!(conversation.messages.len(), 0);
    }

    #[test]
    fn test_add_user_message() {
        let (mut chat_manager, _temp_dir) = create_test_chat_manager();
        
        chat_manager.create_conversation();
        chat_manager.add_user_message("Hello, world!").unwrap();
        
        let conversation = chat_manager.get_current_conversation().unwrap();
        assert_eq!(conversation.messages.len(), 1);
        
        let message = &conversation.messages[0];
        assert!(matches!(message.role, MessageRole::User));
        if let MessageContent::Text(text) = &message.content {
            assert_eq!(text, "Hello, world!");
        } else {
            panic!("Expected text content");
        }
    }

    #[test]
    fn test_add_agent_response() {
        let (mut chat_manager, _temp_dir) = create_test_chat_manager();
        
        chat_manager.create_conversation();
        let parsed_content = chat_manager.add_agent_response("This is an agent response.").unwrap();
        
        assert_eq!(parsed_content.len(), 1);
        if let MessageContent::Text(text) = &parsed_content[0] {
            assert_eq!(text, "This is an agent response.");
        } else {
            panic!("Expected text content");
        }
        
        let conversation = chat_manager.get_current_conversation().unwrap();
        assert_eq!(conversation.messages.len(), 1);
        
        let message = &conversation.messages[0];
        assert!(matches!(message.role, MessageRole::Agent));
    }

    #[test]
    fn test_workspace_context_update() {
        let (mut chat_manager, _temp_dir) = create_test_chat_manager();
        
        chat_manager.create_conversation();
        
        let file_tree = vec!["src/main.rs".to_string(), "Cargo.toml".to_string()];
        let open_files = vec!["src/main.rs".to_string()];
        
        chat_manager.update_workspace_context(file_tree.clone(), open_files.clone()).unwrap();
        
        assert_eq!(chat_manager.workspace_context.file_tree, file_tree);
        assert_eq!(chat_manager.workspace_context.open_files, open_files);
        assert_eq!(chat_manager.workspace_context.project_type, Some(ProjectType::Rust));
        
        let conversation = chat_manager.get_current_conversation().unwrap();
        assert!(conversation.workspace_snapshot.is_some());
    }

    #[test]
    fn test_response_parser() {
        let parser = ResponseParser::new();
        
        // Test simple text parsing
        let content = parser.parse_response("Simple text message").unwrap();
        assert_eq!(content.len(), 1);
        if let MessageContent::Text(text) = &content[0] {
            assert_eq!(text, "Simple text message");
        }
        
        // Test command detection
        let response_with_command = "Here's a command:\n```bash\nls -la\n```\nThat should work.";
        assert!(parser.contains_commands(response_with_command));
        
        let commands = parser.extract_commands(response_with_command);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].trim(), "ls -la");
    }

    #[test]
    fn test_message_formatter() {
        let formatter = MessageFormatter::new();
        
        let message = Message::new(MessageRole::User, MessageContent::Text("Test message".to_string()));
        let formatted = formatter.format_message(&message);
        
        assert!(!formatted.is_empty());
        assert!(formatted[0].contains("👤 You:"));
    }

    #[test]
    fn test_conversation_metadata() {
        let (mut chat_manager, _temp_dir) = create_test_chat_manager();
        
        chat_manager.create_conversation();
        
        let conversation = chat_manager.get_current_conversation().unwrap();
        assert_eq!(conversation.conversation_metadata.message_count, 0);
        
        chat_manager.add_user_message("Test").unwrap();
        
        let conversation = chat_manager.get_current_conversation().unwrap();
        assert_eq!(conversation.conversation_metadata.message_count, 1);
        
        let (user_count, agent_count, system_count) = conversation.get_message_counts();
        assert_eq!(user_count, 1);
        assert_eq!(agent_count, 0);
        assert_eq!(system_count, 0);
    }

    #[test]
    fn test_project_type_detection() {
        let mut workspace_context = WorkspaceContext::new(None);
        
        // Test Rust project detection
        workspace_context.update_file_tree(vec!["Cargo.toml".to_string(), "src/main.rs".to_string()]);
        assert_eq!(workspace_context.project_type, Some(ProjectType::Rust));
        
        // Test JavaScript project detection
        workspace_context.update_file_tree(vec!["package.json".to_string(), "index.js".to_string()]);
        assert_eq!(workspace_context.project_type, Some(ProjectType::JavaScript));
        
        // Test Python project detection
        workspace_context.update_file_tree(vec!["pyproject.toml".to_string(), "main.py".to_string()]);
        assert_eq!(workspace_context.project_type, Some(ProjectType::Python));
        
        // Test Go project detection
        workspace_context.update_file_tree(vec!["go.mod".to_string(), "main.go".to_string()]);
        assert_eq!(workspace_context.project_type, Some(ProjectType::Go));
        
        // Test unknown project
        workspace_context.update_file_tree(vec!["README.md".to_string()]);
        assert_eq!(workspace_context.project_type, Some(ProjectType::Unknown));
    }
}