use log::{error, info};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use crate::client::Client;
use crate::commands::{Command, CommandResult, handle_command, parse_command};
use crate::data_channel;
use crate::file_transfer;

pub fn handle_client(
    mut cmd_stream: TcpStream,
    clients: Arc<Mutex<HashMap<String, Client>>>,
    client_addr: String,
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
                    info!("Received from {}: {:?}", client_addr, command);

                    let command_result =
                        process_command(&clients, &client_addr, command, &mut cmd_stream);
                    command_buffer.clear();

                    if command_result == CommandResult::QUIT {
                        info!("Client {} requested to quit", client_addr);
                        let _ = cmd_stream.write_all(b"221 Goodbye\r\n");
                        let _ = cmd_stream.shutdown(std::net::Shutdown::Both);
                        break;
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

fn process_command(
    clients: &Arc<Mutex<HashMap<String, Client>>>,
    client_addr: &str,
    command: Command,
    cmd_stream: &mut TcpStream,
) -> CommandResult {
    let mut clients_guard = clients.lock().unwrap();

    if let Some(client) = clients_guard.get_mut(client_addr) {
        match handle_command(client, command, cmd_stream) {
            CommandResult::CONNECT(socket_address) => data_channel::handle_connect_command(
                clients,
                client_addr,
                socket_address,
                cmd_stream,
            ),
            CommandResult::STOR(filename) => {
                file_transfer::handle_stor_command(clients, client_addr, &filename, cmd_stream)
            }
            CommandResult::RETR(filename) => {
                file_transfer::handle_retr_command(clients, client_addr, &filename, cmd_stream)
            }
            CommandResult::LIST => {
                file_transfer::handle_list_command(clients, client_addr, cmd_stream)
            }
            result => result,
        }
    } else {
        CommandResult::QUIT
    }
}
