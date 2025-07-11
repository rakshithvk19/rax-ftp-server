//! Server configuration
//!
//! Manages server configuration settings and validation.

use std::path::PathBuf;

/// Server configuration structure
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_clients: usize,
    pub timeout: u64,
    pub server_root: PathBuf,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 2121,
            max_clients: 10,
            timeout: 30,
            server_root: PathBuf::from("./server_root"),
        }
    }
}

impl ServerConfig {
    /// Get the server root directory as a string
    pub fn server_root_str(&self) -> String {
        self.server_root.to_string_lossy().to_string()
    }

    /// Get the absolute path of the server root
    pub fn get_absolute_server_root(&self) -> std::io::Result<PathBuf> {
        self.server_root.canonicalize()
    }
}
