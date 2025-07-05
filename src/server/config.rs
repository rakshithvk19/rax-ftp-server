//! Server configuration
//! 
//! Manages server configuration settings and validation.

/// Server configuration structure
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_clients: usize,
    pub timeout: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 2121,
            max_clients: 10,
            timeout: 30,
        }
    }
}
