//! FTP Transfer modes
//!
//! Handles active and passive mode implementations.

/// FTP transfer modes
#[derive(Debug, Clone)]
pub enum TransferMode {
    Active,
    Passive,
}

/// Active mode configuration
pub struct ActiveMode {
    pub client_ip: String,
    pub client_port: u16,
}

/// Passive mode configuration  
pub struct PassiveMode {
    pub server_ip: String,
    pub server_port: u16,
}
