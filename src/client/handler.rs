use log::{error, info};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

use crate::transfer::ChannelRegistry;
use crate::client::Client;
use crate::protocol::{CommandData, CommandStatus, parse_command};
use crate::protocol::handle_command;
use crate::server::config::ServerConfig;

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
                    let _ = write_half.write_all(b"500 Command too long\r\n").await;
                    continue;
                }

                let trimmed = line.trim_end_matches("\r\n");
                let command = parse_command(trimmed);
                info!("Received from {}: {:?}", client_addr, &command);

                let mut clients_guard = clients.lock().await;
                let mut channel_registry_guard = channel_registry.lock().await;

                match clients_guard.get_mut(&client_addr) {
                    Some(client) => {
                        let result = handle_command(client, &command, &mut channel_registry_guard, &config);

                        match result.status {
                            CommandStatus::CloseConnection => {
                                if let Some(msg) = result.message {
                                    let _ = write_half.write_all(msg.as_bytes()).await;
                                }
                                info!("Client {} requested to quit", client_addr);
                                break;
                            }
                            CommandStatus::Success => {
                                if let Some(msg) = result.message {
                                    info!("Sending response to client {}: {}", client_addr, msg.trim());
                                    let _ = write_half.write_all(msg.as_bytes()).await;
                                }
                            }
                            CommandStatus::Failure(_) => {
                                if let Some(msg) = result.message {
                                    let _ = write_half.write_all(msg.as_bytes()).await;
                                }
                            }
                        }
                    }
                    None => {
                        error!("Client {} not found in clients map", client_addr);
                        let _ = write_half
                            .write_all(b"421 Client session not found\r\n")
                            .await;
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

    let mut clients_guard = clients.lock().await;
    clients_guard.remove(&client_addr);
    info!("Client {} disconnected", client_addr);
}
