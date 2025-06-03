use log::{error, info};
use std::io::{Read, Write};
use std::net::TcpStream;

use crate::auth::AuthState;
use crate::commands::*;

// Server state for each client
#[derive(Default)]
pub struct ServerState {
    auth: AuthState,
}

impl ServerState {
    pub fn get_auth(&mut self) -> &mut AuthState {
        &mut self.auth
    }
}

pub fn handle_client(mut stream: TcpStream) {
    if let Err(e) = stream.write_all(b"220 Welcome to the FTP server\r\n") {
        error!("Failed to send welcome: {}", e);
        return;
    }

    let mut state = ServerState::default();
    let mut buffer = [0; 1024];
    let mut command_buffer = String::new();

    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                info!("Connection closed by client");
                break;
            }
            Ok(n) => {
                command_buffer.push_str(&String::from_utf8_lossy(&buffer[..n]));

                if command_buffer.ends_with("\r\n") {
                    let command = parse_command(&command_buffer);
                    info!(
                        "Received from {}: {:?}",
                        stream.peer_addr().unwrap(),
                        command
                    );

                    let command_result = handle_command(&mut state, command, &mut stream);

                    command_buffer.clear();

                    match command_result {
                        CommandResult::Quit => {
                            info!(
                                "Client {} requested to {:?}",
                                stream.peer_addr().unwrap(),
                                command_result
                            );
                            let _ = stream.shutdown(std::net::Shutdown::Both);
                            break;
                        }
                        CommandResult::Continue => {
                            continue;
                        }
                        CommandResult::Stor => {
                            info!(
                                "Client {} requested to store data",
                                stream.peer_addr().unwrap()
                            );

                            let _ = stream.write_all(b"150 Opening data connection\r\n");
                            let _ = stream.flush();
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
}
