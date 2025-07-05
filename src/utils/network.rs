//! Network utilities
//! 
//! Provides network-related utility functions.

use std::net::{IpAddr, SocketAddr};

/// Parse a socket address string
pub fn parse_socket_addr(addr: &str) -> Result<SocketAddr, std::net::AddrParseError> {
    addr.parse()
}

/// Validate IP address
pub fn is_valid_ip(ip: &str) -> bool {
    ip.parse::<IpAddr>().is_ok()
}

/// Get local IP address (placeholder)
pub fn get_local_ip() -> String {
    "127.0.0.1".to_string()
}
