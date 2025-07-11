//! Module `data_channel`
//!
//! Manages accepting and handling data connections for file transfers
//! in an FTP-like server, coordinating client-specific TCP listeners
//! used for data transfer commands like RETR, STOR, and LIST.
//! Updated to support persistent data connections.

use log::{debug, error, info, warn};
use std::io::ErrorKind;
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::thread;
use std::time::Duration;

use crate::transfer::ChannelRegistry;

/// Attempts to get or establish a data connection for the given client.
/// In persistent mode, it will reuse existing listener setup when available.
///
/// # Arguments
///
/// * `channel_registry` - Mutable reference to the global registry
/// * `client_addr` - The client's socket address
///
/// # Returns
///
/// * `Some(TcpStream)` - A ready-to-use data connection stream
/// * `None` - If the connection could not be established
///
/// # Behavior
///
/// - First checks if there's a persistent setup available for reuse
/// - Validates that the connecting client is the owner of the persistent setup
/// - If reusable setup exists, uses it; otherwise creates a new connection
/// - After each transfer, the stream is closed but the listener setup is preserved
pub fn setup_data_stream(
    channel_registry: &mut ChannelRegistry,
    client_addr: &SocketAddr,
) -> Option<TcpStream> {
    // Check if we have a persistent setup for this client
    if channel_registry.has_persistent_setup(client_addr) {
        info!(
            "Reusing persistent data channel setup for client {}",
            client_addr
        );
        return setup_data_stream_with_persistent_listener(channel_registry, client_addr);
    }

    // No persistent setup available, use the original logic
    debug!(
        "No persistent setup found for client {}, using original setup",
        client_addr
    );
    setup_data_stream_original(channel_registry, client_addr)
}

/// Sets up a data stream using an existing persistent listener setup.
/// Validates that the connecting client is the owner of the persistent setup.
fn setup_data_stream_with_persistent_listener(
    channel_registry: &mut ChannelRegistry,
    client_addr: &SocketAddr,
) -> Option<TcpStream> {
    const MAX_ATTEMPTS: u32 = 10;
    const INITIAL_SLEEP_MS: u64 = 100;
    const TIMEOUT_MSG: &str = "Timeout waiting for persistent data connection";

    let entry = channel_registry.get_mut(client_addr)?;
    
    // Get the listener from the entry (keep it in the entry for persistence)
    let listener = entry.listener_mut()?;
    
    // Set listener to non-blocking mode for polling
    if let Err(e) = listener.set_nonblocking(true) {
        error!("Failed to set persistent listener to non-blocking mode: {}", e);
        return None;
    }

    let mut attempt = 0;
    let mut delay = INITIAL_SLEEP_MS;
    let client_ip = client_addr.ip();

    while attempt < MAX_ATTEMPTS {
        match listener.accept() {
            Ok((stream, peer_addr)) => {
                // Validate that the connecting client is the owner
                if !entry.is_client_allowed(peer_addr.ip()) {
                    warn!(
                        "Rejected connection from {} for client {}'s persistent channel",
                        peer_addr, client_addr
                    );
                    // Close the unauthorized connection
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                    continue;
                }

                info!(
                    "Persistent data connection accepted from {} for client {}",
                    peer_addr, client_addr
                );

                // Set stream back to blocking mode for normal I/O
                if let Err(e) = stream.set_nonblocking(false) {
                    warn!("Failed to set data stream to blocking mode: {}", e);
                }

                // Set the listener back to non-blocking mode to "stop listening"
                if let Err(e) = listener.set_nonblocking(true) {
                    warn!("Failed to set listener back to non-blocking mode: {}", e);
                }

                return Some(stream);
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(delay));
                delay *= 2;
                attempt += 1;
            }
            Err(e) => {
                error!("Fatal error accepting persistent data connection: {}", e);
                break;
            }
        }
    }

    error!(
        "{}: {} after {} attempts",
        TIMEOUT_MSG, client_addr, attempt
    );
    None
}

/// Original data stream setup logic for when no persistent setup exists.
/// This is used for the initial connection or when persistent setup is unavailable.
fn setup_data_stream_original(
    channel_registry: &mut ChannelRegistry,
    client_addr: &SocketAddr,
) -> Option<TcpStream> {
    const MAX_ATTEMPTS: u32 = 10;
    const INITIAL_SLEEP_MS: u64 = 100;
    const TIMEOUT_MSG: &str = "Timeout waiting for data connection";

    let listener = {
        if let Some(entry) = channel_registry.get_mut(client_addr) {
            entry.take_listener()
        } else {
            None
        }
    };

    if let Some(listener) = listener {
        if let Err(e) = listener.set_nonblocking(true) {
            error!("Failed to set data listener to non-blocking mode: {}", e);
            if let Some(entry) = channel_registry.get_mut(client_addr) {
                entry.set_listener(Some(listener));
            }
            return None;
        }

        let mut attempt = 0;
        let mut delay = INITIAL_SLEEP_MS;

        while attempt < MAX_ATTEMPTS {
            match listener.accept() {
                Ok((stream, addr)) => {
                    info!(
                        "Data connection accepted from {} for client {}",
                        addr, client_addr
                    );
                    if let Err(e) = stream.set_nonblocking(false) {
                        warn!("Failed to set data stream to blocking mode: {}", e);
                    }
                    
                    // Put the listener back in the registry for persistence
                    if let Some(entry) = channel_registry.get_mut(client_addr) {
                        entry.set_listener(Some(listener));
                        // Set the listener back to non-blocking mode to "stop listening"
                        if let Some(l) = entry.listener_mut() {
                            let _ = l.set_nonblocking(true);
                        }
                    }
                    
                    return Some(stream);
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(delay));
                    delay *= 2;
                    attempt += 1;
                }
                Err(e) => {
                    error!("Fatal error accepting data connection: {}", e);
                    break;
                }
            }
        }

        error!(
            "{}: {} after {} attempts",
            TIMEOUT_MSG, client_addr, attempt
        );
        if let Some(entry) = channel_registry.get_mut(client_addr) {
            entry.set_listener(Some(listener));
        }
    } else {
        error!("No data listener found for client {}", client_addr);
    }

    None
}

/// Validates that a potential data connection is from the expected client.
/// This is used as an additional security check for persistent connections.
pub fn validate_data_connection_owner(
    channel_registry: &ChannelRegistry,
    client_addr: &SocketAddr,
    connecting_ip: IpAddr,
) -> bool {
    if let Some(entry) = channel_registry.get(client_addr) {
        entry.is_client_allowed(connecting_ip)
    } else {
        false
    }
}
