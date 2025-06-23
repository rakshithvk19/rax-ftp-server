use log::{error, info};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};

use crate::client::Client;
use crate::commands::{Command, CommandData, CommandStatus, handle_command, parse_command};
use crate::data_channel;
use crate::file_transfer;

pub fn handle_client(
    mut cmd_stream: TcpStream,
    clients: Arc<Mutex<HashMap<SocketAddr, Client>>>,
    client_addr: SocketAddr,
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

                    match clients_guard.get_mut(&client_addr) {
                        Some(client) => {
                            let result = handle_command(client, &command);

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
                                    Some(CommandData::Connect(socket_address)) => {
                                        data_channel::handle_connect_command(
                                            &clients,
                                            &client_addr,
                                            Some(*socket_address),
                                            &mut cmd_stream,
                                        )
                                    }
                                    Some(CommandData::File(filename)) => {
                                        if command == Command::STOR(filename.clone()) {
                                            file_transfer::handle_stor_command(
                                                &clients,
                                                &client_addr,
                                                filename,
                                            )
                                        } else {
                                            file_transfer::handle_retr_command(
                                                &clients,
                                                &client_addr,
                                                filename,
                                            )
                                        }
                                    }
                                    Some(CommandData::DirectoryListing(_)) => {
                                        file_transfer::handle_list_command(&clients, &client_addr)
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
