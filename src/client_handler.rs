//! Module `client_handler`
//!
//! Handles individual FTP client connections, reading commands from the control stream,
//! processing commands, and sending appropriate responses. Manages client state and
//! integrates with shared registries for clients and data channels.

use log::{error, info};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};

use crate::channel_registry;
use crate::client::Client;
use crate::command::{CommandData, CommandStatus, parse_command};
use crate::handlers::handle_command;

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
    // Send FTP service ready message on client connect
    if let Err(e) = cmd_stream.write_all(b"220 Welcome to the Rax FTP server\r\n") {
        error!("Failed to send welcome: {}", e);
        return;
    }

    let mut buffer = [0; 1024]; // Buffer for reading TCP stream data
    let mut command_buffer = String::new(); // Buffer to accumulate partial commands

    loop {
        match cmd_stream.read(&mut buffer) {
            Ok(0) => {
                // Connection closed gracefully by client
                info!("Connection closed by client {}", client_addr);
                break;
            }
            Ok(n) => {
                // Append received bytes as UTF-8 string to command buffer
                command_buffer.push_str(&String::from_utf8_lossy(&buffer[..n]));

                // Check if we have received a full FTP command ending with \r\n
                if command_buffer.ends_with("\r\n") {
                    // Parse raw command string into Command enum
                    let command = parse_command(&command_buffer);
                    info!("Received from {}: {:?}", client_addr, &command);

                    // Clear command buffer for next command
                    command_buffer.clear();

                    // Lock clients and channel registries for synchronized access
                    let mut clients_guard = clients.lock().unwrap();
                    let mut channel_registry_guard = channel_registry.lock().unwrap();

                    // Retrieve the client struct from registry
                    match clients_guard.get_mut(&client_addr) {
                        Some(client) => {
                            // Process command, potentially modifying client and registry state
                            let result =
                                handle_command(client, &command, &mut channel_registry_guard);

                            // Determine follow-up action based on command result status
                            let final_result = match result.status {
                                CommandStatus::CloseConnection => {
                                    // Client requested to close connection (QUIT command)
                                    if let Some(msg) = result.message.as_ref() {
                                        let _ = cmd_stream.write_all(msg.as_bytes());
                                    }
                                    info!("Client {} requested to quit", client_addr);
                                    let _ = cmd_stream.shutdown(std::net::Shutdown::Both);
                                    break;
                                }
                                CommandStatus::Failure(_) => {
                                    // Command failed, message already included
                                    result
                                }
                                CommandStatus::Success => {
                                    // Success - check for additional data to send
                                    match &result.data {
                                        Some(CommandData::DirectoryListing(listing)) => {
                                            // Format directory listing lines
                                            let mut listing_output = String::new();
                                            for entry in listing {
                                                listing_output.push_str(&format!("{}\r\n", entry));
                                            }

                                            // Send directory listing on control stream (note: FTP normally uses data connection)
                                            if let Err(e) =
                                                cmd_stream.write_all(listing_output.as_bytes())
                                            {
                                                error!(
                                                    "Failed to send directory listing to client {}: {}",
                                                    client_addr, e
                                                );
                                            }

                                            result // Continue sending the success message afterwards
                                        }
                                        _ => result, // No additional data to send
                                    }
                                }
                            };

                            // Write the final response message to client over control stream
                            if let Some(message) = final_result.message {
                                let _ = cmd_stream.write_all(message.as_bytes());
                            }
                        }
                        None => {
                            // Client record missing from registry - critical error
                            error!("Client {} not found in clients map", client_addr);
                            let _ = cmd_stream.write_all(b"421 Client session not found\r\n");
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                // Reading from TCP stream failed - likely a disconnect or network error
                error!("Failed to read from stream: {}", e);
                break;
            }
        }
    }

    // Remove client from registry on disconnect
    {
        let mut clients_guard = clients.lock().unwrap();
        clients_guard.remove(&client_addr);
    }
    info!("Client {} disconnected", client_addr);
}
