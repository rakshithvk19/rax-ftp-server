//server.rs
use log::{error, info};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::client::Client;
use crate::commands::*;

// Server structure that manages multiple clients
pub struct Server {
    // Map of client connections to their Client data
    clients: Arc<Mutex<HashMap<String, Client>>>,
}

const COMMAND_SOCKET: &str = "127.0.0.1:2121";
const DATA_PORT_RANGE: std::ops::Range<u16> = 2122..2222;

impl Server {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn start(&self) {
        info!("Starting Rax FTP server on {}", COMMAND_SOCKET);
        let listener = TcpListener::bind(COMMAND_SOCKET).unwrap();

        for cmd_stream in listener.incoming() {
            match cmd_stream {
                Ok(cmd_stream) => {
                    let client_addr = cmd_stream.peer_addr().unwrap().to_string();
                    info!("New connection: {}", client_addr);

                    // Register the client
                    {
                        let mut clients = self.clients.lock().unwrap();
                        clients.insert(client_addr.clone(), Client::default());
                    }

                    let clients_ref = Arc::clone(&self.clients);

                    thread::spawn(move || {
                        handle_client(cmd_stream, clients_ref, client_addr);
                    });
                }
                Err(e) => error!("Error accepting connection: {}", e),
            }
        }
    }
}

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
                info!("Connection closed by client");
                break;
            }
            Ok(n) => {
                command_buffer.push_str(&String::from_utf8_lossy(&buffer[..n]));

                if command_buffer.ends_with("\r\n") {
                    let command = parse_command(&command_buffer);
                    info!("Received from {}: {:?}", client_addr, command);

                    let command_result = {
                        let mut clients_guard = clients.lock().unwrap();

                        if let Some(client) = clients_guard.get_mut(&client_addr) {
                            handle_command(client, command, &mut cmd_stream)
                        } else {
                            CommandResult::Quit
                        }
                    };

                    command_buffer.clear();

                    match command_result {
                        CommandResult::Quit => {
                            info!("Client {} requested to quit", client_addr);
                            let _ = cmd_stream.write_all(b"221 Goodbye\r\n");
                            let _ = cmd_stream.shutdown(std::net::Shutdown::Both);
                            break;
                        }
                        CommandResult::Continue => {
                            continue;
                        }
                        CommandResult::Stor => {
                            info!("Client {} requested to store data", client_addr);

                            // Handle data transfer in the same thread
                            if let Some(data_stream) =
                                accept_data_connection(&clients, &client_addr)
                            {
                                let _ = cmd_stream.write_all(b"150 Opening data connection\r\n");
                                let _ = cmd_stream.flush();

                                // Perform the actual data transfer here
                                handle_data_transfer(data_stream, &client_addr);

                                let _ = cmd_stream.write_all(b"226 Transfer complete\r\n");
                            } else {
                                let _ = cmd_stream.write_all(b"425 Can't open data connection\r\n");
                            }
                        }
                        CommandResult::CONNECT => {
                            info!("Initializing data channel for Client {}", client_addr);
                            setup_data_channel(&clients, &client_addr, &mut cmd_stream);
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

    // Clean up client on disconnect
    {
        let mut clients_guard = clients.lock().unwrap();
        clients_guard.remove(&client_addr);
    }
    info!("Client {} disconnected", client_addr);
}

fn setup_data_channel(
    clients: &Arc<Mutex<HashMap<String, Client>>>,
    client_addr: &str,
    cmd_stream: &mut TcpStream,
) {
    // Find an available port for this client
    let data_port = find_available_port();

    match data_port {
        Some(port) => {
            let data_addr = format!("127.0.0.1:{}", port);

            // Create the listener immediately
            match TcpListener::bind(&data_addr) {
                Ok(listener) => {
                    // Set non-blocking mode for the listener
                    if listener.set_nonblocking(true).is_ok() {
                        // Store the listener in the client
                        {
                            let mut clients_guard = clients.lock().unwrap();

                            //Updating listener and subsequent data in client
                            if let Some(client) = clients_guard.get_mut(client_addr) {
                                client.set_data_listener(Some(listener));
                                client.set_data_port(Some(port));
                                client.set_data_channel_init(true);
                            }
                        }

                        let response = format!(
                            "227 Entering Passive Mode (127.0.0.1.{}.{})\r\n",
                            port >> 8,
                            port & 0xFF
                        );
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

//Not needed
fn accept_data_connection(
    clients: &Arc<Mutex<HashMap<String, Client>>>,
    client_addr: &str,
) -> Option<TcpStream> {
    let mut clients_guard = clients.lock().unwrap();

    if let Some(client) = clients_guard.get_mut(client_addr) {
        if let Some(listener) = client.take_data_listener() {
            // Release the lock temporarily while waiting for connection
            drop(clients_guard);

            // Try to accept connection with timeout
            for _ in 0..50 {
                // 5 second timeout (50 * 100ms)
                match listener.accept() {
                    Ok((data_stream, addr)) => {
                        info!(
                            "Data connection accepted from {} for client {}",
                            addr, client_addr
                        );
                        return Some(data_stream);
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(100));
                    }
                    Err(e) => {
                        error!("Error accepting data connection: {}", e);
                        return None;
                    }
                }
            }
            error!("Timeout waiting for data connection from {}", client_addr);

            // Put the listener back if we timed out
            clients_guard = clients.lock().unwrap();
            if let Some(client) = clients_guard.get_mut(client_addr) {
                client.set_data_listener(Some(listener));
            }
        }
    }
    None
}

fn handle_data_transfer(mut data_stream: TcpStream, client_addr: &str) {
    info!("Handling data transfer for client {}", client_addr);

    // Example: Echo received data back (replace with actual file transfer logic)
    let mut buffer = [0; 8192];
    match data_stream.read(&mut buffer) {
        Ok(n) if n > 0 => {
            info!("Received {} bytes from {}", n, client_addr);
            // Process the data here (save to file, etc.)
            let _ = data_stream.write_all(&buffer[..n]);
        }
        Ok(_) => {
            info!("No data received from {}", client_addr);
        }
        Err(e) => {
            error!("Error reading from data stream: {}", e);
        }
    }

    let _ = data_stream.shutdown(std::net::Shutdown::Both);
}

fn find_available_port() -> Option<u16> {
    for port in DATA_PORT_RANGE {
        if TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok() {
            return Some(port);
        }
    }
    None
}
