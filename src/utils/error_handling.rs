use thiserror::Error;
use neovim_lib::CallError;

/// Plugin-specific error types
#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Neovim API error: {0}")]
    NeovimApi(#[from] std::io::Error),

    #[error("Neovim call error: {0}")]
    NeovimCall(#[from] CallError),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("UUID parsing error: {0}")]
    UuidParsing(#[from] uuid::Error),

    #[error("File system error: {0}")]
    FileSystem(String),

    #[error("Command execution error: {0}")]
    CommandExecution(String),

    #[error("Window management error: {0}")]
    WindowManagement(String),

    #[error("Spec workflow error: {0}")]
    SpecWorkflow(String),

    #[error("Agent communication error: {0}")]
    AgentCommunication(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Result type alias for plugin operations
pub type PluginResult<T> = Result<T, PluginError>;

impl PluginError {
    /// Create a configuration error
    pub fn config(msg: &str) -> Self {
        PluginError::Configuration(msg.to_string())
    }

    /// Create a file system error
    pub fn filesystem(msg: &str) -> Self {
        PluginError::FileSystem(msg.to_string())
    }

    /// Create a command execution error
    pub fn command(msg: &str) -> Self {
        PluginError::CommandExecution(msg.to_string())
    }

    /// Create a window management error
    pub fn window(msg: &str) -> Self {
        PluginError::WindowManagement(msg.to_string())
    }

    /// Create a spec workflow error
    pub fn spec(msg: &str) -> Self {
        PluginError::SpecWorkflow(msg.to_string())
    }

    /// Create an agent communication error
    pub fn agent(msg: &str) -> Self {
        PluginError::AgentCommunication(msg.to_string())
    }

    /// Create an unknown error
    pub fn unknown(msg: &str) -> Self {
        PluginError::Unknown(msg.to_string())
    }

    /// Get user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            PluginError::NeovimApi(e) => format!("Neovim communication failed: {}", e),
            PluginError::NeovimCall(e) => format!("Neovim call failed: {}", e),
            PluginError::Configuration(msg) => format!("Configuration issue: {}", msg),
            PluginError::Serialization(e) => format!("Data processing error: {}", e),
            PluginError::UuidParsing(e) => format!("Invalid identifier: {}", e),
            PluginError::FileSystem(msg) => format!("File operation failed: {}", msg),
            PluginError::CommandExecution(msg) => format!("Command failed: {}", msg),
            PluginError::WindowManagement(msg) => format!("Window error: {}", msg),
            PluginError::SpecWorkflow(msg) => format!("Spec workflow error: {}", msg),
            PluginError::AgentCommunication(msg) => format!("Agent communication error: {}", msg),
            PluginError::Unknown(msg) => format!("Unexpected error: {}", msg),
        }
    }

    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            PluginError::NeovimApi(_) => false,
            PluginError::NeovimCall(_) => false,
            PluginError::Configuration(_) => true,
            PluginError::Serialization(_) => true,
            PluginError::UuidParsing(_) => true,
            PluginError::FileSystem(_) => true,
            PluginError::CommandExecution(_) => true,
            PluginError::WindowManagement(_) => true,
            PluginError::SpecWorkflow(_) => true,
            PluginError::AgentCommunication(_) => true,
            PluginError::Unknown(_) => false,
        }
    }
}