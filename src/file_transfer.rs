use log::{error, info};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};

use crate::client::Client;
use crate::commands::CommandResult;
use crate::data_channel;

pub fn handle_stor_command(
    clients: &Arc<Mutex<HashMap<SocketAddr, Client>>>,
    client_addr: &SocketAddr,
    filename: &str,
    cmd_stream: &mut TcpStream,
) -> CommandResult {
    info!(
        "Client {} requested to store data for {}",
        client_addr, filename
    );
    if let Some(mut data_stream) = data_channel::setup_data_stream(clients, client_addr) {
        match File::create(filename) {
            Ok(mut file) => {
                let mut buffer = [0; 1024];
                loop {
                    match data_stream.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(n) => {
                            if let Err(e) = file.write_all(&buffer[..n]) {
                                error!("Failed to write to file {}: {}", filename, e);
                                let _ = cmd_stream.write_all(b"550 Requested action not taken\r\n");
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Failed to read from data stream: {}", e);
                            let _ = cmd_stream
                                .write_all(b"426 Connection closed; transfer aborted\r\n");
                            break;
                        }
                    }
                }
                if file.flush().is_ok() {
                    let _ = cmd_stream.write_all(b"226 Transfer complete\r\n");
                }
            }
            Err(e) => {
                error!("Failed to create file {}: {}", filename, e);
                let _ = cmd_stream.write_all(b"550 Requested action not taken\r\n");
            }
        }
    } else {
        let _ = cmd_stream.write_all(b"425 Can't open data connection\r\n");
    }
    CommandResult::CONTINUE
}

pub fn handle_retr_command(
    clients: &Arc<Mutex<HashMap<SocketAddr, Client>>>,
    client_addr: &SocketAddr,
    filename: &str,
    cmd_stream: &mut TcpStream,
) -> CommandResult {
    info!(
        "Client {} requested to retrieve data for {}",
        client_addr, filename
    );
    if let Some(mut data_stream) = data_channel::setup_data_stream(clients, client_addr) {
        match File::open(filename) {
            Ok(mut file) => {
                let mut buffer = [0; 1024];
                loop {
                    match file.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(n) => {
                            if let Err(e) = data_stream.write_all(&buffer[..n]) {
                                error!("Failed to write to data stream: {}", e);
                                let _ = cmd_stream
                                    .write_all(b"426 Connection closed; transfer aborted\r\n");
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Failed to read from file {}: {}", filename, e);
                            let _ = cmd_stream.write_all(b"451 Requested action aborted\r\n");
                            break;
                        }
                    }
                }
                if data_stream.flush().is_ok() {
                    let _ = cmd_stream.write_all(b"226 Transfer complete\r\n");
                }
            }
            Err(e) => {
                error!("Failed to open file {}: {}", filename, e);
                let _ = cmd_stream.write_all(b"550 Failed to open file\r\n");
            }
        }
    } else {
        let _ = cmd_stream.write_all(b"425 Can't open data connection\r\n");
    }
    CommandResult::CONTINUE
}

pub fn handle_list_command(
    clients: &Arc<Mutex<HashMap<SocketAddr, Client>>>,
    client_addr: &SocketAddr,
    cmd_stream: &mut TcpStream,
) -> CommandResult {
    info!("Client {} requested directory listing", client_addr);
    if let Some(mut data_stream) = data_channel::setup_data_stream(clients, client_addr) {
        match std::fs::read_dir(".") {
            Ok(entries) => {
                let mut file_list = String::new();
                for entry in entries {
                    if let Ok(entry) = entry {
                        file_list.push_str(&format!("{}\r\n", entry.file_name().to_string_lossy()));
                    }
                }
                if let Err(e) = data_stream.write_all(file_list.as_bytes()) {
                    error!("Failed to write to data stream: {}", e);
                    let _ = cmd_stream.write_all(b"426 Connection closed; transfer aborted\r\n");
                } else if data_stream.flush().is_ok() {
                    let _ = cmd_stream.write_all(b"226 Transfer complete\r\n");
                }
            }
            Err(e) => {
                error!("Failed to read directory: {}", e);
                let _ = cmd_stream.write_all(b"550 Failed to list directory\r\n");
            }
        }
    } else {
        let _ = cmd_stream.write_all(b"425 Can't open data connection\r\n");
    }
    CommandResult::CONTINUE
}
