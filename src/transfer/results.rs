//! Transfer result types
//!
//! Defines result structures returned by transfer operations.

use std::net::{SocketAddr, TcpListener};

/// Result of setting up passive mode
#[derive(Debug)]
pub struct PassiveModeResult {
    pub data_socket: SocketAddr,
    pub listener: TcpListener,
}

/// Result of setting up active mode (PORT command)
#[derive(Debug)]
pub struct ActiveModeResult {
    pub data_socket: SocketAddr,
    pub listener: TcpListener,
}
