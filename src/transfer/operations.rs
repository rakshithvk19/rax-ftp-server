//! Transfer operations
//!
//! Handles data channel setup and management for FTP passive and active modes.

use std::net::{SocketAddr, TcpListener};
use std::str::FromStr;
use log::{error, info};

use crate::error::TransferError;
use crate::transfer::results::{PassiveModeResult, ActiveModeResult};
use crate::transfer::{ChannelRegistry, ChannelEntry};

/// Sets up passive mode for data transfer
pub fn setup_passive_mode(
    channel_registry: &mut ChannelRegistry,
    client_addr: SocketAddr,
) -> Result<PassiveModeResult, TransferError> {
    // Find next available socket for data connection
    let data_socket = channel_registry.next_available_socket()
        .ok_or(TransferError::NoAvailablePort)?;
    
    // Bind the listener
    let listener = TcpListener::bind(data_socket)
        .map_err(|e| TransferError::PortBindingFailed(data_socket, e))?;
    
    // Set listener to non-blocking to avoid blocking main thread
    listener.set_nonblocking(true)
        .map_err(TransferError::ListenerConfigurationFailed)?;
    
    // Clone listener for registry
    let listener_clone = listener.try_clone()
        .map_err(|e| TransferError::ListenerConfigurationFailed(e))?;
    
    // Create new channel entry for data connection
    let mut entry = ChannelEntry::default();
    entry.set_data_socket(Some(data_socket));
    entry.set_data_stream(None);
    entry.set_listener(Some(listener_clone));
    
    // Insert into registry
    channel_registry.insert(client_addr, entry);
    
    info!(
        "Client {} bound to data socket {} in PASV mode",
        client_addr, data_socket
    );
    
    Ok(PassiveModeResult {
        data_socket,
        listener,
    })
}

/// Sets up active mode for data transfer (PORT command)
pub fn setup_active_mode(
    channel_registry: &mut ChannelRegistry,
    client_addr: SocketAddr,
    port_command_addr: &str,
) -> Result<ActiveModeResult, TransferError> {
    // Parse the address string to SocketAddr
    let parsed_addr = SocketAddr::from_str(port_command_addr)
        .map_err(|_| TransferError::InvalidPortCommand("Invalid address format".into()))?;
    
    // Validate IP matches client (for security)
    if parsed_addr.ip() != client_addr.ip() {
        return Err(TransferError::IpMismatch {
            expected: client_addr.ip().to_string(),
            provided: parsed_addr.ip().to_string(),
        });
    }
    
    // Validate port range
    let port = parsed_addr.port();
    if port < 1024 {
        return Err(TransferError::InvalidPortRange(port));
    }
    
    // Bind TcpListener on client-specified address
    let listener = TcpListener::bind(parsed_addr)
        .map_err(|e| TransferError::PortBindingFailed(parsed_addr, e))?;
    
    listener.set_nonblocking(true)
        .map_err(TransferError::ListenerConfigurationFailed)?;
    
    // Clone listener for registry
    let listener_clone = listener.try_clone()
        .map_err(|e| TransferError::ListenerConfigurationFailed(e))?;
    
    let mut entry = ChannelEntry::default();
    entry.set_data_socket(Some(parsed_addr));
    entry.set_data_stream(None);
    entry.set_listener(Some(listener_clone));
    
    channel_registry.insert(client_addr, entry);
    
    info!(
        "Client {} bound to data socket {} in PORT mode",
        client_addr, parsed_addr
    );
    
    Ok(ActiveModeResult {
        data_socket: parsed_addr,
        listener,
    })
}

/// Cleans up data channel resources for a client
pub fn cleanup_data_channel(
    channel_registry: &mut ChannelRegistry,
    client_addr: &SocketAddr,
) {
    if let Some(entry) = channel_registry.remove(client_addr) {
        // Drop the entry to ensure all resources are freed
        drop(entry);
        info!(
            "Cleaned up data channel for client {} - listener and resources freed",
            client_addr
        );
    }
}
