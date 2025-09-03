//! Transfer operations
//!
//! Handles data channel setup and management for FTP passive and active modes.
//! Updated to support persistent data connections.

use std::net::{SocketAddr, TcpListener};
use std::str::FromStr;
use log::{error, info};

use crate::error::TransferError;
use crate::transfer::{ChannelRegistry, ChannelEntry};

/// Sets up passive mode for data transfer with persistent connection support
pub fn setup_passive_mode(
    channel_registry: &mut ChannelRegistry,
    client_addr: SocketAddr,
) -> Result<SocketAddr, TransferError> {
    // Clean up any existing entry for this client (replacement behavior)
    if channel_registry.contains(&client_addr) {
        info!(
            "Replacing existing data channel for client {client_addr} with new PASV connection"
        );
        channel_registry.cleanup_all(&client_addr);
    }

    // Find next available socket for data connection
    let data_socket = channel_registry.next_available_socket()
        .ok_or(TransferError::NoAvailablePort)?;
    
    // Bind the listener
    let listener = TcpListener::bind(data_socket)
        .map_err(|e| TransferError::PortBindingFailed(data_socket, e))?;
    
    // Set listener to non-blocking to "stop listening" until needed
    listener.set_nonblocking(true)
        .map_err(TransferError::ListenerConfigurationFailed)?;
    
    // DEBUG: Verify listener was created and configured correctly
    match listener.local_addr() {
        Ok(addr) => info!("DEBUG: PASV listener successfully created on {addr} (non-blocking mode)"),
        Err(e) => error!("DEBUG: Failed to get PASV listener address: {e}"),
    }
    
    // Clone listener for registry
    let listener_clone = listener.try_clone()
        .map_err(TransferError::ListenerConfigurationFailed)?;
    
    // Create new channel entry for persistent data connection
    let mut entry = ChannelEntry::default();
    entry.set_data_socket(Some(data_socket));
    entry.set_data_stream(None);
    entry.set_listener(Some(listener_clone));
    entry.set_owner_ip(Some(client_addr.ip())); // Set ownership
    
    // Insert into registry
    channel_registry.insert(client_addr, entry);
    
    info!(
        "Client {client_addr} configured for passive mode - client will connect to server at {data_socket}"
    );
    
    Ok(data_socket)
}

/// Sets up active mode for data transfer (PORT command) with persistent connection support
pub fn setup_active_mode(
    channel_registry: &mut ChannelRegistry,
    client_addr: SocketAddr,
    port_command_addr: &str,
) -> Result<(), TransferError> {
    // Clean up any existing entry for this client (replacement behavior)
    if channel_registry.contains(&client_addr) {
        info!(
            "Replacing existing data channel for client {client_addr} with new PORT connection"
        );
        channel_registry.cleanup_all(&client_addr);
    }

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
    
    // ✅ CORRECT: In active mode, server stores client's address and connects to it later
    // The client is the one with the TcpListener, not the server!
    
    // Create new channel entry for persistent data connection
    let mut entry = ChannelEntry::default();
    entry.set_data_socket(Some(parsed_addr));  // Store client's data address
    entry.set_data_stream(None);
    entry.set_listener(None);  // No listener in active mode - server connects to client!
    entry.set_owner_ip(Some(client_addr.ip())); // Set ownership
    
    channel_registry.insert(client_addr, entry);
    
    info!(
        "Client {client_addr} configured for active mode - server will connect to client at {parsed_addr}"
    );
    
    Ok(())
}

/// Cleans up only the data stream for a client, keeping the persistent setup intact.
/// This is called after each successful transfer to maintain persistent connection info.
pub fn cleanup_data_stream_only(
    channel_registry: &mut ChannelRegistry,
    client_addr: &SocketAddr,
) {
    if let Some(entry) = channel_registry.get_mut(client_addr) {
        entry.cleanup_stream_only();
        info!(
            "Cleaned up data stream for client {client_addr} - persistent setup maintained"
        );
    }
}

/// Completely cleans up data channel resources for a client.
/// This is called when the client disconnects or on new PASV/PORT commands.
pub fn cleanup_data_channel(
    channel_registry: &mut ChannelRegistry,
    client_addr: &SocketAddr,
) {
    if let Some(mut entry) = channel_registry.remove(client_addr) {
        entry.cleanup_all();
        info!(
            "Completely cleaned up data channel for client {client_addr} - all resources freed"
        );
    }
}
