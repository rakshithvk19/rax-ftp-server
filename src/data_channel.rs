// data_channel.rs
// This file manages accepting and handling data connections for file transfers in an FTP-like server
// coordinating client-specific TCP listeners.

use log::{error, info};
use std::net::{SocketAddr, TcpStream};
use std::thread;
use std::time::Duration;

use crate::channel_registry::ChannelRegistry;

/// Accepts a new connection from the data listener for transfers (RETR, STOR, LIST).
pub fn setup_data_stream(
    channel_registry: &mut ChannelRegistry,
    client_addr: &SocketAddr,
) -> Option<TcpStream> {
    const ACCEPT_ATTEMPTS: u32 = 50;
    const ACCEPT_SLEEP_MS: u64 = 100;
    const TIMEOUT_MSG: &str = "Timeout waiting for data connection";

    // Try to take the listener out of the channel registry for this client address
    let listener = {
        // Get mutable reference to ChannelEntry
        if let Some(entry) = channel_registry.get_mut(client_addr) {
            entry.take_listener()
        } else {
            None
        }
    };

    if let Some(listener) = listener {
        // Set listener non-blocking so we can poll accept without blocking
        if let Err(e) = listener.set_nonblocking(true) {
            error!("Failed to set data listener to non-blocking mode: {}", e);
            // Put listener back into registry before returning
            if let Some(entry) = channel_registry.get_mut(client_addr) {
                entry.set_listener(Some(listener));
            }
            return None;
        }

        for _ in 0..ACCEPT_ATTEMPTS {
            match listener.accept() {
                Ok((stream, addr)) => {
                    info!(
                        "Data connection accepted from {} for client {}",
                        addr, client_addr
                    );

                    // Set stream back to blocking for normal IO
                    if let Err(e) = stream.set_nonblocking(false) {
                        error!("Failed to set data stream to blocking mode: {}", e);
                    }

                    return Some(stream);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No incoming connection yet, wait a bit and retry
                    thread::sleep(Duration::from_millis(ACCEPT_SLEEP_MS));
                }
                Err(e) => {
                    error!("Failed to accept data connection: {}", e);
                    break;
                }
            }
        }

        // Timeout - put listener back into registry so it can be reused
        error!("{}: {}", TIMEOUT_MSG, client_addr);
        if let Some(entry) = channel_registry.get_mut(client_addr) {
            entry.set_listener(Some(listener));
        }
    } else {
        error!("No data listener found for client {}", client_addr);
    }

    None
}
