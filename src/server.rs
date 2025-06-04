use log::{error, info};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

use crate::auth::AuthState;
use crate::commands::*;

// Server state for each client
#[derive(Default)]
pub struct ServerState {
    auth: AuthState,
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            auth: AuthState::default(),
        }
    }

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

pub fn start_server(addr: &str) -> std::io::Result<()> {
    info!("Starting FTP server on {}", addr);
    let listener = TcpListener::bind(addr)?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                info!("New connection: {}", stream.peer_addr()?);
                thread::spawn(|| {
                    handle_client(stream);
                });
            }
            Err(e) => error!("Error accepting connection: {}", e),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    // #[test]
    // fn test_server_state_new() {
    //     // let state = ServerState::new();
    //     // assert!(state.auth.username.is_none());
    //     // assert!(!state.auth.is_authenticated);
    // }

    // #[test]
    // fn test_server_state_get_auth() {
    //     // let mut state = ServerState::new();
    //     // let auth = state.get_auth();
    //     // assert!(!auth.is_authenticated);
    // }

    //Works
    #[test]
    fn test_handle_client_quit() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = thread::spawn(move || {
            let (client, _) = listener.accept().unwrap();
            handle_client(client);
        });

        let mut client = TcpStream::connect(addr).unwrap();
        client.write_all(b"QUIT\r\n").unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        assert!(response.contains("221 Goodbye"));

        handle.join().unwrap();
    }

    // #[test]
    // fn test_handle_client_welcome_message() {
    //     let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    //     let addr = listener.local_addr().unwrap();

    //     let handle = thread::spawn(move || {
    //         let (client, _) = listener.accept().unwrap();
    //         handle_client(client);
    //     });

    //     let mut client = TcpStream::connect(addr).unwrap();
    //     let mut response = String::new();
    //     client.read_to_string(&mut response).unwrap();

    //     assert!(response.starts_with("220 Welcome"));

    //     handle.join().unwrap();
    // }

    // #[test]
    // fn test_handle_client_invalid_command() {
    //     let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    //     let addr = listener.local_addr().unwrap();

    //     let handle = thread::spawn(move || {
    //         let (client, _) = listener.accept().unwrap();
    //         handle_client(client);
    //     });

    //     let mut client = TcpStream::connect(addr).unwrap();
    //     client.write_all(b"INVALID\r\n").unwrap();
    //     let mut response = String::new();
    //     client.read_to_string(&mut response).unwrap();
    //     assert!(response.contains("500 Unknown command"));

    //     handle.join().unwrap();
    // }

    // #[test]
    // fn test_handle_client_stor_command() {
    //     let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    //     let addr = listener.local_addr().unwrap();

    //     let handle = thread::spawn(move || {
    //         let (client, _) = listener.accept().unwrap();
    //         handle_client(client);
    //     });

    //     let mut client = TcpStream::connect(addr).unwrap();
    //     client.write_all(b"STOR file.txt\r\n").unwrap();
    //     let mut response = String::new();
    //     client.read_to_string(&mut response).unwrap();
    //     assert!(response.contains("150 Opening data connection"));

    //     handle.join().unwrap();
    // }
}
