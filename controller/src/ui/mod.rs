pub mod window_manager;
pub mod layout;
pub mod visual_blocks;
pub mod command_approval_ui;

pub use window_manager::WindowManager;
pub use layout::{LayoutMode, WindowDimensions, WindowState, WindowConfig, Position, Size, WindowLayoutConfig};
pub use visual_blocks::{OperationBlock, VisualBlockRenderer, VisualBlockManager, OperationStatus, BlockState, BlockType};
pub use command_approval_ui::CommandApprovalUI;