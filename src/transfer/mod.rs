//! Data transfer functionality
//! 
//! Manages file transfers, data channels, and connection modes.

pub mod data_channel;
pub mod channel_registry;
pub mod file_ops;
pub mod modes;

pub use data_channel::setup_data_stream;
pub use channel_registry::{ChannelRegistry, ChannelEntry};
pub use file_ops::{handle_file_upload, handle_file_download};
