pub mod chat_manager;
pub mod command_executor;
pub mod command_integration;
pub mod command_workflow;
pub mod file_operations;

pub use chat_manager::{
    ChatManager, Conversation, Message, MessageContent, MessageRole, MessageMetadata,
    SpecContext, SpecPhase, SpecUpdate, RequirementsDocument, DesignDocument, 
    TasksDocument, Task, CorrectnessProperty, PropertyType
};
pub use command_executor::{CommandExecutor, CommandBlock, ApprovalStatus, CommandOutput, RiskLevel};
pub use command_integration::{CommandIntegration, CommandApprovalEvent};
pub use command_workflow::{CommandWorkflow, WorkflowStatistics};
pub use file_operations::FileOperationsManager;