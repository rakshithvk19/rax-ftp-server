//! Module `data_channel`
//!
//! Manages accepting and handling data connections for file transfers
//! in an FTP-like server, coordinating client-specific TCP listeners
//! used for data transfer commands like RETR, STOR, and LIST.

use log::{error, info};
use std::net::{SocketAddr, TcpStream};
use std::thread;
use std::time::Duration;

use crate::channel_registry::ChannelRegistry;

/// Attempts to accept a new data connection from the TCP listener
/// associated with a particular client address.
///
/// This function polls the client's data listener socket repeatedly
/// in non-blocking mode, waiting for an incoming connection on the data channel.
/// It returns a `TcpStream` representing the accepted data connection,
/// or `None` if no connection was accepted within the allowed time.
///
/// # Arguments
///
/// * `channel_registry` - Mutable reference to the global registry
///   that manages per-client data channel listeners.
/// * `client_addr` - The client's socket address for which the data
///   connection is expected.
///
/// # Returns
///
/// * `Some(TcpStream)` - The accepted data connection stream ready for I/O.
/// * `None` - If the connection could not be accepted or on error.
///
/// # Behavior
///
/// - Temporarily takes ownership of the listener from the registry.
/// - Sets the listener to non-blocking mode to allow polling.
/// - Tries accepting a connection up to a configured number of attempts,
///   sleeping briefly between attempts.
/// - On successful accept, sets the stream back to blocking mode for normal IO.
/// - On failure or timeout, logs errors and reinserts the listener back into the registry.
///
/// # Errors
///
/// Logs any errors related to listener non-blocking setup,
/// accept failures, or stream mode adjustments.
///
pub fn setup_data_stream(
    channel_registry: &mut ChannelRegistry,
    client_addr: &SocketAddr,
) -> Option<TcpStream> {
    // Constants controlling retry attempts and wait duration between accepts.
    const ACCEPT_ATTEMPTS: u32 = 50;
    const ACCEPT_SLEEP_MS: u64 = 100;
    const TIMEOUT_MSG: &str = "Timeout waiting for data connection";

    // Attempt to extract the TCP listener assigned for this client's data channel.
    let listener = {
        if let Some(entry) = channel_registry.get_mut(client_addr) {
            entry.take_listener()
        } else {
            None
        }
    };

    if let Some(listener) = listener {
        // Set listener to non-blocking mode to avoid blocking indefinitely on accept.
        if let Err(e) = listener.set_nonblocking(true) {
            error!("Failed to set data listener to non-blocking mode: {}", e);

            // Put the listener back so it remains usable by future operations.
            if let Some(entry) = channel_registry.get_mut(client_addr) {
                entry.set_listener(Some(listener));
            }
            return None;
        }

        // Poll listener repeatedly to accept an incoming connection.
        for _ in 0..ACCEPT_ATTEMPTS {
            match listener.accept() {
                Ok((stream, addr)) => {
                    info!(
                        "Data connection accepted from {} for client {}",
                        addr, client_addr
                    );

                    // Set stream back to blocking mode for standard read/write behavior.
                    if let Err(e) = stream.set_nonblocking(false) {
                        error!("Failed to set data stream to blocking mode: {}", e);
                    }

                    return Some(stream);
                }
                // No incoming connection yet; sleep and retry.
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(ACCEPT_SLEEP_MS));
                }
                // Accept failed with unexpected error.
                Err(e) => {
                    error!("Failed to accept data connection: {}", e);
                    break;
                }
            }
        }

        // Timeout reached without connection; log and restore listener.
        error!("{}: {}", TIMEOUT_MSG, client_addr);
        if let Some(entry) = channel_registry.get_mut(client_addr) {
            entry.set_listener(Some(listener));
        }
    } else {
        // No listener found for the client address; log error.
        error!("No data listener found for client {}", client_addr);
    }

    None
}
