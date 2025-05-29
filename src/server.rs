use log::{error, info};
use std::io::{Read, Write};
use std::net::TcpStream;

use crate::auth::AuthState;
use crate::commands::{CommandResult, handle_command, parse_command};

// Server state for each client
#[derive(Default)]
pub struct ServerState {
    auth: AuthState,
}

impl ServerState {
    pub fn get_auth(&mut self) -> &mut AuthState {
        &mut self.auth
    }

    // pub fn set_auth(&mut self, auth: AuthState) {
    //     self.auth = auth;
    // }
}

pub fn handle_client(mut stream: TcpStream) {
    // Send welcome message
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

                    if command_result == CommandResult::Quit {
                        info!(
                            "Client {} requested to {:?}",
                            stream.peer_addr().unwrap(),
                            command_result
                        );

                        // Shutdown the connection gracefully, use TcpStream::shutdown

                        break;
                    } else if command_result == CommandResult::Wait {
                        // Wait for more commands
                        continue;
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
