//! Logging middleware
//!
//! Provides request logging functionality.

use log::info;

/// Log a client connection
pub fn log_connection(client_addr: &str) {
    info!("Client connected: {}", client_addr);
}

/// Log a client command
pub fn log_command(client_addr: &str, command: &str) {
    info!("Client {} executed: {}", client_addr, command);
}
