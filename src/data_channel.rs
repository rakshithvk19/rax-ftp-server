use log::{error, info};
use std::collections::HashMap;
use std::io::Write;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::client::Client;
use crate::commands::CommandResult;
use crate::utils::find_available_port;

pub fn handle_connect_command(
    clients: &Arc<Mutex<HashMap<SocketAddr, Client>>>,
    client_addr: &SocketAddr,
    socket_address: Option<SocketAddr>,
    cmd_stream: &mut TcpStream,
) -> CommandResult {
    match socket_address {
        Some(addr) => match TcpListener::bind(&addr) {
            Ok(listener) => {
                if listener.set_nonblocking(true).is_ok() {
                    let mut clients_guard = clients.lock().unwrap();
                    if let Some(client) = clients_guard.get_mut(client_addr) {
                        client.set_data_listener(Some(listener));
                        client.set_data_channel_init(true);
                    }
                    let _ = cmd_stream.write_all(b"200 PORT command successful\r\n");
                    let response = format!("227 Entering Active Mode ({})\r\n", &addr);
                    let _ = cmd_stream.write_all(response.as_bytes());
                    info!("Data channel ready on {} for client {}", &addr, client_addr);
                }
            }
            Err(e) => {
                let _ = cmd_stream.write_all(b"500 Unexpected error while establishing connection with server. Try using a different port.");
                error!("Error binding socket {} to listener. Error: {}", &addr, e);
            }
        },
        None => {
            setup_data_channel(clients, client_addr, cmd_stream);
        }
    }
    CommandResult::CONTINUE
}

pub fn setup_data_channel(
    clients: &Arc<Mutex<HashMap<SocketAddr, Client>>>,
    client_addr: &SocketAddr,
    cmd_stream: &mut TcpStream,
) {
    let data_port = find_available_port();
    match data_port {
        Some(port) => {
            let data_addr = format!("127.0.0.1:{}", port);
            match TcpListener::bind(&data_addr) {
                Ok(listener) => {
                    if listener.set_nonblocking(true).is_ok() {
                        let mut clients_guard = clients.lock().unwrap();
                        if let Some(client) = clients_guard.get_mut(client_addr) {
                            client.set_data_listener(Some(listener));
                            client.set_data_port(Some(port));
                            client.set_data_channel_init(true);
                        }
                        let response =
                            format!("227 Entering Passive Mode (127.0.0.1:{})\r\n", port);
                        let _ = cmd_stream.write_all(response.as_bytes());
                        info!(
                            "Data channel ready on {} for client {}",
                            data_addr, client_addr
                        );
                    } else {
                        let _ = cmd_stream.write_all(b"425 Can't setup data connection\r\n");
                    }
                }
                Err(e) => {
                    error!("Error binding data listener on {}: {}", data_addr, e);
                    let _ = cmd_stream.write_all(b"425 Can't open data connection\r\n");
                }
            }
        }
        None => {
            let _ = cmd_stream.write_all(b"425 Can't open data connection\r\n");
        }
    }
}

pub fn setup_data_stream(
    clients: &Arc<Mutex<HashMap<SocketAddr, Client>>>,
    client_addr: &SocketAddr,
) -> Option<TcpStream> {
    const ACCEPT_ATTEMPTS: u32 = 50;
    const ACCEPT_SLEEP_MS: u32 = 100;
    const ACCEPT_TIMEOUT_SECS: u64 = ((ACCEPT_ATTEMPTS * ACCEPT_SLEEP_MS) / 1000) as u64;

    let listener = {
        let mut clients_guard = clients.lock().unwrap();
        clients_guard
            .get_mut(client_addr)
            .and_then(|client| client.take_data_listener())
    };

    if let Some(listener) = listener {
        for _ in 0..ACCEPT_ATTEMPTS {
            match listener.accept() {
                Ok((data_stream, addr)) => {
                    info!(
                        "Data connection accepted from {} for client {}",
                        addr, client_addr
                    );
                    return Some(data_stream);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(ACCEPT_SLEEP_MS as u64));
                }
                Err(e) => {
                    error!("Error accepting data connection: {}", e);
                    break;
                }
            }
        }
        error!(
            "Timeout ({} seconds) waiting for data connection from {}",
            ACCEPT_TIMEOUT_SECS, client_addr
        );
        let mut clients_guard = clients.lock().unwrap();
        if let Some(client) = clients_guard.get_mut(client_addr) {
            client.set_data_listener(Some(listener));
        }
    }
    None
}
