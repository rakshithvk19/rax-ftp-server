//! Transfer module
//!
//! Handles data channel management and file transfers.

pub mod channel_registry;
pub mod data_channel;
pub mod file_ops;
pub mod modes;
mod operations;
mod results;

pub use channel_registry::{ChannelEntry, ChannelRegistry};
pub use data_channel::setup_data_stream;
pub use file_ops::{handle_file_download, handle_file_upload};
pub use operations::{cleanup_data_channel, setup_active_mode, setup_passive_mode};
pub use results::{ActiveModeResult, PassiveModeResult};
