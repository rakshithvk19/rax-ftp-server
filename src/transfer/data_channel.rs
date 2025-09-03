//! Module `data_channel`
//!
//! Manages data connections for file transfers in FTP server.

use log::{error, info};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;
use std::io::Write;

use crate::client::Client;
use crate::error::TransferError;
use crate::transfer::ChannelRegistry;

/// Validates client authentication and data channel initialization
pub fn validate_client_and_data_channel(client: &Client) -> bool {
    client.is_logged_in() && client.is_data_channel_init()
}

/// Sets up a data connection for the given client
pub fn setup_data_stream(
    channel_registry: &mut ChannelRegistry,
    client_addr: &SocketAddr,
) -> Option<TcpStream> {
    let entry = channel_registry.get_mut(client_addr)?;
    
    // Check if this is active mode (has data_socket but no listener)
    if let Some(data_socket) = entry.data_socket() {
        if entry.listener().is_none() {
            // Active mode: Server connects to client
            info!("Active mode: Server connecting to client at {data_socket}");
            return connect_to_client(*data_socket);
        }
    }
    
    // Passive mode: Accept connection from client
    if let Some(listener) = entry.listener_mut() {
        info!("Passive mode: Accepting connection from client");
        return accept_from_client(listener);
    }
    
    error!("No data channel setup found for client {client_addr}");
    None
}

/// Sends directory listing over data connection
pub fn send_directory_listing(
    channel_registry: &mut ChannelRegistry,
    client_addr: &SocketAddr,
    listing: Vec<String>,
) -> Result<(), TransferError> {
    let mut data_stream = setup_data_stream(channel_registry, client_addr)
        .ok_or_else(|| TransferError::DataChannelSetupFailed("Failed to establish data connection".into()))?;
    
    let listing_data = listing.join("\r\n") + "\r\n";
    
    data_stream.write_all(listing_data.as_bytes())
        .map_err(TransferError::TransferFailed)?;
        
    data_stream.flush()
        .map_err(TransferError::TransferFailed)?;
    
    let _ = data_stream.shutdown(std::net::Shutdown::Both);
    
    info!("Directory listing sent successfully to client {client_addr}");
    Ok(())
}

/// Receives file upload over data connection
pub fn receive_file_upload(
    channel_registry: &mut ChannelRegistry,
    client_addr: &SocketAddr,
    final_filename: &str,
    temp_filename: &str,
) -> Result<(), TransferError> {
    let data_stream = setup_data_stream(channel_registry, client_addr)
        .ok_or_else(|| TransferError::DataChannelSetupFailed("Failed to establish data connection".into()))?;
    
    match crate::transfer::handle_file_upload(data_stream, final_filename, temp_filename) {
        Ok(_) => {
            info!("File upload completed successfully to {client_addr}");
            Ok(())
        }
        Err((_, msg)) => {
            error!("File upload failed for {client_addr}: {msg}");
            Err(TransferError::TransferFailed(
                std::io::Error::other(msg)
            ))
        }
    }
}

/// Active mode: Server connects to client
fn connect_to_client(data_socket: SocketAddr) -> Option<TcpStream> {
    const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);
    
    match TcpStream::connect_timeout(&data_socket, CONNECTION_TIMEOUT) {
        Ok(stream) => {
            info!("Connected to client at {data_socket}");
            Some(stream)
        }
        Err(e) => {
            error!("Failed to connect to client at {data_socket}: {e}");
            None
        }
    }
}

/// Passive mode: Accept connection from client
fn accept_from_client(listener: &mut std::net::TcpListener) -> Option<TcpStream> {
    // Set to blocking mode for accept
    if let Err(e) = listener.set_nonblocking(false) {
        error!("Failed to set listener to blocking mode: {e}");
        return None;
    }
    
    match listener.accept() {
        Ok((stream, peer_addr)) => {
            info!("Accepted connection from {peer_addr}");
            // Reset to non-blocking for next time
            let _ = listener.set_nonblocking(true);
            Some(stream)
        }
        Err(e) => {
            error!("Failed to accept connection: {e}");
            let _ = listener.set_nonblocking(true);
            None
        }
    }
}
