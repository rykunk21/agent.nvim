use thiserror::Error;

#[derive(Debug, Error)]
pub enum ContainerError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Communication error: {0}")]
    CommunicationError(String),

    #[error("LLM error: {0}")]
    LlmError(String),

    #[error("MCP error: {0}")]
    McpError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

pub type ContainerResult<T> = Result<T, ContainerError>;
