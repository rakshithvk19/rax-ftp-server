//! Module `data_channel`
//!
//! Manages accepting and handling data connections for file transfers
//! in an FTP-like server, coordinating client-specific TCP listeners
//! used for data transfer commands like RETR, STOR, and LIST.

use log::{error, info, warn};
use std::io::ErrorKind;
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
/// - Tries accepting a connection using exponential backoff.
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
