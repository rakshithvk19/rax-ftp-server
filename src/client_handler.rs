//client_handler.rs
// Handles client connections and processes FTP commands

use log::{error, info};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};

use crate::channel_registry;
use crate::client::Client;
use crate::command::{CommandData, CommandStatus, parse_command};
use crate::handlers::handle_command;

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
                command_buffer.push_str(&String::from_utf8_lossy(&buffer[..n]));

                if command_buffer.ends_with("\r\n") {
                    let command = parse_command(&command_buffer);
                    info!("Received from {}: {:?}", client_addr, &command);
                    command_buffer.clear();

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
                                    let _ = cmd_stream.shutdown(std::net::Shutdown::Both);
                                    break;
                                }
                                CommandStatus::Failure(_) => {
                                    result // already includes the message
                                }
                                CommandStatus::Success => match &result.data {
                                    Some(CommandData::DirectoryListing(listing)) => {
                                        // Construct listing output
                                        let mut listing_output = String::new();
                                        for entry in listing {
                                            listing_output.push_str(&format!("{}\r\n", entry));
                                        }

                                        // Send the listing to the client on the control stream
                                        if let Err(e) =
                                            cmd_stream.write_all(listing_output.as_bytes())
                                        {
                                            error!(
                                                "Failed to send directory listing to client {}: {}",
                                                client_addr, e
                                            );
                                        }

                                        result // return the result to let the message (like "226 Transfer complete") be sent
                                    }
                                    _ => result, // success without follow-up action
                                },
                            };

                            // Write follow-up result message to control stream
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

    {
        let mut clients_guard = clients.lock().unwrap();
        clients_guard.remove(&client_addr);
    }
    info!("Client {} disconnected", client_addr);
}
