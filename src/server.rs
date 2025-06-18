//server.rs
use log::{error, info};
use std::collections::HashMap;
use std::fs::File;
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
                info!("Connection closed by client {}", client_addr);
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
                            CommandResult::QUIT
                        }
                    };

                    command_buffer.clear();

                    match command_result {
                        CommandResult::QUIT => {
                            info!("Client {} requested to quit", client_addr);
                            let _ = cmd_stream.write_all(b"221 Goodbye\r\n");
                            let _ = cmd_stream.shutdown(std::net::Shutdown::Both);
                            break;
                        }
                        CommandResult::CONTINUE => {
                            continue;
                        }
                        CommandResult::CONNECT(socket_address) => {
                            match socket_address {
                                Some(addr) => {
                                    //Bind socket to TCP listener + set listener to non-blocking mode
                                    match TcpListener::bind(addr) {
                                        Ok(listener) => {
                                            //Set TcpListener to Non-Blocking mode
                                            if listener.set_nonblocking(true).is_ok() {
                                                let mut clients_guard = clients.lock().unwrap();

                                                //Updating listener and subsequent data in client
                                                if let Some(client) =
                                                    clients_guard.get_mut(&client_addr)
                                                {
                                                    client.set_data_listener(Some(listener));
                                                    client.set_data_channel_init(true);
                                                }

                                                let _ = cmd_stream
                                                    .write_all(b"200 PORT command successful\r\n");

                                                let response = format!(
                                                    "227 Entering Active Mode ({})\r\n",
                                                    &addr
                                                );
                                                let _ = cmd_stream.write_all(response.as_bytes());

                                                info!(
                                                    "Data channel ready on {} for client {}",
                                                    &addr, client_addr
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            let _= cmd_stream.write_all(b"500 Unexpected error while establishing connection with server. Try using a different port.");
                                            error!(
                                                "Error binding socket {} to listener. Error: {}",
                                                &addr, e
                                            );
                                            continue;
                                        }
                                    }
                                }
                                None => {
                                    setup_data_channel(&clients, &client_addr, &mut cmd_stream);
                                }
                            }
                            info!("Initializing data channel for Client {}", client_addr);
                        }
                        CommandResult::STOR(filename) => {
                            info!(
                                "Client {} requested to store data for {}",
                                client_addr, filename
                            );
                            if let Some(mut data_stream) = setup_data_stream(&clients, &client_addr)
                            {
                                match File::create(&filename) {
                                    Ok(mut file) => {
                                        let mut buffer = [0; 1024];
                                        loop {
                                            match data_stream.read(&mut buffer) {
                                                Ok(0) => break, // End of data
                                                Ok(n) => {
                                                    if let Err(e) = file.write_all(&buffer[..n]) {
                                                        error!(
                                                            "Failed to write to file {}: {}",
                                                            filename, e
                                                        );
                                                        let _ = cmd_stream.write_all(
                                                            b"550 Requested action not taken\r\n",
                                                        );
                                                        break;
                                                    }
                                                }
                                                Err(e) => {
                                                    error!(
                                                        "Failed to read from data stream: {}",
                                                        e
                                                    );
                                                    let _ = cmd_stream.write_all(b"426 Connection closed; transfer aborted\r\n");
                                                    break;
                                                }
                                            }
                                        }
                                        if file.flush().is_ok() {
                                            let _ =
                                                cmd_stream.write_all(b"226 Transfer complete\r\n");
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to create file {}: {}", filename, e);
                                        let _ = cmd_stream
                                            .write_all(b"550 Requested action not taken\r\n");
                                    }
                                }
                            } else {
                                let _ = cmd_stream.write_all(b"425 Can't open data connection\r\n");
                            }
                        }
                        CommandResult::RETR(filename) => {
                            info!(
                                "Client {} requested to retrieve data for {}",
                                client_addr, filename
                            );
                            if let Some(mut data_stream) = setup_data_stream(&clients, &client_addr)
                            {
                                match File::open(&filename) {
                                    Ok(mut file) => {
                                        let mut buffer = [0; 1024];
                                        loop {
                                            match file.read(&mut buffer) {
                                                Ok(0) => break, // End of file
                                                Ok(n) => {
                                                    if let Err(e) =
                                                        data_stream.write_all(&buffer[..n])
                                                    {
                                                        error!(
                                                            "Failed to write to data stream: {}",
                                                            e
                                                        );
                                                        let _ = cmd_stream.write_all(b"426 Connection closed; transfer aborted\r\n");
                                                        break;
                                                    }
                                                }
                                                Err(e) => {
                                                    error!(
                                                        "Failed to read from file {}: {}",
                                                        filename, e
                                                    );
                                                    let _ = cmd_stream.write_all(
                                                        b"451 Requested action aborted\r\n",
                                                    );
                                                    break;
                                                }
                                            }
                                        }
                                        if data_stream.flush().is_ok() {
                                            let _ =
                                                cmd_stream.write_all(b"226 Transfer complete\r\n");
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to open file {}: {}", filename, e);
                                        let _ =
                                            cmd_stream.write_all(b"550 Failed to open file\r\n");
                                    }
                                }
                            } else {
                                let _ = cmd_stream.write_all(b"425 Can't open data connection\r\n");
                            }
                        }
                        CommandResult::LIST => {
                            info!("Client {} requested directory listing", client_addr);
                            if let Some(mut data_stream) = setup_data_stream(&clients, &client_addr)
                            {
                                match std::fs::read_dir(".") {
                                    Ok(entries) => {
                                        let mut file_list = String::new();
                                        for entry in entries {
                                            if let Ok(entry) = entry {
                                                file_list.push_str(&format!(
                                                    "{}\r\n",
                                                    entry.file_name().to_string_lossy()
                                                ));
                                            }
                                        }
                                        if let Err(e) = data_stream.write_all(file_list.as_bytes())
                                        {
                                            error!("Failed to write to data stream: {}", e);
                                            let _ = cmd_stream.write_all(
                                                b"426 Connection closed; transfer aborted\r\n",
                                            );
                                        } else if data_stream.flush().is_ok() {
                                            let _ =
                                                cmd_stream.write_all(b"226 Transfer complete\r\n");
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to read directory: {}", e);
                                        let _ = cmd_stream
                                            .write_all(b"550 Failed to list directory\r\n");
                                    }
                                }
                            } else {
                                let _ = cmd_stream.write_all(b"425 Can't open data connection\r\n");
                            }
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

// Function to accept data connection with timeout
fn setup_data_stream(
    clients: &Arc<Mutex<HashMap<String, Client>>>,
    client_addr: &str,
) -> Option<TcpStream> {
    // Passive mode
    const ACCEPT_ATTEMPTS: u32 = 50;
    const ACCEPT_SLEEP_MS: u32 = 100;
    const ACCEPT_TIMEOUT_SECS: u64 = ((ACCEPT_ATTEMPTS * (ACCEPT_SLEEP_MS)) / 1000) as u64; // For logging

    let listener = {
        let mut clients_guard = clients.lock().unwrap();
        if let Some(client) = clients_guard.get_mut(client_addr) {
            client.take_data_listener()
        } else {
            None
        }
    };

    if let Some(listener) = listener {
        // Try to accept connection with timeout
        for _ in 0..ACCEPT_ATTEMPTS {
            // Total timeout: ACCEPT_ATTEMPTS * ACCEPT_SLEEP_MS milliseconds
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

        // Put the listener back if we timed out
        let mut clients_guard = clients.lock().unwrap();
        if let Some(client) = clients_guard.get_mut(client_addr) {
            client.set_data_listener(Some(listener));
        }
    }
    None
}

fn find_available_port() -> Option<u16> {
    for port in DATA_PORT_RANGE {
        if TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok() {
            return Some(port);
        }
    }
    None
}
