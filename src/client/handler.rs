use log::{error, info};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

use crate::client::Client;
use crate::protocol::handle_command;
use crate::protocol::{CommandStatus, parse_command};
use crate::server::config::ServerConfig;
use crate::transfer::ChannelRegistry;

const MAX_COMMAND_LENGTH: usize = 512;

/// Handles FTP client session using Tokio async runtime.
///
/// - Uses BufReader to read command lines from the client.
/// - Dispatches commands using `handle_command`.
/// - Manages client state from shared `client_registry` and `channel_registry`.
pub async fn handle_client(
    cmd_stream: TcpStream,
    clients: Arc<Mutex<HashMap<SocketAddr, Client>>>,
    client_addr: SocketAddr,
    channel_registry: Arc<Mutex<ChannelRegistry>>,
    config: Arc<ServerConfig>,
) {
    let (read_half, mut write_half) = cmd_stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                // Client closed the connection
                info!("Connection closed by client {}", client_addr);
                break;
            }
            Ok(_) => {
                // Enforce command length limit
                if line.len() > MAX_COMMAND_LENGTH {
                    error!("Command too long ({} chars) from client {}", line.len(), client_addr);
                    if let Err(e) = write_half.write_all(b"500 Command too long\r\n").await {
                        error!("Failed to send error response to {}: {}", client_addr, e);
                        break;
                    }
                    continue;
                }

                let trimmed = line.trim_end_matches("\r\n");
                let command = parse_command(trimmed);
                info!("Received from {}: {:?}", client_addr, &command);

                let mut clients_guard = clients.lock().await;
                let mut channel_registry_guard = channel_registry.lock().await;

                match clients_guard.get_mut(&client_addr) {
                    Some(client) => {
                        let result =
                            handle_command(client, &command, &mut channel_registry_guard, &config);

                        match result.status {
                            CommandStatus::CloseConnection => {
                                if let Some(msg) = result.message {
                                    if let Err(e) = write_half.write_all(msg.as_bytes()).await {
                                        error!("Failed to send quit response to {}: {}", client_addr, e);
                                    }
                                }
                                info!("Client {} requested to quit", client_addr);
                                break;
                            }
                            CommandStatus::Success => {
                                if let Some(msg) = result.message {
                                    info!(
                                        "Sending success response to client {}: {}",
                                        client_addr,
                                        msg.trim()
                                    );
                                    if let Err(e) = write_half.write_all(msg.as_bytes()).await {
                                        error!("Failed to send success response to {}: {}", client_addr, e);
                                        break;
                                    }
                                }
                            }
                            CommandStatus::Failure(ref reason) => {
                                info!("Command failed for client {}: {}", client_addr, reason);
                                if let Some(msg) = result.message {
                                    if let Err(e) = write_half.write_all(msg.as_bytes()).await {
                                        error!("Failed to send error response to {}: {}", client_addr, e);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    None => {
                        error!("Client {} not found in clients map - terminating connection", client_addr);
                        if let Err(e) = write_half
                            .write_all(b"421 Client session not found\r\n")
                            .await {
                            error!("Failed to send session error to {}: {}", client_addr, e);
                        }
                        break;
                    }
                }
            }
            Err(e) => {
                error!("Failed to read from {}: {}", client_addr, e);
                break;
            }
        }
    }

    // Clean up any remaining data channels
    {
        let mut channel_registry_guard = channel_registry.lock().await;
        if let Some(entry) = channel_registry_guard.remove(&client_addr) {
            drop(entry);
            info!(
                "Cleaned up data channel for disconnecting client {}",
                client_addr
            );
        } else {
            info!("No data channel to clean up for client {}", client_addr);
        }
    }

    // Clean up client from registry
    {
        let mut clients_guard = clients.lock().await;
        if clients_guard.remove(&client_addr).is_some() {
            info!("Client {} removed from registry and disconnected", client_addr);
        } else {
            info!("Client {} was already removed from registry", client_addr);
        }
    }
}
