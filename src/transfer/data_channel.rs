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
/// Handles both active mode (server connects to client) and passive mode (client connects to server).
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
/// - Detects active vs passive mode based on presence of listener
/// - Active mode: Server connects to client's data socket
/// - Passive mode: Server waits for client to connect to server's listener
/// - After each transfer, the stream is closed but the setup is preserved
pub fn setup_data_stream(
    channel_registry: &mut ChannelRegistry,
    client_addr: &SocketAddr,
) -> Option<TcpStream> {
    // Check if we have a persistent setup for this client
    if let Some(entry) = channel_registry.get(client_addr) {
        // Check if this is active mode (has data_socket but no listener)
        if let Some(data_socket) = entry.data_socket() {
            if entry.listener().is_none() {
                // Active mode: Server connects to client
                info!(
                    "Active mode detected for client {} - server connecting to client at {}",
                    client_addr, data_socket
                );
                return setup_data_stream_active_mode(channel_registry, client_addr, *data_socket);
            } else {
                // Passive mode: Use existing listener logic
                info!(
                    "Passive mode detected for client {} - waiting for client to connect",
                    client_addr
                );
                return setup_data_stream_with_persistent_listener(channel_registry, client_addr);
            }
        }
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
    let entry = channel_registry.get_mut(client_addr)?;

    // Get the owner IP before taking mutable borrow of listener
    let owner_ip = entry.owner_ip();

    // Get the listener from the entry
    let listener = entry.listener_mut()?;

    // DEBUG: Verify listener is actually bound to expected port
    match listener.local_addr() {
        Ok(local_addr) => {
            info!(
                "DEBUG: Listener actually bound to: {} for client {}",
                local_addr, client_addr
            );
        }
        Err(e) => {
            error!(
                "DEBUG: Failed to get listener local address for client {}: {}",
                client_addr, e
            );
            return None;
        }
    }

    info!(
        "DEBUG: About to call listener.accept() for client {} at {:?}",
        client_addr,
        std::time::SystemTime::now()
    );

    // Listener should already be in blocking mode from LIST handler
    // Accept connection immediately
    match listener.accept() {
        Ok((stream, peer_addr)) => {
            info!(
                "DEBUG: Accept successful from {} for client {} at {:?}",
                peer_addr,
                client_addr,
                std::time::SystemTime::now()
            );
            // Validate that the connecting client is the owner
            let is_allowed = match owner_ip {
                Some(owner) => owner == peer_addr.ip(),
                None => true, // No owner set, allow any client
            };

            if !is_allowed {
                warn!(
                    "Rejected connection from {} for client {}'s persistent channel",
                    peer_addr, client_addr
                );
                // Close the unauthorized connection
                let _ = stream.shutdown(std::net::Shutdown::Both);
                return None;
            }

            info!(
                "Persistent data connection accepted from {} for client {}",
                peer_addr, client_addr
            );

            // Ensure stream is in blocking mode for normal I/O
            if let Err(e) = stream.set_nonblocking(false) {
                warn!("Failed to set data stream to blocking mode: {}", e);
            }

            return Some(stream);
        }
        Err(e) => {
            error!(
                "DEBUG: Failed to accept persistent data connection for client {}: {} at {:?}",
                client_addr,
                e,
                std::time::SystemTime::now()
            );
            error!("DEBUG: Accept error kind: {:?}", e.kind());
            return None;
        }
    }
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

/// Sets up data stream for active mode by connecting to the client.
/// In active mode, the server initiates the connection to the client's data socket.
fn setup_data_stream_active_mode(
    channel_registry: &mut ChannelRegistry,
    client_addr: &SocketAddr,
    data_socket: SocketAddr,
) -> Option<TcpStream> {
    const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);

    info!(
        "Active mode: Server connecting to client {} at data socket {}",
        client_addr, data_socket
    );

    match TcpStream::connect_timeout(&data_socket, CONNECTION_TIMEOUT) {
        Ok(stream) => {
            info!(
                "Successfully connected to client {} at data socket {} in active mode",
                client_addr, data_socket
            );

            // Set stream to blocking mode for normal I/O
            if let Err(e) = stream.set_nonblocking(false) {
                warn!("Failed to set data stream to blocking mode: {}", e);
            }

            Some(stream)
        }
        Err(e) => {
            error!(
                "Failed to connect to client {} at data socket {} in active mode: {}",
                client_addr, data_socket, e
            );
            None
        }
    }
}
