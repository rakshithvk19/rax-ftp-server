//! Server configuration
//!
//! Manages server configuration settings and validation.

use std::path::PathBuf;

/// Server configuration structure
pub struct ServerConfig {
    pub server_root: PathBuf,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server_root: PathBuf::from("./server_root"),
        }
    }
}

impl ServerConfig {
    /// Get the server root directory as a string
    pub fn server_root_str(&self) -> String {
        self.server_root.to_string_lossy().to_string()
    }
}
