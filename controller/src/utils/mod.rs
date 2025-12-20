pub mod neovim_api;
pub mod error_handling;

pub use neovim_api::NeovimApiWrapper;
pub use error_handling::{PluginError, PluginResult};