//! Module `client_handler`
//!
//! Handles individual FTP client connections, reading commands from the control stream,
//! processing commands, and sending appropriate responses. Manages client state and
//! integrates with shared registries for clients and data channels.

use log::{error, info};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};

use crate::channel_registry;
use crate::client::Client;
use crate::command::{CommandData, CommandStatus, parse_command};
use crate::handlers::handle_command;

const MAX_COMMAND_LENGTH: usize = 512;

/// Processes commands from a single FTP client over the control connection.
///
/// - Sends initial FTP greeting on connection.
/// - Reads incoming data, buffering until full commands (ending with `\r\n`) are received.
/// - Parses and handles FTP commands via `handle_command`.
/// - Sends responses back to the client over the control connection.
/// - Manages client state and command lifecycle.
/// - Removes client from registry on disconnect or quit.
///
/// # Arguments
///
/// * `cmd_stream` - The TCP stream representing the control connection with the client.
/// * `clients` - Shared registry of connected clients, synchronized via mutex.
/// * `client_addr` - Socket address of the connected client.
/// * `channel_registry` - Shared registry for data channel listeners and streams.
pub fn handle_client(
    mut cmd_stream: TcpStream,
    clients: Arc<Mutex<HashMap<SocketAddr, Client>>>,
    client_addr: SocketAddr,
    channel_registry: Arc<Mutex<channel_registry::ChannelRegistry>>,
) {
    if let Err(e) = cmd_stream.write_all(b"220 Welcome to the Rax FTP server\r\n") {
        error!("Failed to send welcome: {}", e);
        return;
    }

    let mut buffer = [0; 1024];
    let mut command_buffer = String::new();

    loop {
        match cmd_stream.read(&mut buffer) {
            Ok(0) => {
                info!("Connection closed by client {}", client_addr);
                break;
            }
            Ok(n) => {
                let chunk = String::from_utf8_lossy(&buffer[..n]);

                if command_buffer.len() + chunk.len() > MAX_COMMAND_LENGTH {
                    let _ = cmd_stream.write_all(b"500 Command too long\r\n");
                    command_buffer.clear();
                    continue;
                }

                command_buffer.push_str(&chunk);

                while let Some(pos) = command_buffer.find("\r\n") {
                    let raw_command = command_buffer[..pos].trim().to_string();
                    command_buffer.drain(..pos + 2);

                    let command = parse_command(&raw_command);
                    info!("Received from {}: {:?}", client_addr, &command);

                    let mut clients_guard = clients.lock().unwrap();
                    let mut channel_registry_guard = channel_registry.lock().unwrap();

                    match clients_guard.get_mut(&client_addr) {
                        Some(client) => {
                            let result =
                                handle_command(client, &command, &mut channel_registry_guard);

                            let final_result = match result.status {
                                CommandStatus::CloseConnection => {
                                    if let Some(msg) = result.message.as_ref() {
                                        let _ = cmd_stream.write_all(msg.as_bytes());
                                    }
                                    info!("Client {} requested to quit", client_addr);
                                    let _ = cmd_stream.shutdown(Shutdown::Both);
                                    break;
                                }
                                CommandStatus::Failure(_) => result,
                                CommandStatus::Success => match &result.data {
                                    Some(CommandData::DirectoryListing(listing)) => {
                                        let listing_output = listing.join("\r\n") + "\r\n";
                                        if let Err(e) =
                                            cmd_stream.write_all(listing_output.as_bytes())
                                        {
                                            error!(
                                                "Failed to send directory listing to client {}: {}",
                                                client_addr, e
                                            );
                                        }
                                        result
                                    }
                                    _ => result,
                                },
                            };

                            if let Some(message) = final_result.message {
                                let _ = cmd_stream.write_all(message.as_bytes());
                            }
                        }
                        None => {
                            error!("Client {} not found in clients map", client_addr);
                            let _ = cmd_stream.write_all(b"421 Client session not found\r\n");
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to read from stream: {}", e);
                break;
            }
        }
    }

    let mut clients_guard = clients.lock().unwrap();
    clients_guard.remove(&client_addr);
    info!("Client {} disconnected", client_addr);
}
