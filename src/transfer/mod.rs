//! Data transfer functionality
//!
//! Manages file transfers, data channels, and connection modes.

pub mod channel_registry;
pub mod data_channel;
pub mod file_ops;
pub mod modes;

pub use channel_registry::{ChannelEntry, ChannelRegistry};
pub use data_channel::setup_data_stream;
pub use file_ops::{handle_file_download, handle_file_upload};
