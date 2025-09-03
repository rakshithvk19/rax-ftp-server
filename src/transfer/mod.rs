//! Transfer module for FTP server
//!
//! Handles data channel management, file transfers, and connection operations
//! with support for persistent data connections.

pub mod channel_registry;
pub mod data_channel;
pub mod file_ops;
pub mod operations;

// Re-export key types and functions
pub use channel_registry::{ChannelEntry, ChannelRegistry};
pub use data_channel::{
    receive_file_upload, send_directory_listing, setup_data_stream,
    validate_client_and_data_channel,
};
pub use file_ops::{handle_file_download, handle_file_upload};
pub use operations::{
    cleanup_data_channel, cleanup_data_stream_only, setup_active_mode, setup_passive_mode,
};
