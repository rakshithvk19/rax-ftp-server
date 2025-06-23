use log::{error, info};
use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::client::Client;
use crate::commands::{CommandResult, CommandStatus};

/// Sets up the data listener socket for both PORT and PASV commands.
pub fn handle_connect_command(
    clients: &Arc<Mutex<HashMap<SocketAddr, Client>>>,
    client_addr: &SocketAddr,
    socket_address: Option<SocketAddr>,
    _cmd_stream: &mut TcpStream, // no longer used for writing directly
) -> CommandResult {
    let addr = match socket_address {
        Some(addr) => addr,
        None => {
            let clients_guard = clients.lock().unwrap();
            if let Some(client) = clients_guard.get(client_addr) {
                if let Some(addr) = client.data_socket() {
                    addr
                } else {
                    error!("No data socket found for PASV mode");
                    return CommandResult {
                        status: CommandStatus::Failure("No data socket found".into()),
                        message: Some("425 Can't open data connection\r\n".into()),
                        data: None,
                    };
                }
            } else {
                error!("Client not found for PASV mode setup");
                return CommandResult {
                    status: CommandStatus::Failure("Client not found".into()),
                    message: Some("425 Can't open data connection\r\n".into()),
                    data: None,
                };
            }
        }
    };

    match TcpListener::bind(addr) {
        Ok(listener) => {
            if let Err(e) = listener.set_nonblocking(true) {
                error!("Failed to set non-blocking mode on listener: {}", e);
                return CommandResult {
                    status: CommandStatus::Failure("Failed to set non-blocking".into()),
                    message: Some("425 Can't setup data connection\r\n".into()),
                    data: None,
                };
            }

            let mut clients_guard = clients.lock().unwrap();
            if let Some(client) = clients_guard.get_mut(client_addr) {
                client.set_data_listener(Some(listener));
                client.set_data_channel_init(true);
            }

            info!("Data channel ready on {} for client {}", addr, client_addr);

            CommandResult {
                status: CommandStatus::Success,
                message: Some(format!("227 Entering Passive Mode ({})\r\n", addr)),
                data: None,
            }
        }
        Err(e) => {
            error!("Error binding socket {}: {}", addr, e);
            CommandResult {
                status: CommandStatus::Failure("Failed to bind socket".into()),
                message: Some("425 Can't open data connection\r\n".into()),
                data: None,
            }
        }
    }
}

/// Accepts a new connection from the data listener for transfers (RETR, STOR, LIST).
pub fn setup_data_stream(
    clients: &Arc<Mutex<HashMap<SocketAddr, Client>>>,
    client_addr: &SocketAddr,
) -> Option<TcpStream> {
    const ACCEPT_ATTEMPTS: u32 = 50;
    const ACCEPT_SLEEP_MS: u64 = 100;
    const TIMEOUT_MSG: &str = "Timeout waiting for data connection";

    let listener = {
        let mut clients_guard = clients.lock().unwrap();
        clients_guard
            .get_mut(client_addr)
            .and_then(|client| client.take_data_listener())
    };

    if let Some(listener) = listener {
        for _ in 0..ACCEPT_ATTEMPTS {
            match listener.accept() {
                Ok((stream, addr)) => {
                    info!(
                        "Data connection accepted from {} for client {}",
                        addr, client_addr
                    );
                    return Some(stream);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(ACCEPT_SLEEP_MS));
                }
                Err(e) => {
                    error!("Failed to accept data connection: {}", e);
                    break;
                }
            }
        }

        error!("{}: {}", TIMEOUT_MSG, client_addr);
        let mut clients_guard = clients.lock().unwrap();
        if let Some(client) = clients_guard.get_mut(client_addr) {
            client.set_data_listener(Some(listener)); // restore listener if timed out
        }
    }

    None
}
